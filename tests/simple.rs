extern crate tempfile;

extern crate posify;

use posify::barcode::{BarcodeType, Font, TextPosition};
use posify::printer::{Printer, SupportedPrinters};

#[test]
fn simple() {
    let vid: u16 = 0x154f;
    let pid: u16 = 0x0517;

    let mut printer = Printer::new(None, None, SupportedPrinters::SNBC, vid, pid).unwrap();

    let _ = printer
        .chain_hwinit()
        .unwrap()
        .chain_align("ct")
        .unwrap()
        .chain_underline_mode(Some("thick"))
        .unwrap()
        .chain_text("Code128")
        .unwrap()
        .chain_underline_mode(Some("off"))
        .unwrap()
        .chain_feed(1)
        .unwrap()
        .chain_barcode(
            "0123456",
            BarcodeType::Code128,
            TextPosition::Below,
            Font::FontA,
            2,
            0x40,
        )
        .unwrap()
        .chain_feed(5)
        .unwrap()
        .chain_partial_cut()
        .unwrap();
}
