//! Delays

use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;

use crate::clock::GenericClockController;
use crate::time::Hertz;
use hal::blocking::delay::{DelayMs, DelayUs};

use core::sync::atomic::{
    AtomicU32,
    Ordering::Relaxed,
};

/// System timer (SysTick) as a polling-driven delay provider
pub struct Delay {
    sysclock: Hertz,
    syst: SYST,
}

impl Delay {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new(mut syst: SYST, clocks: &mut GenericClockController) -> Self {
        syst.set_clock_source(SystClkSource::Core);

        Delay {
            syst,
            sysclock: clocks.gclk0().into(),
        }
    }

    /// Releases the system timer (SysTick) resource
    pub fn free(self) -> SYST {
        self.syst
    }
}

impl DelayMs<u32> for Delay {
    fn delay_ms(&mut self, ms: u32) {
        self.delay_us(ms * 1_000);
    }
}

impl DelayMs<u16> for Delay {
    fn delay_ms(&mut self, ms: u16) {
        self.delay_ms(ms as u32);
    }
}

impl DelayMs<u8> for Delay {
    fn delay_ms(&mut self, ms: u8) {
        self.delay_ms(ms as u32);
    }
}

impl DelayUs<u32> for Delay {
    fn delay_us(&mut self, us: u32) {
        // The SysTick Reload Value register supports values between 1 and 0x00FFFFFF.
        const MAX_RVR: u32 = 0x00FF_FFFF;

        let mut total_rvr = us * (self.sysclock.0 / 1_000_000);

        while total_rvr != 0 {
            let current_rvr = if total_rvr <= MAX_RVR {
                total_rvr
            } else {
                MAX_RVR
            };

            self.syst.set_reload(current_rvr);
            self.syst.clear_current();
            self.syst.enable_counter();

            // Update the tracking variable while we are waiting...
            total_rvr -= current_rvr;

            while !self.syst.has_wrapped() {}

            self.syst.disable_counter();
        }
    }
}

impl DelayUs<u16> for Delay {
    fn delay_us(&mut self, us: u16) {
        self.delay_us(us as u32)
    }
}

impl DelayUs<u8> for Delay {
    fn delay_us(&mut self, us: u8) {
        self.delay_us(us as u32)
    }
}


/// Static controller for an Interrupt-driven System timer (SysTick) with
/// a 1ms interval.
pub struct MultiDelayController {
    syst: core::cell::UnsafeCell<SYST>,
    wraps: core::cell::UnsafeCell<AtomicU32>, // Should increment every 1ms.
}

impl MultiDelayController {
    /// Configures the system timer (SysTick) as a delay provider
    pub fn new<'a>(mut syst: SYST, clocks: &mut GenericClockController) -> Self {
        let sysclock: Hertz = clocks.gclk0().into();
        syst.disable_counter();
        syst.disable_interrupt();
        syst.set_clock_source(SystClkSource::Core);

        let ticks_per_ms = sysclock.0 / 1_000;
        syst.set_reload(ticks_per_ms);
        syst.clear_current();

        Self {
            syst: core::cell::UnsafeCell::new(syst),
            wraps: core::cell::UnsafeCell::new(AtomicU32::new(0)),
        }
    }

    #[doc(hidden)]
    pub fn handle_interrupt(&self) {
        unsafe {
            if (*self.syst.get()).has_wrapped() {
                (*self.wraps.get()).fetch_add(1, Relaxed);
            }
        }
    }

    #[doc(hidden)]
    pub unsafe fn enable(&mut self) {
        (*self.syst.get()).enable_interrupt();
        (*self.syst.get()).enable_counter();
    }

    /// Returns the number of 16ms intervals that have occurred.
    fn get_wraps(&self) -> u32 {
        unsafe {
            (*self.wraps.get()).load(Relaxed)
        }
    }

    pub fn delay_ms(&self, num: u32) {
        let start_wraps = self.get_wraps();
        while num > 0 && self.get_wraps() != start_wraps.wrapping_add(num) {
            cortex_m::asm::wfi();
        }
    }
}

// /// Cloneable type, each of which lets you delay independently.
// #[derive(Clone,Copy)]
// pub struct MultiDelayBuilder<'a> {
//     alias: core::ptr::NonNull<MultiDelayController>,
//     pd: core::marker::PhantomData<&'a ()>,
// }
//
// impl<'a> MultiDelayBuilder<'a> {
//     pub fn wait_lol(&self) -> u32 {
//         unsafe {
//             self.alias.as_ref().wraps.load(Relaxed)
//         }
//     }
// }

#[macro_export]
macro_rules! systick_delay {
    ($name:ident) => {
            use cortex_m::interrupt as _systick_interrupt;
            use cortex_m::peripheral::SYST as _systick_syst;
            use atsamd_hal::clock::GenericClockController as _systick_ClockController;
            use atsamd_hal::delay as _systick_delay;

            static mut $name: Option<_systick_delay::MultiDelayController> = None;

            fn init_systick_delay<'a>(syst: _systick_syst, clocks: &mut _systick_ClockController) -> &'a _systick_delay::MultiDelayController { // -> _systick_delay::MultiDelayBuilder {
                _systick_interrupt::free(|_| {
                    // No need to check if already initialized as we consume a required
                    // value (SYST).
                    let controller = _systick_delay::MultiDelayController::new(syst, clocks);
                    unsafe {
                        $name = Some(controller)
                    };
                    unsafe {
                        $name.as_mut().map(|s| s.enable())
                    };
                    unsafe {
                        $name.as_ref().unwrap()
                    }
                })
            }


            #[cortex_m_rt::exception]
            #[allow(non_upper_case_globals, unused_unsafe)]
            unsafe fn SysTick() {
                _systick_interrupt::free(|_| {
                    $name.as_ref().map(|s| s.handle_interrupt())
                });
            }
    };
}
