use posify::barcode::{BarcodeType, Font, TextPosition};
use posify::printer::{self, Printer, SupportedPrinters};

fn main() -> Result<(), printer::Error> {
    let (mfg, vid, pid) = Printer::get_mfg_info().unwrap();
    println!("{:?}: ({:?}:{:?})", mfg, vid, pid);

    let mut printer = Printer::new(None, None, mfg, vid, pid)?;

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
