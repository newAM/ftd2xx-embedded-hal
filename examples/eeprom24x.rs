//! This example writes to and reads from the 24x serial EEPROM

use eeprom24x::Eeprom24x;
use eeprom24x::SlaveAddr;
use ftd2xx_embedded_hal as hal;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let ftdi = hal::Ft232hHal::new()
        .expect("Failed to open FT232H device")
        .init_default()
        .expect("Failed to initialize MPSSE");

    let i2c = ftdi.i2c().expect("Failed to initialize I2C");
    let mut eeprom = Eeprom24x::new_24x04(i2c, SlaveAddr::default());
    let delay = Duration::from_millis(5);
    let byte_w = 0xe5;
    let addr = 0x0;

    eeprom.write_byte(addr, byte_w).unwrap();
    sleep(delay);

    let byte_r = eeprom.read_byte(addr).unwrap();
    assert_eq!(byte_w, byte_r);
    println!("read and write byte: {:#x}", byte_r);
}
