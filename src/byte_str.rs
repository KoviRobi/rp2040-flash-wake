use heapless::Vec;

pub trait ToByteStr {
    type FormatErr;
    fn to_string<const N: usize, const BASE: u8>(
        &self,
        buf: &mut Vec<u8, N>,
    ) -> Result<(), Self::FormatErr>;
}

pub trait ParseByteStr: Sized {
    type ParseErr;
    fn parse(buf: &[u8]) -> Result<Self, Self::ParseErr>;
}

pub enum FormatNumErr {
    /// Not enough bytes to fully format the number
    Overrun,
}

fn to_string<const N: usize, const BASE: u8>(
    mut n: usize,
    buf: &mut Vec<u8, N>,
) -> Result<(), FormatNumErr> {
    loop {
        let div = n / BASE as usize;
        let rem = n % BASE as usize;
        if rem < 10 {
            buf.push(b'0' + rem as u8)
        } else {
            buf.push(b'A' - 10 + rem as u8)
        }
        .or(Err(FormatNumErr::Overrun))?;
        n = div;
        if n == 0 {
            break;
        }
    }
    buf.reverse();
    Ok(())
}

impl ToByteStr for u8 {
    type FormatErr = FormatNumErr;

    fn to_string<const N: usize, const BASE: u8>(
        &self,
        buf: &mut Vec<u8, N>,
    ) -> Result<(), Self::FormatErr> {
        to_string::<N, BASE>(*self as usize, buf)
    }
}

impl ToByteStr for u32 {
    type FormatErr = FormatNumErr;

    fn to_string<const N: usize, const BASE: u8>(
        &self,
        buf: &mut Vec<u8, N>,
    ) -> Result<(), Self::FormatErr> {
        to_string::<N, BASE>(*self as usize, buf)
    }
}

pub enum ParseNumErr {
    /// Failed to parse number because of character .0 causes overflow
    /// (left-to-right parse)
    Overflow,
    /// Failed to parse number because of character .0 cannot be parsed as a
    /// number
    BadDigit(u8),
    /// Bad numeric base
    BadBase(u8),
}

fn parse_digit(n: usize, chr: u8, base: u8) -> Result<usize, ParseNumErr> {
    use ParseNumErr::*;
    if !(2..36).contains(&base) {
        return Err(BadBase(base));
    }
    let m = n.checked_mul(base as usize).ok_or(Overflow)?;
    if chr.is_ascii_digit() && chr - b'0' < base {
        m.checked_add((chr - b'0') as usize).ok_or(Overflow)
    } else if chr.is_ascii_lowercase() && chr + 10 - b'a' < base {
        m.checked_add((chr + 10 - b'a') as usize).ok_or(Overflow)
    } else if chr.is_ascii_uppercase() && chr + 10 - b'A' < base {
        m.checked_add((chr + 10 - b'A') as usize).ok_or(Overflow)
    } else if chr == b'_' {
        // Ignore, to use as delimiter
        Ok(n)
    } else {
        Err(BadDigit(chr))
    }
}

fn parse_num(buf: &[u8]) -> Result<usize, ParseNumErr> {
    if buf.starts_with(b"0x") || buf.starts_with(b"0X") {
        buf.iter()
            .skip(2)
            .try_fold(0, |acc, chr| parse_digit(acc, *chr, 16))
    } else if buf.starts_with(b"0o") {
        buf.iter()
            .skip(2)
            .try_fold(0, |acc, chr| parse_digit(acc, *chr, 8))
    } else if buf.starts_with(b"0b") || buf.starts_with(b"0B") {
        buf.iter()
            .skip(2)
            .try_fold(0, |acc, chr| parse_digit(acc, *chr, 1))
    } else {
        buf.iter()
            .try_fold(0, |acc, chr| parse_digit(acc, *chr, 10))
    }
}

impl ParseByteStr for u8 {
    type ParseErr = ParseNumErr;

    fn parse(buf: &[u8]) -> Result<Self, Self::ParseErr> {
        parse_num(buf)?.try_into().or(Err(ParseNumErr::Overflow))
    }
}

impl ParseByteStr for u16 {
    type ParseErr = ParseNumErr;

    fn parse(buf: &[u8]) -> Result<Self, Self::ParseErr> {
        parse_num(buf)?.try_into().or(Err(ParseNumErr::Overflow))
    }
}

impl ParseByteStr for u32 {
    type ParseErr = ParseNumErr;

    fn parse(buf: &[u8]) -> Result<Self, Self::ParseErr> {
        parse_num(buf)?.try_into().or(Err(ParseNumErr::Overflow))
    }
}

impl ParseByteStr for usize {
    type ParseErr = ParseNumErr;

    fn parse(buf: &[u8]) -> Result<Self, Self::ParseErr> {
        parse_num(buf)
    }
}
