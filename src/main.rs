#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;

use eframe::egui;
use serialport::{DataBits, Parity, SerialPort, SerialPortInfo, SerialPortType, StopBits};
use windows::core::*;
use windows::Devices::Enumeration::DeviceInformation;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };

    let chips_device_id = get_chips_id().unwrap().unwrap();
    let chips_port_info =
        get_chips_serial_port_info(&chips_device_id).expect("Failed to find available port");

    let _ = open_chips_device(chips_port_info.clone());

    eframe::run_simple_native("My egui App", options, move |ctx, _frame| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            ui.label(format!("{}", chips_port_info.port_name));
        });
    })
}

fn open_chips_device(chips_port_info: SerialPortInfo) -> Option<Box<dyn SerialPort>> {
    // Fails if another application is already using the device
    serialport::new(chips_port_info.port_name.clone(), 115200)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_secs(1))
        .open()
        .ok()
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
