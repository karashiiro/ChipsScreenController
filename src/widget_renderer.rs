use fontdue::layout::Layout;
use fontdue::Font;
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

    pub fn render_text(
        &mut self,
        layout: &Layout,
        fonts: &[Font],
        x: i32,
        y: i32,
        color: Color,
    ) -> Result<()> {
        let mut text_coordinate_list: Vec<Point> = vec![];

        for glyph in layout.glyphs() {
            // TODO: Maintain local state of loaded fonts and raster cache
            let (metrics, bitmap) = fonts[glyph.font_index].rasterize(glyph.parent, glyph.key.px);
            for char_x in 0..metrics.width {
                for char_y in 0..metrics.height {
                    let value = bitmap[char_x + metrics.width * char_y];
                    if value != 0 {
                        // TODO: Transparency requires keeping a local buffer of the screen state.
                        // We can't receive data from the device quickly, so we need to constantly keep
                        // track of what the current screen state should be locally so we know how to
                        // handle transparency. We can then do an HSV calculation to figure out how to
                        // overlay the values.
                        text_coordinate_list.push(Point::new(
                            x + glyph.x as i32 + char_x as i32,
                            y + glyph.y as i32 + char_y as i32,
                        ));
                    }
                }
            }
        }

        self.render_pixels(color, &text_coordinate_list)
    }
}
