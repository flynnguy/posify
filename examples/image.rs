use image;
use std::io;
use std::fs::OpenOptions;

use posify::img;
use posify::printer::{Printer, SupportedPrinters};

fn main() -> io::Result<()> {
    let logo = image::open("rust.png")
        .expect("File not found!")
        .resize(256, 256, image::imageops::Lanczos3);
    let logo = img::Image::from(logo);

    let file = OpenOptions::new()
        .write(true)
        .open("/dev/usb/lp0").unwrap();
    let mut printer = Printer::new(file, None, None, SupportedPrinters::P3);

    printer
        .chain_hwinit()?
        .chain_align("ct")?
        .chain_raster(&logo, None)?
        .chain_feed(1)?
        .chain_partial_cut()?
        .flush()
}
