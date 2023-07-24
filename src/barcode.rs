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

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CodeCError {
    #[error("Not a Number")]
    NotANumber,
    #[error("Length not divisible by two")]
    InvalidLength,
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
            SupportedPrinters::Epic => {
                Ok([0x1d,0x77,0x1]) // 2 is the default. Setting the width to 2
                                    // with a long code128 barcode causes the
                                    // barcode to exceed the print area and not
                                    // print
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
            Font::Standard => [0x1d, 0x66, 0x00],
            Font::Compressed => [0x1d, 0x66, 0x01],
            Font::FontA => [0x1d, 0x66, 0x00],
            Font::FontB => [0x1d, 0x66, 0x01],
        }
    }

    pub fn set_barcode_type(&mut self) -> [u8; 3] {
        match self.kind {
            BarcodeType::EAN13 => [0x1d, 0x6b, 0x02],
            BarcodeType::Code128 => {
                if self.printer == SupportedPrinters::SNBC {
                    [0x1d, 0x6b, 0x49]
                } else {
                    [0x1d, 0x6b, 0x08]
                }
            }
            // TODO: Add more barcode types
            _ => [0x1d, 0x6b, 0x02], // Default to EAN13?
        }
    }

    // to_codeset_c converts a string of numbers to the u8 value
    // of pairs of numbers according to the Code Set C
    //
    // ex:
    // "00" => 0x00 (Decimal 0)
    // "10" => 0x0A (Decimal 10)
    pub fn to_codeset_c(barcode: String) -> Result<Vec<u8>, CodeCError> {
        // Barcode can only contain digits, no alpha chars
        if !barcode.chars().all(|x| x.is_ascii_digit()) {
            return Err(CodeCError::NotANumber);
        }
        // Length must be divisible by 2
        // alternatively we might be able to just prepend a 0?
        if barcode.len() % 2 != 0 {
            return Err(CodeCError::InvalidLength);
        }

        // Split digit pairs into individual Vec items
        let mut split: Vec<&str> = Vec::new();
        for (i, _) in barcode.as_bytes().iter().enumerate() {
            if i % 2 == 0 {
                split.push(Some(&barcode[i..i + 2]).unwrap());
            }
        }
        // Now convert string pairs to numeric equiv in codeset c
        let mut converted: Vec<u8> = Vec::new();
        for pair in split {
            let code: u8 = pair.parse().unwrap();
            converted.push(code);
        }

        Ok(converted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codeset_c_tests() {
        let resp = Barcode::to_codeset_c("foo".to_string());
        assert_eq!(resp, Err(CodeCError::NotANumber));

        let resp = Barcode::to_codeset_c("123".to_string());
        assert_eq!(resp, Err(CodeCError::InvalidLength));

        let resp = Barcode::to_codeset_c("00".to_string()).unwrap();
        assert_eq!(resp, vec![0x00_u8]);

        let resp = Barcode::to_codeset_c("01".to_string()).unwrap();
        assert_eq!(resp, vec![0x01_u8]);

        let resp = Barcode::to_codeset_c("1234".to_string()).unwrap();
        assert_eq!(resp, vec![0x0c_u8, 0x22]);
    }
}
