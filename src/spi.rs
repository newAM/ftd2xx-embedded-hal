use crate::{Ft232hInner, PinUse};
use embedded_hal::spi::Polarity;
use libftd2xx::{ClockData, ClockDataOut, FtdiCommon, MpsseCmdBuilder, TimeoutError};
use std::{cell::RefCell, sync::Mutex};

/// FTDI SPI interface.
pub struct Spi<'a> {
    /// Parent FTDI device.
    mtx: &'a Mutex<RefCell<Ft232hInner>>,
    /// MPSSE command used to clock data in and out simultaneously.
    ///
    /// This is set by [`Spi::set_clock_polarity`].
    clk: ClockData,
    /// MPSSE command used to clock data out.
    ///
    /// This is set by [`Spi::set_clock_polarity`].
    clk_out: ClockDataOut,
}

impl<'a> Spi<'a> {
    pub(crate) fn new(mtx: &Mutex<RefCell<Ft232hInner>>) -> Result<Spi, TimeoutError> {
        let lock = mtx.lock().expect("Failed to aquire FTDI mutex");
        let mut inner = lock.borrow_mut();
        inner.allocate_pin(0, PinUse::MpsseSpi);
        inner.allocate_pin(1, PinUse::MpsseSpi);
        inner.allocate_pin(2, PinUse::MpsseSpi);

        // clear direction of first 3 pins
        inner.direction &= !0x07;
        // set SCK (AD0) and MOSI (AD1) as output pins
        inner.direction |= 0x03;

        // set GPIO pins to new state
        let cmd: MpsseCmdBuilder = MpsseCmdBuilder::new()
            .set_gpio_lower(inner.value, inner.direction)
            .send_immediate();
        inner.ft.write_all(cmd.as_slice())?;

        Ok(Spi {
            mtx,
            clk: ClockData::MsbPosIn,
            clk_out: ClockDataOut::MsbNeg,
        })
    }

    /// Set the SPI clock polarity.
    ///
    /// FTD2XX devices only supports [SPI mode] 0 and 2, clock phase is fixed.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use embedded_hal::spi::Polarity;
    /// use ftd2xx_embedded_hal as hal;
    ///
    /// let ftdi = hal::Ft232hHal::new()?.init_default()?;
    /// let mut spi = ftdi.spi()?;
    /// spi.set_clock_polarity(Polarity::IdleLow);
    /// # Ok::<(), std::boxed::Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// [SPI mode]: https://en.wikipedia.org/wiki/Serial_Peripheral_Interface#Mode_numbers
    pub fn set_clock_polarity(&mut self, cpol: Polarity) {
        let (clk, clk_out) = match cpol {
            Polarity::IdleLow => (ClockData::MsbPosIn, ClockDataOut::MsbNeg),
            Polarity::IdleHigh => (ClockData::MsbNegIn, ClockDataOut::MsbPos),
        };

        // destructuring assignments are unstable
        self.clk = clk;
        self.clk_out = clk_out
    }
}

impl<'a> embedded_hal::blocking::spi::Write<u8> for Spi<'a> {
    type Error = TimeoutError;
    fn write(&mut self, words: &[u8]) -> Result<(), Self::Error> {
        let cmd: MpsseCmdBuilder = MpsseCmdBuilder::new()
            .clock_data_out(self.clk_out, words)
            .send_immediate();

        let lock = self.mtx.lock().expect("Failed to aquire FTDI mutex");
        let mut inner = lock.borrow_mut();
        inner.ft.write_all(cmd.as_slice())
    }
}

impl<'a> embedded_hal::blocking::spi::Transfer<u8> for Spi<'a> {
    type Error = TimeoutError;
    fn transfer<'w>(&mut self, words: &'w mut [u8]) -> Result<&'w [u8], Self::Error> {
        let cmd: MpsseCmdBuilder = MpsseCmdBuilder::new()
            .clock_data(self.clk, words)
            .send_immediate();

        let lock = self.mtx.lock().expect("Failed to aquire FTDI mutex");
        let mut inner = lock.borrow_mut();
        inner.ft.write_all(cmd.as_slice())?;
        inner.ft.read_all(words)?;

        Ok(words)
    }
}

impl<'a> embedded_hal::spi::FullDuplex<u8> for Spi<'a> {
    type Error = TimeoutError;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buf: [u8; 1] = [0];
        let cmd: MpsseCmdBuilder = MpsseCmdBuilder::new()
            .clock_data(self.clk, &buf)
            .send_immediate();

        let lock = self.mtx.lock().expect("Failed to aquire FTDI mutex");
        let mut inner = lock.borrow_mut();
        inner.ft.write_all(cmd.as_slice())?;
        inner.ft.read_all(&mut buf)?;

        Ok(buf[0])
    }

    fn send(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        let cmd: MpsseCmdBuilder = MpsseCmdBuilder::new()
            .clock_data_out(self.clk_out, &[byte])
            .send_immediate();

        let lock = self.mtx.lock().expect("Failed to aquire FTDI mutex");
        let mut inner = lock.borrow_mut();
        inner.ft.write_all(cmd.as_slice())?;
        Ok(())
    }
}
