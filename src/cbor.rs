#[cfg(any(feature = "arbitrary", test))]
use arbitrary::{Arbitrary, Unstructured};
use num_bigint::{BigInt, Sign};

use crate::{Error, FromCbor, IntoCbor, Result};

use std::{
    cmp,
    convert::{TryFrom, TryInto},
    io,
};

macro_rules! read_r {
    ($r:ident, $buf:expr) => {
        err_at!(IOError, $r.read_exact($buf))?
    };
}

macro_rules! write_w {
    ($w:ident, $buf:expr) => {
        err_at!(IOError, $w.write($buf))?
    };
}

/// Recursion limit for nested Cbor objects.
pub const RECURSION_LIMIT: u32 = 1000;

/// Cbor type enumerated over its major variants.
///
/// Use one of the conversion trait to convert language-native-type to a
/// Cbor variant. For lazy decoding, use [Cbor::Binary] variant.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Cbor {
    Major0(Info, u64),              // uint 0-23,24,25,26,27
    Major1(Info, u64),              // nint 0-23,24,25,26,27
    Major2(Info, Vec<u8>),          // byts 0-23,24,25,26,27,31
    Major3(Info, Vec<u8>),          // text 0-23,24,25,26,27,31
    Major4(Info, Vec<Cbor>),        // list 0-23,24,25,26,27,31
    Major5(Info, Vec<(Key, Cbor)>), // dict 0-23,24,25,26,27,31
    Major6(Info, Tag),              // tags similar to major0
    Major7(Info, SimpleValue),      // type refer SimpleValue
    Binary(Vec<u8>),                // for lazy decoding cbor data
}

#[cfg(any(feature = "arbitrary", test))]
impl<'a> Arbitrary<'a> for Cbor {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        use Cbor::*;

        let major = u.arbitrary::<u8>()? % 8;
        let val: Cbor = match major {
            0 => {
                let val: u64 = u.arbitrary()?;
                let info: Info = val.into();
                Major0(info, val)
            }
            1 => {
                let val: u64 = {
                    let val: i64 = u.arbitrary()?;
                    val.abs().try_into().unwrap()
                };
                let info: Info = val.into();
                Major1(info, val)
            }
            2 => {
                let val: Vec<u8> = u.arbitrary()?;
                let n: u64 = val.len().try_into().unwrap();
                let info: Info = n.into();
                Major2(info, val)
            }
            3 => {
                let val: String = u.arbitrary()?;
                let n: u64 = val.len().try_into().unwrap();
                let info: Info = n.into();
                Major3(info, val.as_bytes().to_vec())
            }
            4 => {
                let val: Vec<Cbor> = u.arbitrary()?;
                let n: u64 = val.len().try_into().unwrap();
                let info: Info = n.into();
                Major4(info, val)
            }
            5 => {
                let val: Vec<(Key, Cbor)> = u.arbitrary()?;
                let n: u64 = val.len().try_into().unwrap();
                let info: Info = n.into();
                Major5(info, val)
            }
            6 => {
                let tag: Tag = u.arbitrary()?;
                tag.into()
            }
            7 => {
                let sval: SimpleValue = u.arbitrary()?;
                sval.into_cbor().unwrap()
            }
            _ => unreachable!(),
        };

        Ok(val)
    }
}

impl Cbor {
    fn pretty_print(&self, p: &str) -> Result<String> {
        use std::str::from_utf8;
        use Cbor::{
            Binary, Major0, Major1, Major2, Major3, Major4, Major5, Major6, Major7,
        };

        let s = match self {
            Major0(info, val) => {
                format!("{}Maj0({},0x{:x})", p, info.pretty_print()?, val)
            }
            Major1(info, val) => {
                format!("{}Maj1({},0x{:x})", p, info.pretty_print()?, val)
            }
            Major2(_info, val) => format!("{}Byts({},{:?})", p, val.len(), val),
            Major3(_info, val) => {
                let txt = from_utf8(val).unwrap();
                format!("{}Text({},{:?})", p, val.len(), txt)
            }
            Major4(_info, vals) => {
                let mut ss = vec![format!("{}List({})", p, vals.len())];
                let p = p.to_owned() + "  ";
                for val in vals.iter() {
                    ss.push(val.pretty_print(&p)?);
                }
                ss.join("\n")
            }
            Major5(_info, vals) => {
                let mut ss = vec![format!("{}Dict({})", p, vals.len())];
                let p = p.to_owned() + "  ";
                for (key, val) in vals.iter() {
                    ss.push(key.pretty_print()?);
                    ss.push(val.pretty_print(&p)?);
                }
                ss.join("\n")
            }
            Major6(_info, val) => format!("{}{}", p, val.pretty_print(p)?),
            Major7(info, val) => format!(
                "{}Maj7({},{})",
                p,
                info.pretty_print()?,
                val.pretty_print()?
            ),
            Binary(bytes) => Cbor::decode(&mut bytes.as_slice())?.0.pretty_print(p)?,
        };

        Ok(s)
    }
}

