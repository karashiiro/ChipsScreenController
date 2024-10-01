#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use crate::color::Color;
use crate::device::{get_chips_id, get_chips_serial_port_info, ChipsDevice};
use crate::errors::Result;
use device::Point;
use eframe::egui;
use fontdue::Font;
use image::ImageReader;
use rand::Rng;
use serialport::SerialPortInfo;
use widget_renderer::WidgetRenderer;

mod color;
mod device;
mod errors;
mod widget_renderer;

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

        let mut widget_renderer = WidgetRenderer::new(&mut device);

        // Draw image
        let image = ImageReader::open("./src/test_image.png")?.decode()?;
        widget_renderer.render_image(&image, 0, 0)?;

        // Draw rectangle
        let color = Color::new(255, 0, 0);
        widget_renderer.render_rectangle(0, 0, 32, 32, color)?;

        // Draw bar graph
        widget_renderer.render_graph_background(0, 300, 200, 100, color)?;

        let mut bar_graph_data = vec![0; 300];
        let mut rng = rand::thread_rng();
        let distr = rand::distributions::Uniform::new_inclusive(0u8, 100u8);
        for x in &mut bar_graph_data {
            *x = rng.sample(distr);
        }

        widget_renderer.render_bar_graph(
            0,
            300,
            100,
            Color::new(0, 0, 255),
            Color::new(0, 255, 0),
            &bar_graph_data,
        )?;

        // Draw line graph
        widget_renderer.render_graph_background(320, 300, 200, 100, color)?;

        let mut line_graph_data = vec![0; 300];
        let mut rng = rand::thread_rng();
        let distr = rand::distributions::Uniform::new_inclusive(0u8, 100u8);
        for x in &mut line_graph_data {
            *x = rng.sample(distr);
        }

        widget_renderer.render_line_graph(
            320,
            300,
            100,
            Color::new(0, 0, 255),
            Color::new(0, 255, 0),
            &line_graph_data,
        )?;

        // Draw grid with pixels
        let mut grid_points: Vec<Point> = vec![];
        for x in 200..=400 {
            for y in 100..=300 {
                if x % 100 == 0 || y % 100 == 0 {
                    grid_points.push(Point::new(x + 10, y + 10));
                }
            }
        }

        widget_renderer.render_pixels(Color::new(0, 0, 255), &grid_points)?;

        // Draw text
        let font = include_bytes!("../resources/roboto/Roboto-Regular.ttf") as &[u8];
        let roboto_regular = Font::from_bytes(font, fontdue::FontSettings::default()).unwrap();

        // TODO: Ideally we should be able to rasterize more than one character
        let (metrics, bitmap) = roboto_regular.rasterize('g', 24.0);
        let mut text_coordinate_list: Vec<Point> = vec![];
        for x in 0..metrics.width {
            for y in 0..metrics.height {
                let value = bitmap[x + metrics.width * y];
                if value != 0 {
                    // TODO: Transparency requires keeping a local buffer of the screen state.
                    // We can't receive data from the device quickly, so we need to constantly keep
                    // track of what the current screen state should be locally so we know how to
                    // handle transparency. We can then do an HSV calculation to figure out how to
                    // overlay the values.
                    text_coordinate_list.push(Point::new(500 + x as i32, 200 + y as i32));
                }
            }
        }

        widget_renderer.render_pixels(Color::new(255, 255, 255), &text_coordinate_list)?;
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
