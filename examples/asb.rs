use posify::printer::{Printer, SupportedPrinters};

// First Byte
const OFFLINE_BIT: u8 = 3;
const DOOR_STATUS_BIT: u8 = 5;
const PAPER_FEED_BIT: u8 = 6;
// Second Byte
const AUTO_CUTTER_BIT: u8 = 3;
const RECOVERABLE_BIT: u8 = 5;
const AUTOMATIC_RECOVERABLE_BIT: u8 = 6;
// Third Byte
const PAPER_NEAR_END_BIT: u8 = 0;
const PAPER_BIT: u8 = 2;

fn main() {
    let vid: u16 = 0x154f;
    let pid: u16 = 0x0517;

    let mut printer = Printer::new(None, None, SupportedPrinters::SNBC, vid, pid).unwrap();

    // Enable Automatic Status Back
    printer.write(&[0x1D, 0x61, 0x01]).unwrap();

    let mut buffer = [0_u8; 16];
    let mut prev_status = buffer[0];

    // Constantly read looking for errors
    loop {
        printer.read(&mut buffer).unwrap();
        if buffer[0] != prev_status {
            prev_status = buffer[0];
            println!("Status[0]: {:0>8b}", buffer[0]);
            println!("Status[1]: {:0>8b}", buffer[1]);
            println!("Status[2]: {:0>8b}", buffer[2]);
            println!("Status[3]: {:0>8b}", buffer[3]);

            // First Byte
            if ((buffer[0] >> OFFLINE_BIT) & 1) == 1 {
                println!("Printer Offline");
            } else {
                println!("Printer Online");
            }
            if ((buffer[0] >> DOOR_STATUS_BIT) & 1) == 1 {
                println!("Door opened?");
            } else {
                println!("Door closed?");
            }
            if ((buffer[0] >> PAPER_FEED_BIT) & 1) == 1 {
                println!("Paper Feed Active");
            } else {
                println!("Paper Feed Inactive");
            }

            // Second Byte
            if ((buffer[1] >> AUTO_CUTTER_BIT) & 1) == 1 {
                println!("Auto cutter error.");
            } else {
                println!("No auto cutter error");
            }
            if ((buffer[1] >> RECOVERABLE_BIT) & 1) == 1 {
                println!("Recoverable error");
            } else {
                println!("No recoverable error");
            }
            if ((buffer[1] >> AUTOMATIC_RECOVERABLE_BIT) & 1) == 1 {
                println!("Automatically Recoverable error");
            } else {
                println!("No automatic recoverable error");
            }

            // Third Byte
            if ((buffer[2] >> PAPER_NEAR_END_BIT) & 0b11) == 0b11 {
                println!("Paper is near end");
            } else {
                println!("Paper is not near end");
            }
            if ((buffer[2] >> PAPER_BIT) & 0b11) == 0b11 {
                println!("Paper end");
            } else {
                println!("Paper present");
            }
            // Fourth byte seems to be unused
            println!("=============");
        }
    }
}
