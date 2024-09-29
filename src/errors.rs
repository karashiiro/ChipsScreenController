use thiserror::Error;

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
    #[error("invalid image")]
    InvalidImage(#[from] image::ImageError),
    #[error("image too large for screen")]
    ImageTooLarge,
    #[error("image has an invalid format")]
    ImageFormat,
    #[error("win32 error")]
    Win32(#[from] windows_result::Error),
}

pub type Result<T, E = ChipsError> = std::result::Result<T, E>;
