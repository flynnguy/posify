use std::time::Duration;

use posify::barcode::{BarcodeType, Font, TextPosition};
use posify::printer::{self, Printer, SupportedPrinters};

fn main() -> Result<(), printer::Error> {
    let mut vid = 0x0000;
    let mut pid = 0x0000;

    // The following is an example of how to get the vid, pid based on the
    // manufacturer string. It's easier if you already have the vid/pid
    // and you can just define them above and remove this whole block
    // of code
    for device in rusb::devices().unwrap().iter() {
        let timeout = Duration::from_millis(200);
        let device_desc = device.device_descriptor().unwrap();
        let handle = device.open().unwrap();
        let language = handle.read_languages(timeout).unwrap()[0];
        match handle.read_manufacturer_string(language, &device_desc, timeout) {
            Ok(m) => {
                if m.starts_with("SNBC") {
                    vid = device_desc.vendor_id();
                    pid = device_desc.product_id();
                } else {
                    continue;
                }
            }
            Err(_) => continue,
        }
    }

    let mut printer = Printer::new(None, None, SupportedPrinters::SNBC, vid, pid)?;

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