impl Cbor {
    /// Serialize this cbor value.
    pub fn encode<W>(&self, w: &mut W) -> Result<usize>
    where
        W: io::Write,
    {
        self.do_encode(w, 1)
    }

    fn do_encode<W>(&self, w: &mut W, depth: u32) -> Result<usize>
    where
        W: io::Write,
    {
        if depth > RECURSION_LIMIT {
            return err_at!(FailCbor, msg: "encode recursion limit exceeded");
        }

        let major = self.to_major_val();
        let n = match self {
            Cbor::Major0(info, num) => {
                let n = encode_hdr(major, *info, w)?;
                n + encode_addnl(*num, w)?
            }
            Cbor::Major1(info, num) => {
                let n = encode_hdr(major, *info, w)?;
                n + encode_addnl(*num, w)?
            }
            Cbor::Major2(info, byts) => {
                let n = encode_hdr(major, *info, w)?;
                let m =
                    encode_addnl(err_at!(FailConvert, u64::try_from(byts.len()))?, w)?;
                write_w!(w, byts);
                n + m + byts.len()
            }
            Cbor::Major3(info, text) => {
                let n = encode_hdr(major, *info, w)?;
                let m = encode_addnl(err_at!(FailCbor, u64::try_from(text.len()))?, w)?;
                write_w!(w, text);
                n + m + text.len()
            }
            Cbor::Major4(info, list) => {
                let n = encode_hdr(major, *info, w)?;
                let m =
                    encode_addnl(err_at!(FailConvert, u64::try_from(list.len()))?, w)?;
                let mut acc = 0;
                for x in list.iter() {
                    acc += x.do_encode(w, depth + 1)?;
                }
                n + m + acc
            }
            Cbor::Major5(info, map) => {
                let n = encode_hdr(major, *info, w)?;
                let m = encode_addnl(err_at!(FailConvert, u64::try_from(map.len()))?, w)?;
                let mut acc = 0;
                for (key, val) in map.iter() {
                    let key = key.clone().into_cbor()?;
                    acc += key.do_encode(w, depth + 1)?;
                    acc += val.do_encode(w, depth + 1)?;
                }
                n + m + acc
            }
            Cbor::Major6(info, tag) => {
                let n = encode_hdr(major, *info, w)?;
                let m = Tag::encode(tag, w)?;
                n + m
            }
            Cbor::Major7(info, sval) => {
                let n = encode_hdr(major, *info, w)?;
                let m = SimpleValue::encode(sval, w)?;
                n + m
            }
            Cbor::Binary(data) => {
                write_w!(w, data);
                data.len()
            }
        };

        Ok(n)
    }

    /// Deserialize bytes from reader `r` to Cbor value, return the cbor value
    /// and number of bytes read to construct the value.
    pub fn decode<R>(r: &mut R) -> Result<(Cbor, usize)>
    where
        R: io::Read,
    {
        Cbor::do_decode(r, 1)
    }

