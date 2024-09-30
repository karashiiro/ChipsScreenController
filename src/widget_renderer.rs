use image::DynamicImage;

use crate::color::Color;
use crate::device::{ChipsDevice, Point};
use crate::errors::Result;

pub struct WidgetRenderer<'a> {
    device: &'a mut ChipsDevice,
}

impl<'a> WidgetRenderer<'a> {
    pub fn new(device: &'a mut ChipsDevice) -> Self {
        Self { device }
    }

    pub fn render_image(&mut self, image: &DynamicImage, x: i32, y: i32) -> Result<()> {
        self.device.draw_image(image, x, y)
    }

    pub fn render_rectangle(
        &mut self,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        color: Color,
    ) -> Result<()> {
        self.device
            .draw_rectangle(x, y, x + width, y + height, color)
    }

    pub fn render_bar_graph(
        &mut self,
        x: i32,
        y: i32,
        count: i32,
        color_bg: Color,
        color_fg: Color,
        data: &[u8],
    ) -> Result<()> {
        self.device
            .draw_bar_graph(x, y, count as usize, color_bg, color_fg, data)
    }

    pub fn render_line_graph(
        &mut self,
        x: i32,
        y: i32,
        count: i32,
        color_bg: Color,
        color_fg: Color,
        data: &[u8],
    ) -> Result<()> {
        self.device
            .draw_line_graph(x, y, count as usize, color_bg, color_fg, data)
    }

    pub fn render_graph_background(
        &mut self,
        x: i32,
        y: i32,
        height: i32,
        count: i32,
        color: Color,
    ) -> Result<()> {
        self.device
            .draw_rectangle(x, y - height, x + count, y, color)
    }

    pub fn render_pixels(&mut self, color: Color, points: &[Point]) -> Result<()> {
        self.device.draw_pixels(color, points)
    }
}
