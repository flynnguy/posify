use crate::printer::SupportedPrinters;
use std::io;

#[derive(Clone, Copy, PartialEq)]
pub enum BarcodeType {
    UPCA = 0,   // or 65?
    UPCE = 1,   // or 66?
    EAN13 = 2,  // or 67?
    EAN8 = 3,   // or 68?
    CODE39 = 4, // or 69?
    ITF = 5,    // or 70?
    Code93 = 72,
    Codabar = 6, // or 71?
    Code128 = 73,
    PDF417 = 10,   // or 75?
    QRCode = 11,   // or 76?
    Maxicode = 12, // or 77?
    GS1 = 13,      // or 78?
}

pub enum TextPosition {
    Off = 0x00,
    Above = 0x01,
    Below = 0x02,
    Both = 0x03,
}

pub enum Font {
    Standard,   // As defined in SNBC printer docs
    Compressed, // As defined in SNBC printer docs
    FontA,      // As defined in P3 printer docs
    FontB,      // As defined in P3 printer docs
}

pub struct Barcode {
    pub printer: SupportedPrinters,
    pub width: u8,  // 2 <= n <= 6
    pub height: u8, // 1 <= n <= 255
    pub font: Font,
    // pub code: &str,
    pub kind: BarcodeType,
    pub position: TextPosition,
}

impl Barcode {
    pub fn set_width(&mut self) -> io::Result<[u8; 3]> {
        // P3 notes:
        // docs describe the range of the width as 0x01 <= n <= 0x06
        // but then has a table describing values of n < 0x80 and n > 0x80
        // up to 0x86 🤨
        //
        // Currently limiting to 1 <= n <= 6 but we might be able to change that
        match self.printer {
            SupportedPrinters::SNBC => {
                if self.width >= 2 && self.width <= 6 {
                    return Ok([0x1d, 0x77, self.width]);
                }
                Ok([0x1d, 0x77, 0x02]) // 2 is the default according to docs
            }
            SupportedPrinters::P3 => {
                if self.width >= 1 && self.width <= 6 {
                    return Ok([0x1d, 0x77, self.width]);
                }
                Ok([0x1d, 0x77, 0x03]) // 3 is the default according to docs
            }
            _ => Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Command not supported by printer".to_string(),
            )),
        }
    }

    /// Sets the height of the 1D barcode
    /// n specified the number of vertical dots
    ///
    /// P3 default is 0xA2 (20.25mm)
    /// on P3 at least, 8 dots == 1mm
    /// so mm * 8 = height in dots
    ///
    /// So 20.25 * 8 = 162 which is 0xA2 in hex
    pub fn set_height(&mut self) -> [u8; 3] {
        [0x1d, 0x68, self.height as u8]
    }

    /// Selects the print position of HRI (Human Readable Interpretation)
    /// characters when printing a 1D barcode
    pub fn set_text_position(&mut self) -> [u8; 3] {
        // Codes are the same for SNBC printer and S3
        match self.position {
            TextPosition::Off => [0x1d, 0x48, 0x00],
            TextPosition::Above => [0x1d, 0x48, 0x01],
            TextPosition::Below => [0x1d, 0x48, 0x02],
            TextPosition::Both => [0x1d, 0x48, 0x03],
        }
    }

    pub fn set_font(&mut self) -> [u8; 3] {
        match self.font {
            // FontB and Compressed are the same codes, just different printers
            // define them differently so I figured it would be easiest to just
            // define it twice.
            Font::Compressed => [0x1d, 0x66, 0x01],
            Font::FontB => [0x1d, 0x66, 0x01],
            _ => [0x1d, 0x66, 0x00], // Default to standard font or FontA
        }
    }

    pub fn set_barcode_type(&mut self) -> [u8; 3] {
        match self.kind {
            BarcodeType::EAN13 => [0x1d, 0x6b, 0x02],
            BarcodeType::Code128 => [0x1d, 0x6b, 0x08],
            _ => [0x1d, 0x6b, 0x02], // Default to EAN13?
        }
    }
}
