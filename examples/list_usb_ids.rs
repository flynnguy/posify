use std::time::Duration;

/// The following should output a list of usb ids and a description
/// of the device to help you find the device ids for your program
///
/// Alternatively you can use lsusb
fn main() {
    for device in rusb::devices().unwrap().iter() {
        let timeout = Duration::from_millis(200);
        let device_desc = device.device_descriptor().unwrap();
        let handle = device.open().unwrap();
        let language = handle.read_languages(timeout).unwrap()[0];
        let manufacturer = match handle.read_manufacturer_string(language, &device_desc, timeout) {
            Ok(m) => m,
            Err(_) => {
                println!(
                    "Bus {:03} Device {:03} ID {:04x}:{:04x}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id(),
                );
                continue;
            }
        };
        let product = match handle.read_product_string(language, &device_desc, timeout) {
            Ok(p) => p,
            Err(_) => {
                println!(
                    "Bus {:03} Device {:03} ID {:04x}:{:04x} - {}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id(),
                    manufacturer,
                );
                continue;
            }
        };

        println!(
            "Bus {:03} Device {:03} ID {:04x}:{:04x} - {} {}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id(),
            manufacturer,
            product,
        );
    }
}
