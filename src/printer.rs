use std::io;

use std::time::Duration;

use byteorder::{LittleEndian, WriteBytesExt};
use encoding::all::UTF_8;
use encoding::types::{EncoderTrap, EncodingRef};

use crate::barcode::*;
use crate::consts;
use crate::img::Image;

/// Timeout for sending/receiving USB messages
pub const TIMEOUT: u64 = 200;

/// SupportedPrinters enumerates the list of printers that this library knows
/// about. Should be easy to add your own to this library or you could try
/// using an existing one if the command set is similar.
#[derive(Clone, Copy)]
pub enum SupportedPrinters {
    /// Tested on the SNBC BTP-R880NPV
    SNBC,
    /// Tested on the Custom P3 printer
    P3,
    Unknown, // Adding to allow _ no not raise warnings to make adding printers easier
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("USB error: {:?}", 0)]
    Usb(rusb::Error),

    #[error("IO error: {:?}", 0)]
    Io(std::io::Error),

    #[error("Invalid device index")]
    InvalidIndex,

    #[error("Invalid argument")]
    InvalidArgument,

    #[error("No supported languages")]
    NoLanguages,

    #[error("Unable to locate expected endpoints")]
    InvalidEndpoints,

    #[error("Operation timeout")]
    Timeout,

    #[error("Unsupported printer")]
    Unsupported,
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<rusb::Error> for Error {
    fn from(e: rusb::Error) -> Self {
        Error::Usb(e)
    }
}

#[derive(Clone, Debug)]
pub struct UsbInfo {
    /// vendor_id is the USB vendor id used when initializing the printer
    pub vendor_id: u16,
    /// product_id is the USB product id used when initializing the printer
    pub product_id: u16,
    /// manufacturer is a string as defined in libusb for the device
    pub manufacturer: String,
    /// product is a string as defined in libusb for the device
    pub product: String,
    // It seems serial is pretty useless on these printers
    // neither the P3 or SNBC returned anything meaningful
    // here. P3 has a command to get the serial number
    // pub serial: String,
}

/// Allows for printing to a [::device]
/// TODO: This example is outdated
///
/// # Example
/// ```rust
/// use std::fs::File;
/// use posify::printer::Printer;
/// use tempfile::NamedTempFileOptions;
///
/// fn main() -> std::Result, Error<()> {
///     // TODO: Fix this example as NamedTempFileOptions is out of date
///     let tempf = tempfile::NamedTempFileOptions::new().create().unwrap();
///     let file = File::from(tempf);
///     let mut printer = Printer::new(file, None, None, SupportedPrinters::P3);
///
///     printer
///       .chain_size(0,0)?
///       .chain_text("The quick brown fox jumped over the lazy dog")?
///       .chain_feed(1)?
///       .flush()
/// }
/// ```
pub struct Printer {
    codec: EncodingRef,
    trap: EncoderTrap,
    printer: SupportedPrinters,
    _device: rusb::Device<rusb::GlobalContext>,
    handle: rusb::DeviceHandle<rusb::GlobalContext>,
    descriptor: rusb::DeviceDescriptor,
    timeout: Duration,

    /// USB Vendor ID
    vid: u16,
    /// USB Product ID
    pid: u16,
    /// USB Command Endpoint (output)
    cmd_ep: u8,
    /// USB Status Endpoint (input)
    stat_ep: u8,
}

