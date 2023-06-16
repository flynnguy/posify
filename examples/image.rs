use posify::img;
use posify::printer::{Printer, SupportedPrinters};

fn main() -> Result<(), posify::printer::Error> {
    let logo = image::open("rust.png").expect("File not found!").resize(
        256,
        256,
        image::imageops::Lanczos3,
    );
    let logo = img::Image::from(logo);

    let vid: u16 = 0x154f;
    let pid: u16 = 0x0517;

    let mut printer = Printer::new(None, None, SupportedPrinters::P3, vid, pid).unwrap();

    let _ = printer
        .chain_hwinit()?
        .chain_align("ct")?
        .chain_raster(&logo, None)?
        .chain_feed(1)?
        .chain_partial_cut()?
        .flush();
    Ok(())
}
