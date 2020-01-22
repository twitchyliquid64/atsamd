use super::sercom::*;
use crate::gpio;
use cortex_m::interrupt::free as disable_interrupts;


pub static mut WRITER: DbgWriter = DbgWriter {uart: None};

pub struct DbgWriter{
    uart: Option<UART0<
        Sercom0Pad3<gpio::Pa11<gpio::PfC>>,
        Sercom0Pad2<gpio::Pa10<gpio::PfC>>,
        (),
        (),
    >>,
}

impl ::core::fmt::Write for DbgWriter
{
    fn write_str(&mut self, s: &str) -> ::core::fmt::Result {
        match &mut self.uart {
            Some(uart) => uart.write_str(s),
            None => Ok(()),
        }
    }
}

pub fn wire_uart(
    uart: UART0<
        Sercom0Pad3<gpio::Pa11<gpio::PfC>>,
        Sercom0Pad2<gpio::Pa10<gpio::PfC>>,
        (),
        (),
    >,
) {
    disable_interrupts(|_| unsafe {
        WRITER = DbgWriter{uart: Some(uart)};
    });
}