    fn do_decode<R>(reader: &mut R, depth: u32) -> Result<(Cbor, usize)>
    where
        R: io::Read,
    {
        if depth > RECURSION_LIMIT {
            return err_at!(FailCbor, msg: "decode recursion limt exceeded");
        }

        let (major, info, n) = decode_hdr(reader)?;

        let (val, m) = match (major, info) {
            (0, info) => {
                let (val, m) = decode_addnl(info, reader)?;
                (Cbor::Major0(info, val), m)
            }
            (1, info) => {
                let (val, m) = decode_addnl(info, reader)?;
                (Cbor::Major1(info, val), m)
            }
            (2, Info::Indefinite) => {
                let mut data: Vec<u8> = Vec::default();
                let mut m = 0_usize;
                loop {
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    match val {
                        Cbor::Major2(_, chunk) => data.extend_from_slice(&chunk),
                        Cbor::Major7(_, SimpleValue::Break) => break,
                        _ => err_at!(FailConvert, msg: "expected byte chunk")?,
                    }
                    m += k;
                }
                (Cbor::Major2(info, data), m)
            }
            (2, info) => {
                let (val, m) = decode_addnl(info, reader)?;
                let len: usize = err_at!(FailConvert, val.try_into())?;
                let mut data = vec![0; len];
                read_r!(reader, &mut data);
                (Cbor::Major2(info, data), m + len)
            }
            (3, Info::Indefinite) => {
                let mut text: Vec<u8> = Vec::default();
                let mut m = 0_usize;
                loop {
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    match val {
                        Cbor::Major3(_, chunk) => text.extend_from_slice(&chunk),
                        Cbor::Major7(_, SimpleValue::Break) => break,
                        _ => err_at!(FailConvert, msg: "expected byte chunk")?,
                    }
                    m += k;
                }
                (Cbor::Major3(info, text), m)
            }
            (3, info) => {
                let (val, m) = decode_addnl(info, reader)?;
                let len: usize = err_at!(FailConvert, val.try_into())?;
                let mut text = vec![0; len];
                read_r!(reader, &mut text);
                (Cbor::Major3(info, text), m + len)
            }
            (4, Info::Indefinite) => {
                let mut list: Vec<Cbor> = vec![];
                let mut m = 0_usize;
                loop {
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    match val {
                        Cbor::Major7(_, SimpleValue::Break) => break,
                        item => list.push(item),
                    }
                    m += k;
                }
                (Cbor::Major4(info, list), m)
            }
            (4, info) => {
                let mut list: Vec<Cbor> = vec![];
                let (len, mut m) = decode_addnl(info, reader)?;
                for _ in 0..len {
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    list.push(val);
                    m += k;
                }
                (Cbor::Major4(info, list), m)
            }
            (5, Info::Indefinite) => {
                let mut map: Vec<(Key, Cbor)> = Vec::default();
                let mut m = 0_usize;
                loop {
                    let (key, j) = Cbor::do_decode(reader, depth + 1)?;
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    let val = match val {
                        Cbor::Major7(_, SimpleValue::Break) => break,
                        val => val,
                    };
                    map.push((Key::from_cbor(key)?, val));
                    m += j + k;
                }
                (Cbor::Major5(info, map), m)
            }
            (5, info) => {
                let mut map: Vec<(Key, Cbor)> = Vec::default();
                let (len, mut m) = decode_addnl(info, reader)?;
                for _ in 0..len {
                    let (key, j) = Cbor::do_decode(reader, depth + 1)?;
                    let (val, k) = Cbor::do_decode(reader, depth + 1)?;
                    map.push((Key::from_cbor(key)?, val));
                    m += j + k;
                }
                (Cbor::Major5(info, map), m)
            }
            (6, info) => {
                let (tag, m) = Tag::decode(info, reader)?;
                (Cbor::Major6(info, tag), m)
            }
            (7, info) => {
                let (sval, m) = SimpleValue::decode(info, reader)?;
                (Cbor::Major7(info, sval), m)
            }
            _ => unreachable!(),
        };

        Ok((val, (m + n)))
    }

    fn to_major_val(&self) -> u8 {
        match self {
            Cbor::Major0(_, _) => 0,
            Cbor::Major1(_, _) => 1,
            Cbor::Major2(_, _) => 2,
            Cbor::Major3(_, _) => 3,
            Cbor::Major4(_, _) => 4,
            Cbor::Major5(_, _) => 5,
            Cbor::Major6(_, _) => 6,
            Cbor::Major7(_, _) => 7,
            Cbor::Binary(data) => (data[0] & 0xe0) >> 5,
        }
    }

    /// Convert bytes into Cbor major type-2 value. There is an ambiguity
    /// in how we should treat `Vec<u8>` type. On one hand it can be treated
    /// as Cbor bytes (Major type-2) and on the other hand it can be treated
    /// as list of bytes (Major type-4). Since this ambiguity is best resolved
    /// at the application side, we are exposing this API to convert `Vec<u8>`
    /// into Cbor Major type-2, while using the [IntoCbor] trait shall convert
    /// it into Cbor Major type-4, a list of integer.
    pub fn bytes_into_cbor(val: Vec<u8>) -> Result<Self> {
        let n = err_at!(FailConvert, u64::try_from(val.len()))?;
        Ok(Cbor::Major2(n.into(), val))
    }

