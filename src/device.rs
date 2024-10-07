use std::thread;
use std::time::Duration;

use crate::{
    color::Color,
    errors::{ChipsError, Result},
};
use image::{DynamicImage, RgbImage};
use serialport::{
    DataBits, FlowControl, Parity, SerialPort, SerialPortInfo, SerialPortType, StopBits,
};
use windows::Devices::Enumeration::DeviceInformation;

// 3.5-inch model: 480x320
// 5-inch model: 800x480
// 7-inch model: 1024x600
pub const SCREEN_WIDTH: i32 = 800;
pub const SCREEN_HEIGHT: i32 = 480;
pub const PIXEL_DEPTH: u32 = 2;

#[derive(Debug, Clone, Copy)]
pub struct Point(i32, i32);

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self(x, y)
    }
}

#[derive(Debug)]
pub struct ChipsDevice {
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

    pub fn draw_image(&mut self, image: &DynamicImage, x: i32, y: i32) -> Result<()> {
        let width = image.width() as i32;
        let height = image.height() as i32;
        if width + x > SCREEN_WIDTH || height + y > SCREEN_HEIGHT {
            return Err(ChipsError::ImageTooLarge);
        }

        // Convert to RGB so we have a known pixel format to convert from
        let image = image.to_rgb8();

        let mut buf = ChipsDevice::image_to_buffer(&image);
        self.send_command_simple(197, x, y, x + width - 1, y + height - 1)?;
        self.write_to_serial_port(&mut buf)?;
        thread::sleep(Duration::from_millis(10));

        Ok(())
    }

    fn image_to_buffer(image: &RgbImage) -> Vec<u8> {
        let buf_size = (PIXEL_DEPTH * image.width() * image.height()) as usize;
        let mut buf: Vec<u8> = vec![0; buf_size];

        for y in 0..image.height() {
            for x in 0..image.width() {
                // Pixel format is 16bpp, RGB565
                let pixel = image.get_pixel(x, y);
                let pixel_r = (pixel.0[0] >> 3) as u16; // 5 high bits
                let pixel_g = (pixel.0[1] >> 2) as u16; // 6 high bits
                let pixel_b = (pixel.0[2] >> 3) as u16; // 5 high bits
                let pixel_16 = (pixel_r << 11) | (pixel_g << 5) | pixel_b;
                let idx = (x * PIXEL_DEPTH + (PIXEL_DEPTH * image.width()) * y) as usize;

                // Write to the buffer, flipping endianness (device is BE)
                buf[idx] = (pixel_16 & 255) as u8;
                buf[idx + 1] = (pixel_16 >> 8) as u8;
            }
        }

        return buf;
    }

    pub fn draw_pixels(&mut self, color: Color, points: &[Point]) -> Result<()> {
        if points.len() == 0 {
            return Ok(());
        }

        let mut list_1: Vec<u8> = vec![];
        let mut source: Vec<Point> = vec![];
        let mut list_2: Vec<u8> = vec![];

        for point in points {
            if point.0 < 256 && point.1 < 256 {
                list_1.push(point.0 as u8);
                list_1.push(point.1 as u8);
            } else {
                source.push(*point);
            }
        }

        self.draw_pixels_raw(0, 0, color, &list_1)?;

        if source.len() == 0 {
            return Ok(());
        }

        let offset_x = source.iter().map(|point| point.0).min().unwrap_or_default();
        if source.iter().any(|point| point.0 - offset_x > 255) {
            return Err(ChipsError::BoundsTooLarge);
        }

        let offset_y = source.iter().map(|point| point.1).min().unwrap_or_default();
        if source.iter().any(|point| point.1 - offset_y > 255) {
            return Err(ChipsError::BoundsTooLarge);
        }

        for point in source {
            list_2.push((point.0 - offset_x) as u8);
            list_2.push((point.1 - offset_y) as u8);
        }

        self.draw_pixels_raw(offset_x, offset_y, color, &list_2)?;

        Ok(())
    }

    fn draw_pixels_raw(
        &mut self,
        offset_x: i32,
        offset_y: i32,
        color: Color,
        coordinates: &[u8],
    ) -> Result<()> {
        let chunk_size = 64;
        let chunk_reserved = 8;
        let chunk_offset = chunk_size - chunk_reserved;

        let color_16 = color.as_serial();
        let mut source_index = 0;
        let mut buf = vec![0; chunk_size];
        buf[6] = (color_16 >> 8) as u8;
        buf[7] = (color_16 & 255) as u8;

        let mut right = chunk_offset;
        while source_index < coordinates.len() {
            if source_index + chunk_offset > coordinates.len() {
                right = coordinates.len() - source_index;
            }

            let copy_len = right;
            buf[chunk_reserved..(chunk_reserved + copy_len)]
                .copy_from_slice(&coordinates[source_index..(source_index + copy_len)]);

            self.send_command(195, offset_x, offset_y, right as i32, 0, &mut buf)?;

            source_index += right;
        }

        Ok(())
    }

