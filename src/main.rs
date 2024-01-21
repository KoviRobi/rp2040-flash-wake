#![no_std]
#![no_main]

use panic_halt as _;

use rp_pico as bsp;

use bsp::{hal, pac};
use cortex_m::prelude::*;
use hal::fugit::ExtU32;

extern "C" {
    #[link_name = "__vector_table"]
    static VECTOR_TABLE: u32;
}

#[cortex_m_rt_macros::entry]
fn main() -> ! {
    unsafe {
        const SIO_BASE: u32 = 0xd0000000;
        const SPINLOCK0_PTR: *mut u32 = (SIO_BASE + 0x100) as *mut u32;
        const SPINLOCK_COUNT: usize = 32;
        for i in 0..SPINLOCK_COUNT {
            SPINLOCK0_PTR.wrapping_add(i).write_volatile(1);
        }
    }
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    unsafe { core.SCB.vtor.write(&VECTOR_TABLE as *const u32 as u32) };

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let _clocks = hal::clocks::init_clocks_and_plls(
        bsp::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    watchdog.start(5.secs());

    panic!();
}
