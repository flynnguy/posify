use posify::printer::{Printer, SupportedPrinters};

fn main() {
    let vid: u16 = 0x0613;
    let pid: u16 = 0x8800;

    let mut printer = Printer::new(None, None, SupportedPrinters::Epic, vid, pid).unwrap();

    let value = printer.get_firmware_checksum().unwrap();

    println!("firmware_version: {:?}", value.to_string());

    let value = printer.get_firmware_id().unwrap();

    println!("firmware_id: {:?}", value.to_string());
}
