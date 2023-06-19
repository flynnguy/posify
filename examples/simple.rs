use std::error::Error;

use posify::barcode::{BarcodeType, Font, TextPosition};
use posify::printer::{Printer, SupportedPrinters};

fn main() -> Result<(), Box<dyn Error>> {
    let vid: u16 = 0x154f;
    let pid: u16 = 0x0517;

    let mut printer = Printer::new(None, None, SupportedPrinters::P3, vid, pid)?;

    let _ = printer
        .chain_hwinit()?
        .chain_align("ct")?
        .chain_underline_mode(Some("thick"))?
        .chain_text("Underlined Text")?
        .chain_underline_mode(Some("off"))?
        .chain_text("The quick brown fox jumps over the lazy dog")?
        .chain_feed(1)?
        .chain_barcode(
            "0123456789023",
            BarcodeType::Code128,
            TextPosition::Below,
            Font::FontA,
            2,
            0x40,
        )?
        .chain_feed(5)?
        .chain_partial_cut()?
        .flush();

    Ok(())
}
