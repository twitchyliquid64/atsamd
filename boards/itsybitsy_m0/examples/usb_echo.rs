#![no_std]
#![no_main]

extern crate itsybitsy_m0 as hal;
extern crate panic_halt;
extern crate usbd_serial;
extern crate usb_device;
extern crate cortex_m;

use hal::clock::GenericClockController;
use hal::delay::Delay;
use hal::prelude::*;
use hal::entry;
use hal::pac::{interrupt, CorePeripherals, Peripherals};

use hal::dbgprint;
use hal::time::Hertz;
use hal::{uart, uart_debug};

use hal::usb::UsbBus;
use usb_device::bus::UsbBusAllocator;

use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use cortex_m::asm::delay as cycle_delay;
use cortex_m::peripheral::NVIC;

#[entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.PM,
        &mut peripherals.SYSCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut pins = hal::Pins::new(peripherals.PORT);
    unsafe {
        RED_LED = Some(pins.d13.into_open_drain_output(&mut pins.port));
        RED_LED.as_mut().map(|led| {
            led.set_low().unwrap();
        });
    }
    uart_debug::wire_uart(hal::uart(
        &mut clocks,
        Hertz(115200),
        peripherals.SERCOM0,
        &mut peripherals.PM,
        pins.d0,
        pins.d1,
        &mut pins.port,
    ));
    dbgprint!("\n\n\n\n~========== STARTING ==========~\n");

    let bus_allocator = unsafe {
        USB_ALLOCATOR = Some(hal::usb_allocator(
            peripherals.USB,
            &mut clocks,
            &mut peripherals.PM,
            pins.usb_dm,
            pins.usb_dp,
            &mut pins.port,
        ));
        USB_ALLOCATOR.as_ref().unwrap()
    };

    unsafe {
        USB_SERIAL = Some(SerialPort::new(&bus_allocator));
        USB_BUS = Some(
            UsbDeviceBuilder::new(&bus_allocator, UsbVidPid(0x16c0, 0x27dd))
                .manufacturer("Fake company")
                .product("Serial port")
                .serial_number("TEST")
                .device_class(USB_CLASS_CDC)
                .build(),
        );
    }

    unsafe {
        core.NVIC.set_priority(interrupt::USB, 1);
        NVIC::unmask(interrupt::USB);
    }

    loop {
        // cycle_delay(5 * 1024 * 1024);
        // red_led.set_high().unwrap();
        // cycle_delay(5 * 1024 * 1024);
        // red_led.set_low().unwrap();
    }
}

static mut RED_LED: Option<hal::gpio::Pa17<hal::gpio::Output<hal::gpio::OpenDrain>>> = None;
static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
static mut USB_BUS: Option<UsbDevice<UsbBus>> = None;
static mut USB_SERIAL: Option<SerialPort<UsbBus>> = None;

fn poll_usb() {
    unsafe {
        RED_LED.as_mut().map(|led| led.toggle() );
        USB_BUS.as_mut().map(|usb_dev| {
            USB_SERIAL.as_mut().map(|serial| {
                usb_dev.poll(&mut [serial]);
                let mut buf = [0u8; 64];

                if let Ok(count) = serial.read(&mut buf) {
                    for (i, c) in buf.iter().enumerate() {
                        if i > count {
                            break;
                        }
                        serial.write(&[c.clone()]);
                    }
                };
            });
        });
    };
}

#[interrupt]
fn USB() {
    poll_usb();
}
