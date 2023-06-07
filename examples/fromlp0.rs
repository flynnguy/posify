use std::fs::OpenOptions;
use std::io;

use posify::printer::{BarcodeType, Font, Printer, SupportedPrinters, TextPosition};

fn main() -> io::Result<()> {
    let device_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/usb/lp0").unwrap();

    let mut printer = Printer::new(device_file, None, None, SupportedPrinters::P3);

    printer
        .chain_hwinit()?
        .chain_align("ct")?
        .chain_underline_mode(Some("thick"))?
        .chain_text("Underlined Text")?
        .chain_underline_mode(Some("off"))?
        .chain_text("The quick brown fox jumps over the lazy dog")?
        .chain_feed(1)?
        .chain_barcode("0123456789023",
            BarcodeType::Code128,
            TextPosition::Below,
            Font::FontA,
            2,
            0x40)?
        .chain_feed(1)?
        .chain_partial_cut()?
        .flush()
}
