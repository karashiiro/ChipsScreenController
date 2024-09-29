#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::thread;
use std::time::Duration;

use eframe::egui;
use serialport::{
    DataBits, FlowControl, Parity, SerialPort, SerialPortInfo, SerialPortType, StopBits,
};
use thiserror::Error;
use windows::Devices::Enumeration::DeviceInformation;

const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 480;

#[derive(Error, Debug)]
pub enum ChipsError {
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("serial port error")]
    SerialPort(#[from] serialport::Error),
    #[error("render error")]
    InvalidRender(#[from] eframe::Error),
    #[error("invalid length {received} (expected >= {expected})")]
    InvalidLength { received: usize, expected: usize },
    #[error("win32 error")]
    Win32(#[from] windows_result::Error),
    #[error("poisoned mutex")]
    PoisonedMutex,
}

pub type Result<T, E = ChipsError> = std::result::Result<T, E>;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let chips_device_id = get_chips_id().unwrap().unwrap();
    let chips_port_info = get_chips_serial_port_info(&chips_device_id);

    let chips_device = chips_port_info
        .clone()
        .and_then(|port_info| Some(ChipsDevice::new(port_info)));

    if let Some(mut device) = chips_device {
        device.connect()?;
        device.startup()?;
        device.set_brightness(100)?;

        let color = Color::new(255, 0, 0);
        device.draw_rectangle(0, 0, 64, 64, color)?;
    }

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            match chips_port_info.clone() {
                Some(port_info) => ui.label(port_info.port_name),
                None => ui.label("Failed to locate device."),
            }
        });
    })?;

    Ok(())
}

#[derive(Debug, Copy, Clone)]
struct Color(u8, u8, u8);

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }

    pub fn as_serial(&self) -> u16 {
        ((self.0 as i32) << 8 & 63488 | (self.1 as i32) << 3 & 2016 | (self.2 as i32) >> 3) as u16
    }
}

#[derive(Debug)]
struct ChipsDevice {
    serial_port_info: SerialPortInfo,
    serial_port: Option<Box<dyn SerialPort>>,
}

impl ChipsDevice {
    pub fn new(serial_port_info: SerialPortInfo) -> Self {
        return Self {
            serial_port_info,
            serial_port: None,
        };
    }

    pub fn connect(&mut self) -> Result<()> {
        let mut serial_port = Self::open_chips_serial_port(self.serial_port_info.clone())?;
        serial_port.write_data_terminal_ready(true)?;
        self.serial_port = Some(serial_port);
        Ok(())
    }

    // Is this ever called?
    pub fn startup(&mut self) -> Result<()> {
        self.send_command_code(109)?;
        self.adjust_screen(false, true, true)
    }

    pub fn shutdown(&mut self) -> Result<()> {
        // We don't implement Drop with this since that makes it easy to cause accidental shutdowns
        self.send_command_code(108)?;
        Ok(())
    }

    pub fn restart(&mut self) -> Result<()> {
        self.send_command_code(101)?;
        thread::sleep(Duration::from_secs(1));
        Ok(())
    }

    pub fn set_brightness(&mut self, value: i32) -> Result<()> {
        self.send_command_simple(110, value, 0, 0, 0)
    }

    pub fn adjust_screen(
        &mut self,
        is_mirror: bool,
        is_landscape: bool,
        is_invert: bool,
    ) -> Result<()> {
        if is_mirror {
            self.send_command_122(1)?
        } else {
            self.send_command_122(0)?
        }

        // is_landscape & is_invert
        let mut landscape_invert = 3;
        if is_landscape && !is_invert {
            landscape_invert = 2;
        } else if !is_landscape & is_invert {
            landscape_invert = 1;
        } else if !is_landscape && !is_invert {
            landscape_invert = 0;
        }

        self.send_command_121(landscape_invert, SCREEN_WIDTH, SCREEN_HEIGHT)
    }

    pub fn draw_rectangle(
        &mut self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: Color,
    ) -> Result<()> {
        let color_16 = color.as_serial();
        let ecc = ((((color_16 as i32) >> 2) + 2 & 15) | ((bottom >> 3) + 3 & 240)) as u8;
        self.kd_draw_rectangle(136, left, top, right, bottom, color_16, ecc)
    }