    /// This is converse of [Cbor::bytes_into_cbor].
    pub fn into_bytes(self) -> Result<Vec<u8>> {
        match self {
            Cbor::Major2(_, val) => Ok(val),
            _ => err_at!(FailConvert, msg: "not bytes"),
        }
    }
}

/// 5-bit value for additional info. Refer to Cbor [spec] for details.
///
/// [spec]: https://tools.ietf.org/html/rfc7049
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Info {
    /// additional info is in-lined.
    Tiny(u8), // 0..=23
    /// additional info of 8-bit unsigned integer.
    U8,
    /// additional info of 16-bit unsigned integer.
    U16,
    /// additional info of 32-bit unsigned integer.
    U32,
    /// additional info of 64-bit unsigned integer.
    U64,
    /// Reserved.
    Reserved28,
    /// Reserved.
    Reserved29,
    /// Reserved.
    Reserved30,
    /// Indefinite encoding.
    Indefinite,
}

#[cfg(any(feature = "arbitrary", test))]
impl<'a> Arbitrary<'a> for Info {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let tn = u.arbitrary::<u8>()? % 24;
        Ok(*u.choose(&[
            Info::Tiny(tn),
            Info::U8,
            Info::U16,
            Info::U32,
            Info::U64,
            Info::Indefinite,
        ])?)
    }
}

impl TryFrom<u8> for Info {
    type Error = Error;

    fn try_from(b: u8) -> Result<Info> {
        let val = match b {
            0..=23 => Info::Tiny(b),
            24 => Info::U8,
            25 => Info::U16,
            26 => Info::U32,
            27 => Info::U64,
            28 => Info::Reserved28,
            29 => Info::Reserved29,
            30 => Info::Reserved30,
            31 => Info::Indefinite,
            _ => err_at!(Fatal, msg: "unreachable")?,
        };

        Ok(val)
    }
}

impl From<u64> for Info {
    fn from(num: u64) -> Info {
        match num {
            0..=23 => Info::Tiny(num as u8),
            n if n <= (u8::MAX as u64) => Info::U8,
            n if n <= (u16::MAX as u64) => Info::U16,
            n if n <= (u32::MAX as u64) => Info::U32,
            _ => Info::U64,
        }
    }
}

impl TryFrom<usize> for Info {
    type Error = Error;

    fn try_from(num: usize) -> Result<Info> {
        Ok(err_at!(FailConvert, u64::try_from(num))?.into())
    }
}

impl Info {
    fn pretty_print(&self) -> Result<String> {
        let s = match self {
            Info::Tiny(val) => format!("Tiny(0x{:x})", val),
            Info::U8 => "U8".to_string(),
            Info::U16 => "U16".to_string(),
            Info::U32 => "U32".to_string(),
            Info::U64 => "U64".to_string(),
            Info::Reserved28 => "Reserved28".to_string(),
            Info::Reserved29 => "Reserved29".to_string(),
            Info::Reserved30 => "Reserved30".to_string(),
            Info::Indefinite => "Indefinite".to_string(),
        };

        Ok(s)
    }
}

fn encode_hdr<W>(major: u8, info: Info, w: &mut W) -> Result<usize>
where
    W: io::Write,
{
    let info = match info {
        Info::Tiny(val) if val <= 23 => val,
        Info::Tiny(val) => err_at!(FailCbor, msg: "{} > 23", val)?,
        Info::U8 => 24,
        Info::U16 => 25,
        Info::U32 => 26,
        Info::U64 => 27,
        Info::Reserved28 => 28,
        Info::Reserved29 => 29,
        Info::Reserved30 => 30,
        Info::Indefinite => 31,
    };
    write_w!(w, &[(major as u8) << 5 | info]);
    Ok(1)
}

fn decode_hdr<R>(r: &mut R) -> Result<(u8, Info, usize)>
where
    R: io::Read,
{
    let mut scratch = [0_u8; 8];
    read_r!(r, &mut scratch[..1]);

    let b = scratch[0];

    let major = (b & 0xe0) >> 5;
    let info = b & 0x1f;
    Ok((major, info.try_into()?, 1 /* only 1-byte read */))
}

fn encode_addnl<W>(num: u64, w: &mut W) -> Result<usize>
where
    W: io::Write,
{
    let mut scratch = [0_u8; 8];
    let n = match num {
        0..=23 => 0,
        n if n <= (u8::MAX as u64) => {
            scratch[..1].copy_from_slice(&(n as u8).to_be_bytes());
            1
        }
        n if n <= (u16::MAX as u64) => {
            scratch[..2].copy_from_slice(&(n as u16).to_be_bytes());
            2
        }
        n if n <= (u32::MAX as u64) => {
            scratch[..4].copy_from_slice(&(n as u32).to_be_bytes());
            4
        }
        n => {
            scratch[..8].copy_from_slice(&n.to_be_bytes());
            8
        }
    };
    write_w!(w, &scratch[..n]);
    Ok(n)
}

