#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::color::Color;
use crate::device::{get_chips_id, get_chips_serial_port_info, ChipsDevice};
use crate::errors::Result;
use eframe::egui;
use image::ImageReader;
use rand::Rng;
use serialport::SerialPortInfo;

mod color;
mod device;
mod errors;

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

        // Fix screen orientation
        device.adjust_screen(true, true, true)?;

        let image = ImageReader::open("./src/test_image.png")?.decode()?;
        device.draw_image(&image, 0, 0)?;

        let color = Color::new(255, 0, 0);
        device.draw_rectangle(0, 0, 300, 300, color)?;

        let mut bar_graph_data = vec![0; 300];
        let mut rng = rand::thread_rng();
        let distr = rand::distributions::Uniform::new_inclusive(0u8, 255u8);
        for x in &mut bar_graph_data {
            *x = rng.sample(distr);
        }
        device.draw_bar_graph(
            0,
            300,
            Color::new(0, 0, 255),
            Color::new(0, 255, 0),
            &bar_graph_data,
        )?;
    }

    eframe::run_native(
        "Image Viewer",
        options,
        Box::new(|cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(App::new(chips_port_info)))
        }),
    )?;

    Ok(())
}

struct App {
    chips_port_info: Option<SerialPortInfo>,
}

impl App {
    pub fn new(chips_port_info: Option<SerialPortInfo>) -> Self {
        Self { chips_port_info }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("My egui Application");
            match self.chips_port_info.clone() {
                Some(port_info) => ui.label(port_info.port_name),
                None => ui.label("Failed to locate device."),
            };

            ui.image(egui::include_image!("test_image.png"));
        });
    }
}
