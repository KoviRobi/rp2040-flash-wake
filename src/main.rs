#![no_std]
#![no_main]

use panic_halt as _;

use rp_pico as bsp;

use bsp::{hal, pac};
use cortex_m::prelude::*;
use hal::fugit::ExtU32;
use hal::usb::UsbBus as UsbBusImpl;
use hal::Clock;
use pac::interrupt;
use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_serial::SerialPort;

/// The USB Device Driver (shared with the interrupt).
static mut USB_DEVICE: Option<UsbDevice<UsbBusImpl>> = None;

/// The USB Bus Driver (shared with the interrupt).
static mut USB_BUS: Option<UsbBusAllocator<UsbBusImpl>> = None;

/// The USB Serial Device Driver (shared with the interrupt).
static mut USB_SERIAL: Option<SerialPort<UsbBusImpl>> = None;

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
    let clocks = hal::clocks::init_clocks_and_plls(
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

    let usb_bus = UsbBusAllocator::new(UsbBusImpl::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));
    // Safety: Interrupts not yet enabled
    unsafe { USB_BUS = Some(usb_bus) };
    let bus_ref = unsafe { USB_BUS.as_ref().unwrap() };

    let usb_serial = SerialPort::new(bus_ref);
    // Safety: Interrupts not yet enabled
    unsafe { USB_SERIAL = Some(usb_serial) };

    let usb_dev = UsbDeviceBuilder::new(bus_ref, UsbVidPid(0x1234, 0xabcd))
        .manufacturer("KoviRobi")
        .product("RP2040 flash wake")
        .serial_number("Test")
        .build();
    unsafe { USB_DEVICE = Some(usb_dev) };

    // Safety: No USB after this outside of the interrupt handler
    unsafe { pac::NVIC::unmask(bsp::hal::pac::Interrupt::USBCTRL_IRQ) };

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    loop {
        delay.delay_ms(1000);
        watchdog.feed();
    }
}

#[allow(non_snake_case)]
#[interrupt]
fn USBCTRL_IRQ() {
    let usb_dev = unsafe { USB_DEVICE.as_mut().unwrap() };
    let serial = unsafe { USB_SERIAL.as_mut().unwrap() };
    let mut buf = [0u8; 64];

    if usb_dev.poll(&mut [serial]) {
        match serial.read(&mut buf) {
            Err(_e) => {
                // Do nothing
            }
            Ok(0) => {
                // Do nothing
            }
            Ok(count) => {
                for b in buf.iter_mut().take(count) {
                    if *b == b'r' {
                        bsp::hal::rom_data::reset_to_usb_boot(1 << 25, 0);
                    }
                }
            }
        }
    }
}