    pub fn draw_line_graph(
        &mut self,
        x: i32,
        y: i32,
        count: usize,
        color_bg: Color,
        color_fg: Color,
        data: &[u8],
    ) -> Result<()> {
        let color_bg_16 = color_bg.as_serial();
        let color_fg_16 = color_fg.as_serial();
        let ecc = ((((color_fg_16 as i32) >> 2) + 2 & 15) | (((color_bg_16 as i32) >> 3) + 3 & 240))
            as u8;

        let chunk_size = 64;
        let chunk_reserved = 12;
        let chunk_offset = chunk_size - chunk_reserved - 1;
        let mut source_index: usize = 0;
        let mut right: usize;
        while source_index < count {
            let mut buf: Vec<u8> = vec![0; chunk_size];
            right = chunk_offset;
            if source_index + chunk_offset > count {
                right = count - source_index;
            }

            let left = x as usize + source_index + 1;
            let copy_len = right + 1;
            buf[chunk_reserved..(chunk_reserved + copy_len)]
                .copy_from_slice(&data[source_index..(source_index + copy_len)]);

            if source_index == 0 {
                self.kd_draw_buf(
                    144,
                    (left as i32) | 32768,
                    y,
                    right as i32,
                    color_bg_16 as i32,
                    color_fg_16 as i32,
                    ecc,
                    &mut buf,
                )?;
            } else {
                self.kd_draw_buf(
                    144,
                    left as i32,
                    y,
                    right as i32,
                    color_bg_16 as i32,
                    color_fg_16 as i32,
                    ecc,
                    &mut buf,
                )?;
            }

            source_index += right;
        }

        thread::sleep(Duration::from_millis(5));

        Ok(())
    }

    pub fn draw_bar_graph(
        &mut self,
        x: i32,
        y: i32,
        count: usize,
        color_bg: Color,
        color_fg: Color,
        data: &[u8],
    ) -> Result<()> {
        let color_bg_16 = color_bg.as_serial();
        let color_fg_16 = color_fg.as_serial();
        let ecc = ((((color_fg_16 as i32) >> 2) + 2 & 15) | (((color_bg_16 as i32) >> 3) + 3 & 240))
            as u8;

        let chunk_size = 64;
        let chunk_reserved = 12;
        let chunk_offset = chunk_size - chunk_reserved;
        let mut source_index: usize = 0;
        let mut right: usize;
        while source_index < count {
            let mut buf: Vec<u8> = vec![0; chunk_size];
            right = chunk_offset;
            if source_index + chunk_offset > count {
                right = count - source_index;
            }

            let left = x as usize + source_index;
            let copy_len = right;
            buf[chunk_reserved..(chunk_reserved + copy_len)]
                .copy_from_slice(&data[source_index..(source_index + copy_len)]);

            self.kd_draw_buf(
                137,
                left as i32,
                y,
                right as i32,
                color_bg_16 as i32,
                color_fg_16 as i32,
                ecc,
                &mut buf,
            )?;

            source_index += right;
        }

        thread::sleep(Duration::from_millis(5));

        Ok(())
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
        self.kd_draw(136, left, top, right, bottom, color_16 as i32, ecc)
    }

    fn kd_draw(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: i32,
        ecc: u8,
    ) -> Result<()> {
        let mut data = vec![0; 12];
        self.kd_draw_buf(
            command_code,
            left,
            top,
            right,
            bottom,
            color,
            ecc,
            &mut data,
        )
    }

    fn kd_draw_buf(
        &mut self,
        command_code: u8,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        color: i32,
        ecc: u8,
        data: &mut [u8],
    ) -> Result<()> {
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

        self.write_to_serial_port(data)?;
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
        // serialport doesn't support separate read and write timeouts, so we need to set a value
        // long enough for both - the official app uses 1s as a read timeout with no write timeout.
        // This value is mostly determined by how long it will take to write an image to the screen.
        let io_timeout = Duration::from_secs(10);

        // Fails if another application is already using the device
        let serial_port = serialport::new(chips_port_info.port_name.clone(), 115200)
            .data_bits(DataBits::Eight)
            .flow_control(FlowControl::Hardware)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(io_timeout)
            .open()?;
        Ok(serial_port)
    }
}

pub fn get_chips_serial_port_info(chips_device_id: &str) -> Option<SerialPortInfo> {
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

pub fn get_chips_id() -> Result<Option<String>> {
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
