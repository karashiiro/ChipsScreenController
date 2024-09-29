#[derive(Debug, Copy, Clone)]
pub struct Color(u8, u8, u8);

impl Color {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }

    pub fn as_serial(&self) -> u16 {
        ((self.0 as i32) << 8 & 63488 | (self.1 as i32) << 3 & 2016 | (self.2 as i32) >> 3) as u16
    }
}