impl Printer {
    pub fn new(
        codec: Option<EncodingRef>,
        trap: Option<EncoderTrap>,
        printer: SupportedPrinters,
        vid: u16,
        pid: u16,
    ) -> Result<Self, Error> {
        // Iterate over the devices to find the printer
        let mut matches: Vec<_> = rusb::devices()?
            .iter()
            // Filter out the devices that match the vendor_id and product_id (should only be 1)
            .filter_map(|d| {
                let desc = match d.device_descriptor() {
                    Ok(d) => d,
                    Err(_) => {
                        return None;
                    }
                };
                if desc.vendor_id() == vid && desc.product_id() == pid {
                    Some((d, desc))
                } else {
                    None
                }
            })
            .collect();
        let (device, descriptor) = matches.remove(0);

        let mut handle = device.open()?;

        let config_desc = match device.config_descriptor(0) {
            Ok(v) => v,
            Err(e) => {
                return Err(e.into());
            }
        };

        let interface = match config_desc.interfaces().next() {
            Some(x) => x,
            None => {
                return Err(Error::InvalidEndpoints);
            }
        };

        let (mut cmd_ep, mut stat_ep) = (None, None);

        for interface_desc in interface.descriptors() {
            for endpoint_desc in interface_desc.endpoint_descriptors() {
                match (endpoint_desc.transfer_type(), endpoint_desc.direction()) {
                    (rusb::TransferType::Bulk, rusb::Direction::In) => {
                        stat_ep = Some(endpoint_desc.address())
                    }
                    (rusb::TransferType::Bulk, rusb::Direction::Out) => {
                        cmd_ep = Some(endpoint_desc.address())
                    }
                    (_, _) => continue,
                }
            }
        }

        let (cmd_ep, stat_ep) = match (cmd_ep, stat_ep) {
            (Some(cmd), Some(stat)) => (cmd, stat),
            _ => {
                return Err(Error::InvalidEndpoints);
            }
        };

        match handle.kernel_driver_active(interface.number())? {
            true => {
                handle.detach_kernel_driver(interface.number())?;
            }
            false => {
                log::trace!("Kernel driver inactive");
            }
        }
        let _ = handle.claim_interface(interface.number());

        Ok(Printer {
            // file,
            codec: codec.unwrap_or(UTF_8 as EncodingRef),
            trap: trap.unwrap_or(EncoderTrap::Replace),
            printer,
            _device: device,
            handle,
            descriptor,
            timeout: Duration::from_millis(TIMEOUT),
            vid,
            pid,
            cmd_ep,
            stat_ep,
        })
    }

    pub fn info(&mut self) -> Result<UsbInfo, Error> {
        let languages = self.handle.read_languages(self.timeout)?;
        let language = languages[0];
        // let active_config = self.handle.active_configuration()?;
        // println!("Active Config: {:?}", active_config);

        let manufacturer =
            self.handle
                .read_manufacturer_string(language, &self.descriptor, self.timeout)?;
        let product = self
            .handle
            .read_product_string(language, &self.descriptor, self.timeout)?;
        // Serial is pretty useless on these printers and doesn't work for SNBC
        // let serial =
        //     self.handle
        //         .read_serial_number_string(language, &self.descriptor, self.timeout)?;
        Ok(UsbInfo {
            vendor_id: self.vid,
            product_id: self.pid,
            manufacturer,
            product,
            // serial,
        })
    }

    // --------------------------------------------------

    fn encode(&mut self, content: &str) -> io::Result<Vec<u8>> {
        self.codec
            .encode(content, self.trap)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }

    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let n_bytes = self.handle.write_bulk(self.cmd_ep, buf, self.timeout)?;
        if n_bytes != buf.len() {
            return Err(Error::Timeout);
        }

