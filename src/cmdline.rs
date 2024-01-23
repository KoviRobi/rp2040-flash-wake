use crate::byte_str::{ParseByteStr, ParseNumErr, ToByteStr};
use crate::{bsp, hal, pac, SerialPortT};
use heapless::Vec;

enum ParseArgErr {
    /// Bad argument at index .0, failed to parse because of .1
    BadArg(usize, ParseNumErr),
    NotEnoughArgs,
    TooManyArgs,
    UnknownCommand,
}

fn parse_rest_args<T, const N: usize>(
    args: &[&[u8]],
    arg_index: usize,
    buf: &mut Vec<T, N>,
) -> Result<(), ParseArgErr>
where
    T: ParseByteStr<ParseErr = ParseNumErr>,
{
    for (arg, i) in args.iter().zip(arg_index..) {
        match T::parse(arg) {
            Ok(n) => {
                buf.push(n).or(Err(ParseArgErr::TooManyArgs))?;
            }
            Err(err) => return Err(ParseArgErr::BadArg(i, err)),
        }
    }
    Ok(())
}

/// Should only be called from one place, and this should be the only thing
/// manipulating QSPI (after setup by main, done before interrupts [or whatever
/// calls this] is enabled)
fn flash_cmd(cmd: &mut [u8]) {
    // Safety: We are not executing from flash
    unsafe {
        hal::rom_data::connect_internal_flash();
        hal::rom_data::flash_exit_xip();
    }

    let io_qspi = unsafe { &*pac::IO_QSPI::ptr() };
    let qspi_ss = io_qspi.gpio_qspiss();
    let xip_ssi = unsafe { &*pac::XIP_SSI::ptr() };

    // CSn low (chip select)
    qspi_ss.gpio_ctrl.write(|w| w.outover().low());

    for byte in cmd {
        loop {
            if xip_ssi.sr.read().tfnf().bit_is_set() {
                xip_ssi.dr0.write(|w| w.dr().variant(*byte as u32));
                break;
            }
        }
        loop {
            if xip_ssi.sr.read().rfne().bit_is_set() {
                *byte = xip_ssi.dr0.read().bits() as u8;
                break;
            }
        }
    }

    // CSn high (chip deselect)
    qspi_ss.gpio_ctrl.write(|w| w.outover().high());

    // Safety: We are not executing from flash
    unsafe {
        hal::rom_data::flash_flush_cache();
        hal::rom_data::flash_enter_cmd_xip();
    }
}

fn exec_cmd(serial: &mut SerialPortT, args: &mut [u8]) {
    flash_cmd(args);
    let _ = serial.write(b" ->");
    for byte in args {
        let mut buf: Vec<u8, 2> = Vec::new();
        byte.to_string::<2, 16>(&mut buf).ok().unwrap();
        let _ = serial.write(b" ");
        let _ = serial.write(buf.as_slice());
    }
    let _ = serial.write(b"\r\n");
}

fn parse_with_error<const NUM_ARGS: usize>(
    serial: &mut SerialPortT,
    cmdline: &[&[u8]],
) -> Result<(), ParseArgErr> {
    match cmdline[0] {
        b"h" | b"help" => {
            let _ = serial.write(
                b"Commands:\r\n\
                  \r\n\
                  h|help: print this help\r\n\
                  b|boot: bootloader\r\n\
                  c|cmd|command BYTE*: send bytes to flash\r\n\
                  r|read ADDR [WORDS=1]: read words from ADDR\r\n\
                  w|write ADDR WORD+: write words to ADDR\r\n",
            );
        }
        b"b" | b"boot" => bsp::hal::rom_data::reset_to_usb_boot(1 << 25, 0),
        b"c" | b"cmd" | b"command" => {
            let mut bytes: Vec<u8, NUM_ARGS> = Vec::new();
            parse_rest_args(&cmdline[1..], 1, &mut bytes)?;
            exec_cmd(serial, &mut bytes);
        }
        b"r" | b"read" => {
            use ParseArgErr::*;
            match cmdline.len() {
                1 => Err(NotEnoughArgs)?,
                2 | 3 => {}
                _ => Err(TooManyArgs)?,
            }
            let start = u32::parse(cmdline[1]).map_err(|err| BadArg(1, err))?;
            let words = if cmdline.len() == 3 {
                usize::parse(cmdline[2]).map_err(|err| BadArg(2, err))?
            } else {
                1
            };
            let mut buf: Vec<u8, 8> = Vec::new();
            let _ = serial.write(b"\r\n");
            for addr in (start..).step_by(4).take(words) {
                let ptr = addr as *const u32;
                unsafe { &*ptr }.to_string::<8, 16>(&mut buf).ok().unwrap();
                let _ = serial.write(b" 0x");
                let _ = serial.write(buf.as_slice());
                buf.clear();
            }
            let _ = serial.write(b"\r\n");
        }
        b"w" | b"write" => {
            use ParseArgErr::BadArg;
            if cmdline.len() < 3 {
                Err(ParseArgErr::NotEnoughArgs)?;
            }
            let start = u32::parse(cmdline[1]).map_err(|err| BadArg(1, err))?;
            let mut words: Vec<u32, NUM_ARGS> = Vec::new();
            parse_rest_args(&cmdline[2..], 2, &mut words)?;
            for (addr, data) in (start..).step_by(4).zip(words) {
                let ptr = addr as *mut u32;
                *unsafe { &mut *ptr } = data;
            }
            let _ = serial.write(b"\r\n");
        }
        _ => Err(ParseArgErr::UnknownCommand)?,
    }
    Ok(())
}

pub fn parse<const NUM_ARGS: usize>(serial: &mut SerialPortT, cmdline: &mut [u8]) {
    let cmd = cmdline
        .split(|c| c.is_ascii_whitespace())
        .filter(|c| c != &[])
        .collect::<Vec<_, NUM_ARGS>>();
    if cmd.is_empty() {
        let _ = serial.write(b"\r\n");
        return;
    }
    use ParseArgErr::*;
    use ParseNumErr::*;
    match parse_with_error::<NUM_ARGS>(serial, &cmd) {
        Ok(()) => {}
        Err(UnknownCommand) => {
            let _ = serial.write(b"Unknown command \"");
            let _ = serial.write(cmdline);
            let _ = serial.write(b"\"\r\n");
        }
        Err(NotEnoughArgs) => {
            let _ = serial.write(b"Not enough arguments\r\n");
        }
        Err(TooManyArgs) => {
            let _ = serial.write(b"Too many arguments\r\n");
        }
        Err(BadArg(arg_no, BadDigit(chr))) => {
            let _ = serial.write(b"Bad argument ");
            let mut buf: Vec<u8, 3> = Vec::new();
            let _ = (arg_no as u8).to_string::<3, 10>(&mut buf);
            let _ = serial.write(buf.as_slice());
            let _ = serial.write(b" `");
            let _ = serial.write(&[chr]);
            let _ = serial.write(b"` (digit parse)\r\n");
        }
        Err(BadArg(arg_no, Overflow)) => {
            let _ = serial.write(b"Bad argument ");
            let mut buf: Vec<u8, 3> = Vec::new();
            let _ = (arg_no as u8).to_string::<3, 10>(&mut buf);
            let _ = serial.write(buf.as_slice());
            let _ = serial.write(b" (number overflow)\r\n");
        }
        Err(BadArg(_, BadBase(_))) => {
            let _ = serial.write(b"Bad numerical base\r\n");
        }
    }
}