fn decode_addnl<R>(info: Info, r: &mut R) -> Result<(u64, usize)>
where
    R: io::Read,
{
    let mut scratch = [0_u8; 8];
    let (num, n) = match info {
        Info::Tiny(num) => (num as u64, 0),
        Info::U8 => {
            read_r!(r, &mut scratch[..1]);
            (
                u8::from_be_bytes(scratch[..1].try_into().unwrap()) as u64,
                1,
            )
        }
        Info::U16 => {
            read_r!(r, &mut scratch[..2]);
            (
                u16::from_be_bytes(scratch[..2].try_into().unwrap()) as u64,
                2,
            )
        }
        Info::U32 => {
            read_r!(r, &mut scratch[..4]);
            (
                u32::from_be_bytes(scratch[..4].try_into().unwrap()) as u64,
                4,
            )
        }
        Info::U64 => {
            read_r!(r, &mut scratch[..8]);
            (
                u64::from_be_bytes(scratch[..8].try_into().unwrap()) as u64,
                8,
            )
        }
        Info::Indefinite => (0, 0),
        _ => err_at!(FailCbor, msg: "no additional value")?,
    };
    Ok((num, n))
}

/// Major type 7, simple-value. Refer to Cbor [spec] for details.
///
/// [spec]: https://tools.ietf.org/html/rfc7049
#[derive(Debug, Copy, Clone)]
pub enum SimpleValue {
    /// 0..=19 and 28..=30 and 32..=255 are unassigned.
    Unassigned,
    /// Boolean type, value true.
    True, // 20, tiny simple-value
    /// Boolean type, value false.
    False, // 21, tiny simple-value
    /// Null unitary type, can be used in place of optional types.
    Null, // 22, tiny simple-value
    /// Undefined unitary type.
    Undefined, // 23, tiny simple-value
    /// Reserved.
    Reserved24(u8), // 24, one-byte simple-value
    /// 16-bit floating point.
    F16(u16), // 25, not-implemented
    /// 32-bit floating point.
    F32(f32), // 26, single-precision float
    /// 64-bit floating point.
    F64(f64), // 27, double-precision float
    /// Break stop for indefinite encoding.
    Break, // 31
}

#[cfg(any(feature = "arbitrary", test))]
impl<'a> Arbitrary<'a> for SimpleValue {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let f4 = u.arbitrary::<f32>()?;
        let f8 = u.arbitrary::<f64>()?;

        Ok(*u.choose(&[
            SimpleValue::True,
            SimpleValue::False,
            SimpleValue::Null,
            SimpleValue::F32(f4),
            SimpleValue::F64(f8),
        ])?)
    }
}

impl Eq for SimpleValue {}

impl PartialEq for SimpleValue {
    fn eq(&self, other: &Self) -> bool {
        use SimpleValue::*;

        match (self, other) {
            (Unassigned, Unassigned) => true,
            (True, True) => true,
            (False, False) => true,
            (Null, Null) => true,
            (Undefined, Undefined) => true,
            (Reserved24(a), Reserved24(b)) => a == b,
            (F16(a), F16(b)) => a == b,
            (F32(a), F32(b)) => a.total_cmp(b) == cmp::Ordering::Equal,
            (F64(a), F64(b)) => a.total_cmp(b) == cmp::Ordering::Equal,
            (Break, Break) => true,
            (_, _) => false,
        }
    }
}

impl SimpleValue {
    fn pretty_print(&self) -> Result<String> {
        let s = match self {
            SimpleValue::Unassigned => "Unassigned".to_string(),
            SimpleValue::True => "True".to_string(),
            SimpleValue::False => "False".to_string(),
            SimpleValue::Null => "Null".to_string(),
            SimpleValue::Undefined => "Undefined".to_string(),
            SimpleValue::Reserved24(val) => format!("Reserved24(0x{:x})", val),
            SimpleValue::F16(val) => format!("F16({})", val),
            SimpleValue::F32(val) => format!("F32({})", val),
            SimpleValue::F64(val) => format!("F64({})", val),
            SimpleValue::Break => "Break".to_string(),
        };

        Ok(s)
    }
}

