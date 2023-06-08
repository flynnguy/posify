use std::io;
use std::fs::OpenOptions;

use posify::printer::{Printer, SupportedPrinters};

fn main() -> io::Result<()> {
    let tempf = OpenOptions::new()
        .write(true)
        .create(true)
        .open("/tmp/printer_test.txt")
        .unwrap();

    let mut printer = Printer::new(tempf, None, None, SupportedPrinters::P3);

    printer
        .chain_hwinit()?
        .chain_align("ct")?
        .chain_style("bu")?
        .chain_size(0, 0)?
        .chain_text("The quick brown fox jumps over the lazy dog")?
        .chain_text("敏捷的棕色狐狸跳过懒狗")?
        .chain_feed(1)?
        .flush()
}
