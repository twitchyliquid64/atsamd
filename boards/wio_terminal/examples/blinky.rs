#![no_std]
#![no_main]

use panic_halt as _;
use wio_terminal as wio;

use wio::hal::clock::GenericClockController;
use wio::hal::systick_delay;
use wio::pac::{CorePeripherals, Peripherals};
use wio::prelude::*;
use wio::{entry, Pins, Sets};

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    //let mut delay = Delay::new(core.SYST, &mut clocks);

    let delay_builder = init_systick_delay(core.SYST, &mut clocks);

    let mut sets: Sets = Pins::new(peripherals.PORT).split();
    let mut user_led = sets.user_led.into_open_drain_output(&mut sets.port);
    user_led.set_low().unwrap();

    loop {
        delay_builder.delay_ms(250);
        user_led.toggle();
    }
}

systick_delay!(SYSTICK_TIMER);