impl IntoCbor for SimpleValue {
    fn into_cbor(self) -> Result<Cbor> {
        use SimpleValue::*;

        let val = match self {
            Unassigned => err_at!(FailConvert, msg: "simple-value-unassigned")?,
            val @ True => Cbor::Major7(Info::Tiny(20), val),
            val @ False => Cbor::Major7(Info::Tiny(21), val),
            val @ Null => Cbor::Major7(Info::Tiny(22), val),
            Undefined => err_at!(FailConvert, msg: "simple-value-undefined")?,
            Reserved24(_) => err_at!(FailConvert, msg: "simple-value-unassigned1")?,
            F16(_) => err_at!(FailConvert, msg: "simple-value-f16")?,
            val @ F32(_) => Cbor::Major7(Info::U32, val),
            val @ F64(_) => Cbor::Major7(Info::U64, val),
            val @ Break => Cbor::Major7(Info::Indefinite, val),
        };

        Ok(val)
    }
}

impl SimpleValue {
    pub fn to_type_order(&self) -> usize {
        use SimpleValue::*;

        match self {
            Unassigned => 4,
            True => 8,
            False => 12,
            Null => 16,
            Undefined => 20,
            Reserved24(_) => 24,
            F16(_) => 28,
            F32(_) => 32,
            F64(_) => 36,
            Break => 40,
        }
    }

    fn encode<W>(sval: &SimpleValue, w: &mut W) -> Result<usize>
    where
        W: io::Write,
    {
        use SimpleValue::*;

        let mut scratch = [0_u8; 8];
        let n = match sval {
            True | False | Null | Undefined | Break | Unassigned => 0,
            Reserved24(num) => {
                scratch[0] = *num;
                1
            }
            F16(f) => {
                scratch[0..2].copy_from_slice(&f.to_be_bytes());
                2
            }
            F32(f) => {
                scratch[0..4].copy_from_slice(&f.to_be_bytes());
                4
            }
            F64(f) => {
                scratch[0..8].copy_from_slice(&f.to_be_bytes());
                8
            }
        };
        write_w!(w, &scratch[..n]);
        Ok(n)
    }

    fn decode<R>(info: Info, r: &mut R) -> Result<(SimpleValue, usize)>
    where
        R: io::Read,
    {
        let mut scratch = [0_u8; 8];
        let (val, n) = match info {
            Info::Tiny(20) => (SimpleValue::True, 0),
            Info::Tiny(21) => (SimpleValue::False, 0),
            Info::Tiny(22) => (SimpleValue::Null, 0),
            Info::Tiny(23) => err_at!(FailCbor, msg: "simple-value-undefined")?,
            Info::Tiny(_) => err_at!(FailCbor, msg: "simple-value-unassigned")?,
            Info::U8 => err_at!(FailCbor, msg: "simple-value-unassigned1")?,
            Info::U16 => err_at!(FailCbor, msg: "simple-value-f16")?,
            Info::U32 => {
                read_r!(r, &mut scratch[..4]);
                let val = f32::from_be_bytes(scratch[..4].try_into().unwrap());
                (SimpleValue::F32(val), 4)
            }
            Info::U64 => {
                read_r!(r, &mut scratch[..8]);
                let val = f64::from_be_bytes(scratch[..8].try_into().unwrap());
                (SimpleValue::F64(val), 8)
            }
            Info::Reserved28 => err_at!(FailCbor, msg: "simple-value-reserved")?,
            Info::Reserved29 => err_at!(FailCbor, msg: "simple-value-reserved")?,
            Info::Reserved30 => err_at!(FailCbor, msg: "simple-value-reserved")?,
            Info::Indefinite => (SimpleValue::Break, 0),
        };
        Ok((val, n))
    }
}

#[derive(Copy, Clone)]
enum TagNum {
    UBigNum = 2,
    SBigNum = 3,
    Identifier = 39,
    Any = 65535, // always invalid
}

impl From<u64> for TagNum {
    fn from(num: u64) -> TagNum {
        match num {
            2 => TagNum::UBigNum,
            3 => TagNum::SBigNum,
            39 => TagNum::Identifier,
            _ => TagNum::Any,
        }
    }
}