        Ok(n_bytes)
    }
    // Old file based write
    // fn write2(&mut self, buf: &[u8]) -> io::Result<usize> {
    //     self.file.write(buf)
    // }

    pub fn chain_write_u8(&mut self, n: u8) -> Result<&mut Self, Error> {
        self.write_u8(n).map(|_| self)
    }
    pub fn write_u8(&mut self, n: u8) -> Result<usize, Error> {
        self.write(vec![n].as_slice())
    }

    fn write_u16le(&mut self, n: u16) -> Result<usize, Error> {
        let mut wtr = vec![];
        wtr.write_u16::<LittleEndian>(n)?;
        self.write(wtr.as_slice())
    }

    // Useful when using a file handler, probably not needed now
    pub fn flush(&mut self) -> Result<(), Error> {
        // self.file.flush()
        Ok(())
    }

    /// ESC @ - Initialize printer, clear data in print buffer and set print mode
    /// to the default mode when powered on.
    ///
    /// Seems to be the same for SNBC and P3 printers
    ///
    /// ASCII    ESC   @
    /// Hex      1b   40
    /// Decimal  27   64
    /// Notes:
    ///   - The data in the receive buffer is not cleared
    ///   - The macro definition is not cleared
    ///   - The NV bitmap data is not cleared (SNBC, not sure about P3)
    pub fn hwinit(&mut self) -> Result<usize, Error> {
        self.write(&[0x1b, 0x40])
    }
    pub fn chain_hwinit(&mut self) -> Result<&mut Self, Error> {
        self.hwinit().map(|_| self)
    }

    /// ESC = n - Enable/Disable Printer
    /// Docs describe this as "Select printer to which host computer sends data"
    ///
    /// SNBC:
    ///
    /// ASCII    ESC   =  n
    /// Hex      1b   3d  n
    /// Decimal  27   61  n
    /// Range: 0 <= n <= 1
    ///
    /// Meaning of n is as follows:
    ///
    /// | Bit | 1/0 | Hex | Decimal | Function         |
    /// |-----|-----|-----|---------|------------------|
    /// |  0  |  0  |  00 |    0    | Printer disabled |
    /// |  0  |  1  |  01 |    1    | Printer enabled  |
    /// | 1-7 |     |     |         | Undefined        |
    ///
    /// Notes:
    /// When the printer is disabled, it ignores all commands except for
    /// real-time commands (DLE EOT, DLE ENQ, DLE DC4) until it is enabled by
    /// this command.
    ///
    /// Default: n = 1
    ///
    /// P3:
    /// Select the device to which the host computer sends data, using n as follows:
    ///
    /// |      n       | Function        |
    /// |--------------|-----------------|
    /// |  0x01, 0x03  | Device enabled  |
    /// |     0x02     | Device disabled |
    ///
    /// Default: n = 0x01

    pub fn enable(&mut self) -> Result<usize, Error> {
        match self.printer {
            SupportedPrinters::SNBC => self.write(&[0x1b, 0x3d, 0x01]),
            SupportedPrinters::P3 => self.write(&[0x1b, 0x3d, 0x01]),
            _ => Err(Error::Unsupported),
        }
    }
    pub fn chain_enable(&mut self) -> Result<&mut Self, Error> {
        self.enable().map(|_| self)
    }

    pub fn disable(&mut self) -> Result<usize, Error> {
        match self.printer {
            SupportedPrinters::SNBC => self.write(&[0x1b, 0x3d, 0x00]),
            SupportedPrinters::P3 => self.write(&[0x1b, 0x3d, 0x02]),
            _ => Err(Error::Unsupported),
        }
    }
    pub fn chain_disable(&mut self) -> Result<&mut Self, Error> {
        self.disable().map(|_| self)
    }

    // TODO: There doesn't seem to be a hwreset command for snbc
    // pub fn hwreset(&mut self) -> io::Result<usize> {
    //     self.write(consts::HW_RESET)
    // }
    // pub fn chain_hwreset(&mut self) -> io::Result<&mut Self> {
    //     self.hwreset().map(|_| self)
    // }

    pub fn print(&mut self, content: &str) -> Result<usize, Error> {
        // let rv = self.encode(content);
        let rv = self.encode(content)?;
        self.write(rv.as_slice())
    }
    pub fn chain_print(&mut self, content: &str) -> Result<&mut Self, Error> {
        self.print(content).map(|_| self)
    }

    pub fn println(&mut self, content: &str) -> Result<usize, Error> {
        self.print(format!("{}{}", content, "\n").as_ref())
    }
    pub fn chain_println(&mut self, content: &str) -> Result<&mut Self, Error> {
        self.println(content).map(|_| self)
    }

    // TODO: This seems useless? just use print/println?
    pub fn text(&mut self, content: &str) -> Result<usize, Error> {
        self.println(content)
    }
    pub fn chain_text(&mut self, content: &str) -> Result<&mut Self, Error> {
        self.text(content).map(|_| self)
    }

    pub fn underline_mode(&mut self, mode: Option<&str>) -> Result<usize, Error> {
        let mode = mode.unwrap_or("OFF");
        let mode_upper = mode.to_uppercase();
        match mode_upper.as_ref() {
            "OFF" => Ok(self.write(&[0x1b, 0x2d, 0x00])?),
            "ON" => Ok(self.write(&[0x1b, 0x2d, 0x01])?),
            "THICK" => Ok(self.write(&[0x1b, 0x2d, 0x02])?),
            _ => Ok(self.write(&[0x1b, 0x2d, 0x00])?),
        }
    }
    pub fn chain_underline_mode(&mut self, mode: Option<&str>) -> Result<&mut Self, Error> {
        self.underline_mode(mode).map(|_| self)
    }

    /// ESC 2/ESC 3 n - Set line spacing
    ///
    /// ESC 2 (0x1b, 0x32) Sets line spacing to default
    /// ESC 3 (0x1b, 0x33, n) Specifies a specific line spacing
    ///
    /// ASCII    ESC   2
    /// Hex      1b   32
    /// Decimal  27   50
    ///
    /// ASCII    ESC   3  n
    /// Hex      1b   33  n
    /// Decimal  27   51  n
    /// Range: 0 <= n <= 255
    ///
    /// Notes:
    ///   - The line spacing can be set independently in standard mode and in
    ///     page mode.
    ///   - The horizontal and vertical motion units are specified by GS P.
    ///     Changing the horizontal or vertical motion unit does not affect the
    ///     current line spacing.
    ///   - In standard mode, the vertical motion unit (y) is used.
    ///   - In page mode, this command functions as follows, depending on the
    ///     direction and starting position of the printable area:
    ///     1) When the starting position is set to the upper left or lower right
    ///        of the printable area by ESC T, the vertical motion unit (y) is
    ///        used.
    ///     2) When the starting position is set to the upper right or lower left
    ///        of the printable area by ESC T, the horizontal motion unit (x) is
    ///        used.
    ///   - The maximum paper feed amount is 1016 mm (40 inches). Even if a paper
    ///     feed amount of more than 1016 mm (40 inches) is set, the printer
    ///     feeds the paper only 1016 mm (40 inches).
    ///
    /// Default: The default line spacing is approximately 4.23mm (1/6 inches).
    pub fn line_space(&mut self, n: i32) -> Result<usize, Error> {
        if (0..=255).contains(&n) {
            Ok(self.write(&[0x1b, 0x33, n as u8])?)
        } else {
            self.write(&[0x1b, 0x32])
        }
    }
    pub fn chain_line_space(&mut self, n: i32) -> Result<&mut Self, Error> {
        self.line_space(n).map(|_| self)
    }

    pub fn feed(&mut self, n: usize) -> Result<usize, Error> {
        let n = if n < 1 { 1 } else { n };
        self.write("\n".repeat(n).as_ref())
    }
    pub fn chain_feed(&mut self, n: usize) -> Result<&mut Self, Error> {
        self.feed(n).map(|_| self)
    }

    pub fn chain_control(&mut self, ctrl: &str) -> Result<&mut Self, Error> {
        self.control(ctrl).map(|_| self)
    }
    pub fn control(&mut self, ctrl: &str) -> Result<usize, Error> {
        let ctrl_upper = ctrl.to_uppercase();
        let ctrl_value = match ctrl_upper.as_ref() {
            "LF" => consts::CTL_LF,
            "FF" => consts::CTL_FF,
            "CR" => consts::CTL_CR,
            "HT" => consts::CTL_HT,
            "VT" => consts::CTL_VT,
            _ => return Err(Error::Unsupported),
        };
        self.write(ctrl_value)
    }

    pub fn chain_align(&mut self, alignment: &str) -> Result<&mut Self, Error> {
        self.align(alignment).map(|_| self)
    }
    pub fn align(&mut self, alignment: &str) -> Result<usize, Error> {
        let align_upper = alignment.to_uppercase();
        let align_value = match align_upper.as_ref() {
            "LT" => consts::TXT_ALIGN_LT,
            "CT" => consts::TXT_ALIGN_CT,
            "RT" => consts::TXT_ALIGN_RT,
            _ => return Err(Error::InvalidArgument),
        };
        self.write(align_value)
    }

    pub fn chain_font(&mut self, family: &str) -> Result<&mut Self, Error> {
        self.font(family).map(|_| self)
    }
    pub fn font(&mut self, family: &str) -> Result<usize, Error> {
        let family_upper = family.to_uppercase();
        let family_value = match family_upper.as_ref() {
            "A" => consts::TXT_FONT_A,
            "B" => consts::TXT_FONT_B,
            "C" => consts::TXT_FONT_C,
            _ => return Err(Error::InvalidArgument),
        };
        self.write(family_value)
    }

    pub fn chain_style(&mut self, kind: &str) -> Result<&mut Self, Error> {
        self.style(kind).map(|_| self)
    }
    pub fn style(&mut self, kind: &str) -> Result<usize, Error> {
        let kind_upper = kind.to_uppercase();
        match kind_upper.as_ref() {
            "B" => Ok(self.write(consts::TXT_UNDERL_OFF)? + self.write(consts::TXT_BOLD_ON)?),
            "U" => Ok(self.write(consts::TXT_BOLD_OFF)? + self.write(consts::TXT_UNDERL_ON)?),
            "U2" => Ok(self.write(consts::TXT_BOLD_OFF)? + self.write(consts::TXT_UNDERL2_ON)?),
            "BU" => Ok(self.write(consts::TXT_BOLD_ON)? + self.write(consts::TXT_UNDERL_ON)?),
            "BU2" => Ok(self.write(consts::TXT_BOLD_ON)? + self.write(consts::TXT_UNDERL2_ON)?),
            // "NORMAL" | _ =>
            _ => Ok(self.write(consts::TXT_BOLD_OFF)? + self.write(consts::TXT_UNDERL_OFF)?),
        }
    }

    pub fn chain_size(&mut self, width: usize, height: usize) -> Result<&mut Self, Error> {
        self.size(width, height).map(|_| self)
    }
    pub fn size(&mut self, width: usize, height: usize) -> Result<usize, Error> {
        let mut n = self.write(consts::TXT_NORMAL)?;
        if width == 2 {
            n += self.write(consts::TXT_2WIDTH)?;
        }
        if height == 2 {
            n += self.write(consts::TXT_2HEIGHT)?;
        }
        Ok(n)
    }

    pub fn chain_barcode(
        &mut self,
        code: &str,
        kind: BarcodeType,
        position: TextPosition,
        font: Font,
        width: u8,
        height: u8,
    ) -> Result<&mut Self, Error> {
        self.barcode(code, kind, position, font, width, height)
            .map(|_| self)
    }
    pub fn barcode(
        &mut self,
        code: &str,
        kind: BarcodeType,
        position: TextPosition,
        font: Font,
        width: u8,
        height: u8,
    ) -> Result<usize, Error> {
        let mut n = 0;
        let mut bc = Barcode {
            printer: self.printer,
            width,
            height,
            position,
            font,
            kind,
        };
        n += self.write(&bc.set_width()?)?;
        n += self.write(&bc.set_height())?;
        n += self.write(&bc.set_text_position())?;
        n += self.write(&bc.set_font())?;
        n += self.write(&bc.set_barcode_type())?;

        // Code128 requires the Code Set to be sent before the barcode text
        //
        // Currently we just default to Code B, but we might want to think about
        // allowing the selection of the code set
        //
        // 128A (Code Set A) – ASCII characters 00 to 95 (0–9, A–Z and control codes), special characters, and FNC 1–4
        // 128B (Code Set B) – ASCII characters 32 to 127 (0–9, A–Z, a–z), special characters, and FNC 1–4
        // 128C (Code Set C) – 00–99 (encodes two digits with a single code point) and FNC1
        if kind == BarcodeType::Code128 {
            // self.write(&[0x7b_u8, 0x41_u8])?; // Code Set A
            self.write(&[0x7b_u8, 0x42_u8])?; // Code Set B
                                              // self.write(&[0x7b_u8, 0x43_u8])?; // Code Set C
        }
        self.write(code.as_bytes())?;
        self.write(&[0x00_u8])?; // Need to send NULL to finish
        Ok(n)
    }

    #[cfg(feature = "qrcode")]
    pub fn chain_qrimage(&mut self) -> Result<&mut Self, Error> {
        self.qrimage().map(|_| self)
    }
    #[cfg(feature = "qrcode")]
    pub fn qrimage(&mut self) -> Result<usize, Error> {
        Ok(0)
    }

    #[cfg(feature = "qrcode")]
    pub fn chain_qrcode(
        &mut self,
        code: &str,
        version: Option<i32>,
        level: &str,
        size: Option<i32>,
    ) -> Result<&mut Self, Error> {
        self.qrcode(code, version, level, size).map(|_| self)
    }
    #[cfg(feature = "qrcode")]
    pub fn qrcode(
        &mut self,
        code: &str,
        version: Option<i32>,
        level: &str,
        size: Option<i32>,
    ) -> Result<usize, Error> {
        let level = level.to_uppercase();
        let level_value = match level.as_ref() {
            "M" => consts::QR_LEVEL_M,
            "Q" => consts::QR_LEVEL_Q,
            "H" => consts::QR_LEVEL_H,
            // "L" | _ =>
            _ => consts::QR_LEVEL_L,
        };
        let mut n = 0;
        n += self.write(consts::TYPE_QR)?;
        n += self.write(consts::CODE2D)?;
        n += self.write_u8(version.unwrap_or(3) as u8)?;
        n += self.write(level_value)?;
        n += self.write_u8(size.unwrap_or(3) as u8)?;
        n += self.write_u16le(code.len() as u16)?;
        n += self.write(code.as_bytes())?;
        Ok(n)
    }

    pub fn chain_cashdraw(&mut self, pin: i32) -> Result<&mut Self, Error> {
        self.cashdraw(pin).map(|_| self)
    }
    pub fn cashdraw(&mut self, pin: i32) -> Result<usize, Error> {
        let pin_value = if pin == 5 {
            consts::CD_KICK_5
        } else {
            consts::CD_KICK_2
        };
        self.write(pin_value)
    }

    pub fn chain_full_cut(&mut self) -> Result<&mut Self, Error> {
        self.full_cut().map(|_| self)
    }

    pub fn full_cut(&mut self) -> Result<usize, Error> {
        match self.printer {
            SupportedPrinters::SNBC => self.write(&[0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x00]),
            // p3 seems to only support partial cut
            _ => Err(Error::Unsupported),
        }
    }

    pub fn chain_partial_cut(&mut self) -> Result<&mut Self, Error> {
        self.partial_cut().map(|_| self)
    }

    pub fn partial_cut(&mut self) -> Result<usize, Error> {
        match self.printer {
            SupportedPrinters::SNBC => self.write(&[0x0a, 0x0a, 0x0a, 0x1d, 0x56, 0x01]),
            SupportedPrinters::P3 => self.write(&[0x0a, 0x0a, 0x0a, 0x1b, 0x6d]),
            _ => Err(Error::Unsupported),
        }
    }

    pub fn chain_bit_image(
        &mut self,
        image: &Image,
        density: Option<&str>,
    ) -> Result<&mut Self, Error> {
        self.bit_image(image, density).map(|_| self)
    }
    pub fn bit_image(&mut self, image: &Image, density: Option<&str>) -> Result<usize, Error> {
        let density = density.unwrap_or("d24");
        let density_upper = density.to_uppercase();
        let header = match density_upper.as_ref() {
            "S8" => consts::BITMAP_S8,
            "D8" => consts::BITMAP_D8,
            "S24" => consts::BITMAP_S24,
            // "D24" | _ =>
            _ => consts::BITMAP_D24,
        };
        let n = if density == "s8" || density == "d8" {
            1
        } else {
            3
        };
        let mut n_bytes = 0;
        n_bytes += self.line_space(0)?;
        for line in image.bitimage_lines(n * 8) {
            n_bytes += self.write(header)?;
            n_bytes += self.write_u16le((line.len() / n as usize) as u16)?;
            n_bytes += self.write(line.as_ref())?;
            n_bytes += self.feed(1)?;
        }
        Ok(n_bytes)
    }

    pub fn chain_raster(&mut self, image: &Image, mode: Option<&str>) -> Result<&mut Self, Error> {
        self.raster(image, mode).map(|_| self)
    }
    pub fn raster(&mut self, image: &Image, mode: Option<&str>) -> Result<usize, Error> {
        let mode_upper = mode.unwrap_or("NORMAL").to_uppercase();
        let header = match mode_upper.as_ref() {
            // Double Wide
            "DW" => &[0x1d, 0x76, 0x30, 0x01],
            // Double Height
            "DH" => &[0x1d, 0x76, 0x30, 0x02],
            // Quadruple
            "QD" => &[0x1d, 0x76, 0x30, 0x03],
            // "NORMAL" | _ =>
            _ => &[0x1d, 0x76, 0x30, 0x00],
        };
        let mut n_bytes = 0;
        n_bytes += self.write(header)?;
        n_bytes += self.write_u16le(((image.width + 7) / 8) as u16)?;
        n_bytes += self.write_u16le(image.height as u16)?;
        n_bytes += self.write(image.get_raster().as_ref())?;
        Ok(n_bytes)
    }

    pub fn get_serial(&mut self) -> Result<String, Error> {
        match self.printer {
            SupportedPrinters::P3 => {
                self.write(&[0x1c, 0xea, 0x52])?;
                let mut buffer = [0_u8; 16];
                let _ = self
                    .handle
                    .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
                let value = std::str::from_utf8(&buffer).unwrap();
                Ok(value.to_string())
            }
            _ => Err(Error::Unsupported),
        }
    }

    pub fn get_cut_count(&mut self) -> Result<String, Error> {
        self.write(&[0x1d, 0xe2]).unwrap();
        let mut buffer = [0_u8; 16]; // TODO: This is more than enough now... but what about as
                                     // cuts increase?
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        let value = std::str::from_utf8(&buffer).unwrap(); // This seems to trim the padding
        Ok(value.to_string())
    }

    pub fn get_rom_version(&mut self) -> Result<String, Error> {
        self.write(&[0x1d, 0x49, 0x03]).unwrap();
        let mut buffer = [0_u8; 4];
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        let value = std::str::from_utf8(&buffer).unwrap();
        Ok(value.to_string())
    }

    pub fn get_power_count(&mut self) -> Result<String, Error> {
        self.write(&[0x1d, 0xe5]).unwrap();
        let mut buffer = [0_u8; 8];
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        let value = std::str::from_utf8(&buffer).unwrap();
        Ok(value.to_string())
    }

    pub fn get_printed_length(&mut self) -> Result<String, Error> {
        self.write(&[0x1d, 0xe3]).unwrap();
        let mut buffer = [0_u8; 8];
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        let value = std::str::from_utf8(&buffer).unwrap();
        Ok(value.to_string())
    }

    pub fn get_remaining_paper(&mut self) -> Result<String, Error> {
        self.write(&[0x1d, 0xe1]).unwrap();
        let mut buffer = [0_u8; 8];
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        let value = std::str::from_utf8(&buffer).unwrap();
        Ok(value.to_string())
    }

    /// starting with a value in centimeters, calculate nH and nL as follows:
    /// nH = <cm> / 256
    /// nL = <cm> - (nH * 256)
    ///
    /// So if we wanted to calculated based on 15 meters:
    /// 15m = 1500cm
    /// nH = 1500 / 256 = 5
    /// nL = 1500 - (nH * 256) = 1500 - (5 * 256) = 220
    ///
    /// Then convert to hex:
    /// 5 = 0x05
    /// 220 = 0xdc
    pub fn set_paper_end_limit(&mut self) -> Result<(), Error> {
        // TODO: what should we pass in, length in meters and then calculate?
        let n_l: u8 = 0x00;
        let n_h: u8 = 0x00;
        self.write(&[0x1d, 0xe6, n_h, n_l]).unwrap();
        Ok(())
    }

    pub fn paper_loaded(&mut self) -> Result<bool, Error> {
        self.write(&[0x1d, 0x72, 0x01]).unwrap();
        let mut buffer = [0_u8; 1];
        let _ = self
            .handle
            .read_bulk(self.stat_ep, &mut buffer, self.timeout)?;
        Ok(buffer[0] == 0x00_u8)
    }

    // TODO: Flesh this out more
    // So `0x10, 0x04, n` can get a few different status results:
    // | n    | Type |
    // |------|------|
    // | 0x01 | device status |
    // | 0x02 | off-line status |
    // | 0x03 | error status |
    // | 0x04 | paper roll sensor status |
    // | 0x11 | print status |
    // | 0x14 | full status |
    // | 0x15 | device id |
    //
    // We should probably evaluate what we want to get and implement it here
    // Below is an example using off-line status to get state of paper door
    pub fn get_status(&mut self) -> Result<String, Error> {
        self.write(&[0x10, 0x04, 0x02]).unwrap();
        let mut buffer = [0_u8; 16];

        self.read(&mut buffer)?;
        let status = &buffer[0];
        let mask = 1 << 2;
        if status & mask != 0 {
            return Ok("Cover open".to_string());
        }

        Ok("No Errors".to_string())
    }

    pub fn read(&mut self, buf: &mut [u8; 16]) -> Result<(), Error> {
        let _ = self.handle.read_bulk(self.stat_ep, buf, self.timeout)?;
        Ok(())
    }
}