    fn kd_draw_rectangle(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: u16,
        ecc: u8,
    ) -> Result<()> {
        let mut data = vec![0; 12];
        data[0] = (left >> 8) as u8;
        data[1] = (left & 255) as u8;
        data[2] = (top >> 8) as u8;
        data[3] = (top & 255) as u8;
        data[4] = (right >> 8) as u8;
        data[5] = (right & 255) as u8;
        data[6] = (bottom >> 8) as u8;
        data[7] = (bottom & 255) as u8;
        data[8] = (color >> 8) as u8;
        data[9] = (color & 255) as u8;
        data[10] = ecc;
        data[11] = command_code;

        self.write_to_serial_port(&mut data)?;
        Ok(())
    }

    fn send_command_121(&mut self, landscape_invert: u8, width: i32, height: i32) -> Result<()> {
        let landscape_invert = landscape_invert + 100;
        let mut data: Vec<u8> = vec![0; 16];

        data[6] = landscape_invert;
        data[7] = (width >> 8) as u8;
        data[8] = (width & 255) as u8;
        data[9] = (height >> 8) as u8;
        data[10] = (height & 255) as u8;

        self.send_command(121, 0, 0, 0, 0, &mut data)
    }

    fn send_command_122(&mut self, mode_num: u8) -> Result<()> {
        let mut data: Vec<u8> = vec![0; 16];
        data[6] = mode_num;
        self.send_command(122, 0, 0, 0, 0, &mut data)
    }

    fn send_command_code(&mut self, command_code: u8) -> Result<()> {
        let mut data: Vec<u8> = vec![0; 6];
        self.send_command_delayed(command_code, 0, 0, 0, 0, &mut data, 5)
    }

    fn send_command_simple(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
    ) -> Result<()> {
        let mut data: Vec<u8> = vec![0; 6];
        self.send_command_delayed(command_code, left, top, right, bottom, &mut data, 5)
    }

    fn send_command(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        data: &mut [u8],
    ) -> Result<()> {
        self.send_command_delayed(command_code, left, top, right, bottom, data, 5)
    }

    fn send_command_delayed(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        data: &mut [u8],
        delay: u64,
    ) -> Result<()> {
        if data.len() < 6 {
            return Err(ChipsError::InvalidLength {
                received: data.len(),
                expected: 6,
            });
        }

        data[0] = (left >> 2) as u8;
        data[1] = (((left & 3) << 6) + (top >> 4)) as u8;
        data[2] = (((top & 15) << 4) + (right >> 6)) as u8;
        data[3] = (((right & 63) << 2) + (bottom >> 8)) as u8;
        data[4] = (bottom & 255) as u8;
        data[5] = command_code;
        self.write_to_serial_port(data)?;
        thread::sleep(Duration::from_millis(delay));
        Ok(())
    }

    fn write_to_serial_port(&mut self, data: &[u8]) -> Result<()> {
        if let Some(ref mut serial_port) = &mut self.serial_port {
            serial_port.write_all(data)?;
        }

        Ok(())
    }

    fn open_chips_serial_port(chips_port_info: SerialPortInfo) -> Result<Box<dyn SerialPort>> {
        // Fails if another application is already using the device
        let serial_port = serialport::new(chips_port_info.port_name.clone(), 115200)
            .data_bits(DataBits::Eight)
            .flow_control(FlowControl::Hardware)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(Duration::from_secs(1))
            .open()?;
        Ok(serial_port)
    }
}

fn get_chips_serial_port_info(chips_device_id: &str) -> Option<SerialPortInfo> {
    serialport::available_ports()
        .expect("Failed to enumerate available ports")
        .into_iter()
        .find(|port| match &port.port_type {
            SerialPortType::UsbPort(usb_port) => usb_port
                .serial_number
                .clone()
                .and_then(|serial_number| Some(chips_device_id.contains(&serial_number)))
                .unwrap_or(false),
            _ => false,
        })
}

fn get_chips_id() -> Result<Option<String>> {
    let device_info_collection = DeviceInformation::FindAllAsync()?.get()?;
    for device_info in device_info_collection {
        let device_enabled = device_info.IsEnabled()?;
        let device_id = device_info.Id()?;
        if device_enabled && device_id.to_string().contains("USB35INCHIPSV2") {
            return Ok(Some(device_id.to_string()));
        }
    }

    Ok(None)
}