/// Major type 6, Tag values. Refer to Cbor [spec] for details.
///
/// [spec]: https://tools.ietf.org/html/rfc7049
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Tag {
    /// Tag 2, arbitrarily sized positive integers, byte-string in network byte order.
    UBigNum(Box<Cbor>),
    /// Tag 3, arbitrarily sized signed integers, byte-string in network byte order.
    SBigNum(Box<Cbor>),
    /// Tag 39, used as identifier marker. This implementation shall
    /// treat them as literal values. Used by `Cborize` procedural
    /// macro to match values with types.
    Identifier(Box<Cbor>),
    /// Catch all tag-value, follows the generic Tag specification
    /// for Cbor.
    Value(u64),
}

#[cfg(any(feature = "arbitrary", test))]
impl<'a> Arbitrary<'a> for Tag {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let tag = *u
            .choose(&[
                TagNum::UBigNum,
                TagNum::SBigNum,
                TagNum::Identifier,
                TagNum::Any,
            ])
            .unwrap();
        match tag {
            TagNum::UBigNum | TagNum::SBigNum => {
                let val: BigInt = u.arbitrary()?;
                let (sign, bytes) = val.to_bytes_be();
                let val = Box::new(Cbor::bytes_into_cbor(bytes).unwrap());
                match sign {
                    Sign::Plus | Sign::NoSign => Ok(Tag::UBigNum(val)),
                    Sign::Minus => Ok(Tag::SBigNum(val)),
                }
            }
            TagNum::Identifier => {
                let val: Cbor = u.arbitrary()?;
                Ok(Tag::Identifier(Box::new(val)))
            }
            TagNum::Any => {
                let num: u64 = u.arbitrary()?;
                Ok(Tag::Value(num))
            }
        }
    }
}

impl From<Tag> for Cbor {
    fn from(tag: Tag) -> Cbor {
        let num = tag.to_tag_value();
        Cbor::Major6(num.into(), tag)
    }
}

impl Tag {
    /// Construct a Tag value from u64 type.
    pub fn from_value(value: u64) -> Tag {
        Tag::Value(value)
    }

    /// Wrap value with Identifier tag.
    pub fn from_identifier(value: Cbor) -> Tag {
        Tag::Identifier(Box::new(value))
    }

    /// Fetch the u64 type value for tag.
    pub fn to_tag_value(&self) -> u64 {
        match self {
            Tag::UBigNum(_) => TagNum::UBigNum as u64,
            Tag::SBigNum(_) => TagNum::SBigNum as u64,
            Tag::Identifier(_) => TagNum::Identifier as u64,
            Tag::Value(val) => *val,
        }
    }

    fn encode<W>(tag: &Tag, w: &mut W) -> Result<usize>
    where
        W: io::Write,
    {
        let num = tag.to_tag_value();
        let mut n = encode_addnl(num, w)?;
        n += match tag {
            Tag::UBigNum(val) => val.encode(w)?,
            Tag::SBigNum(val) => val.encode(w)?,
            Tag::Identifier(val) => val.encode(w)?,
            Tag::Value(_) => 0,
        };

        Ok(n)
    }

    fn decode<R>(info: Info, r: &mut R) -> Result<(Tag, usize)>
    where
        R: io::Read,
    {
        let (tag, n) = decode_addnl(info, r)?;
        let (tag, m) = match TagNum::from(tag) {
            TagNum::UBigNum => {
                let (val, m) = Cbor::decode(r)?;
                (Tag::UBigNum(Box::new(val)), m)
            }
            TagNum::SBigNum => {
                let (val, m) = Cbor::decode(r)?;
                (Tag::SBigNum(Box::new(val)), m)
            }
            TagNum::Identifier => {
                let (val, m) = Cbor::decode(r)?;
                (Tag::Identifier(Box::new(val)), m)
            }
            _ => (Tag::Value(tag as u64), 0),
        };
        Ok((tag, m + n))
    }

    fn pretty_print(&self, p: &str) -> Result<String> {
        let s = match self {
            Tag::UBigNum(val) => {
                let val = BigInt::from_bytes_be(Sign::Plus, &val.clone().into_bytes()?);
                format!("Tag::UBigNum(0x{:x})", val)
            }
            Tag::SBigNum(val) => {
                let val = BigInt::from_bytes_be(Sign::Minus, &val.clone().into_bytes()?);
                format!("Tag::SBigNum(0x{:x})", val)
            }
            Tag::Identifier(val) => {
                let mut ss = vec!["Tag::Identifier".to_string()];
                let p = p.to_owned() + "  ";
                ss.push(val.pretty_print(&p)?);
                ss.join("\n")
            }
            Tag::Value(val) => format!("Tag::Value(0x{:x})", val),
        };

        Ok(s)
    }
}

