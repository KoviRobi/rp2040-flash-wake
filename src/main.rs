#![no_std]
#![no_main]

use panic_halt as _;

use rp2040_pac as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    panic!();
}
