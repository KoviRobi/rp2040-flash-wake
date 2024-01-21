#![no_std]
#![no_main]

use panic_halt as _;

use rp2040_pac as _;

extern "C" {
    #[link_name = "__vector_table"]
    static VECTOR_TABLE: u32;
}

#[cortex_m_rt_macros::entry]
fn main() -> ! {
    let core = cortex_m::Peripherals::take().unwrap();
    unsafe { core.SCB.vtor.write(&VECTOR_TABLE as *const u32 as u32) };
    panic!();
}