/// Possible types that can be used as a key in cbor-map.
#[derive(Debug, Clone)]
pub enum Key {
    Bool(bool),
    N64(i64),
    U64(u64),
    F32(f32),
    F64(f64),
    Bytes(Vec<u8>),
    Text(String),
}

#[cfg(any(feature = "arbitrary", test))]
impl<'a> Arbitrary<'a> for Key {
    fn arbitrary(u: &mut Unstructured<'a>) -> arbitrary::Result<Self> {
        let bl = Key::Bool(u.arbitrary::<bool>()?);
        let nn = Key::N64(-u.arbitrary::<i64>()?.abs());
        let pn = Key::U64(u.arbitrary::<u64>()?);
        let f4 = Key::F32(u.arbitrary::<f32>()?);
        let f8 = Key::F64(u.arbitrary::<f64>()?);
        let bs = Key::Bytes(u.arbitrary::<Vec<u8>>()?);
        let sr = Key::Text(u.arbitrary::<String>()?);

        Ok(u.choose(&[bl, nn, pn, f4, f8, bs, sr])?.clone())
    }
}

impl Key {
    /// As per cbor [spec], map's key can be a heterogeneous collection of types.
    /// That is, some of the keys can be Boolean, other can be numbers etc ..
    ///
    /// This function defines the ordering for supported key types. As,
    /// * Key::Bool, sort before every other keys.
    /// * Key::N64, sort after boolean type.
    /// * Key::U64, sort after negative integers.
    /// * Key::F32, sort after positive integers.
    /// * Key::F64, sort after 32-bit floating point numbers.
    /// * Key::Bytes, sort after 64-bit floating point numbers.
    /// * Key::Text, sort after bytes.
    ///
    /// [spec]: https://tools.ietf.org/html/rfc7049
    pub fn to_type_order(&self) -> usize {
        use Key::*;

        match self {
            Bool(_) => 4,
            N64(_) => 8,
            U64(_) => 8,
            F32(_) => 12,
            F64(_) => 16,
            Bytes(_) => 20,
            Text(_) => 24,
        }
    }

    fn pretty_print(&self) -> Result<String> {
        let s = match self {
            Key::Bool(val) => format!("Key(B:{})", val),
            Key::N64(val) => format!("Key(N:0x{:x})", val),
            Key::U64(val) => format!("Key(P:0x{:x})", val),
            Key::F32(val) => format!("Key(F:{})", val),
            Key::F64(val) => format!("Key(D:{})", val),
            Key::Bytes(val) => format!("Key(B:{:?})", val),
            Key::Text(val) => format!("Key(T:{:?})", val),
        };

        Ok(s)
    }
}

impl Eq for Key {}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        use Key::*;

        match (self, other) {
            (Bool(a), Bool(b)) => a == b,
            (N64(a), N64(b)) => a == b,
            (U64(a), U64(b)) => a == b,
            (F32(a), F32(b)) => a.total_cmp(b) == cmp::Ordering::Equal,
            (F64(a), F64(b)) => a.total_cmp(b) == cmp::Ordering::Equal,
            (Bytes(a), Bytes(b)) => a == b,
            (Text(a), Text(b)) => a == b,
            (_, _) => false,
        }
    }
}

impl Ord for Key {
    fn cmp(&self, other: &Key) -> cmp::Ordering {
        use Key::*;

        let (a, b) = (self.to_type_order(), other.to_type_order());
        if a == b {
            match (self, other) {
                (Bool(a), Bool(b)) => a.cmp(b),
                (N64(a), N64(b)) => a.cmp(b),
                (U64(a), U64(b)) => a.cmp(b),
                (N64(_), U64(_)) => cmp::Ordering::Less,
                (U64(_), N64(_)) => cmp::Ordering::Greater,
                (Bytes(a), Bytes(b)) => a.cmp(b),
                (Text(a), Text(b)) => a.cmp(b),
                (F32(a), F32(b)) => a.total_cmp(b),
                (F64(a), F64(b)) => a.total_cmp(b),
                (_, _) => unreachable!(),
            }
        } else {
            a.cmp(&b)
        }
    }
}

impl PartialOrd for Key {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Return pretty formated string representing `val` that can be printed on
/// terminal, log-file for eye-ball verification.
pub fn pretty_print(val: &Cbor) -> Result<String> {
    val.pretty_print("")
}

#[cfg(test)]
#[path = "cbor_test.rs"]
mod cbor_test;
