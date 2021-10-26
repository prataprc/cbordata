// Implement IntoCbor and FromCbor for standard types and types defined in this package.

use num_bigint::{BigInt, Sign};

#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
use std::{
    convert::{TryFrom, TryInto},
    ffi,
    sync::Arc,
};

use crate::{Cbor, Error, FromCbor, IntoCbor, Key, Result, SimpleValue, Tag};

impl<T, const N: usize> IntoCbor for [T; N]
where
    T: Clone + IntoCbor,
{
    fn into_cbor(self) -> Result<Cbor> {
        let info = err_at!(FailConvert, u64::try_from(self.len()))?.into();
        let mut val: Vec<Cbor> = vec![];
        for item in self.iter() {
            val.push(item.clone().into_cbor()?)
        }
        Ok(Cbor::Major4(info, val))
    }
}

impl<T, const N: usize> FromCbor for [T; N]
where
    T: Copy + Default + FromCbor,
{
    fn from_cbor(val: Cbor) -> Result<[T; N]> {
        let mut arr = [T::default(); N];
        let n = arr.len();
        match val {
            Cbor::Major4(_, data) if n == data.len() => {
                for (i, item) in data.into_iter().enumerate() {
                    arr[i] = T::from_cbor(item)?;
                }
                Ok(arr)
            }
            Cbor::Major4(_, data) => {
                err_at!(FailConvert, msg: "different array arity {} {}", n, data.len())
            }
            _ => err_at!(FailCbor, msg: "not an list"),
        }
    }
}

impl IntoCbor for bool {
    fn into_cbor(self) -> Result<Cbor> {
        match self {
            true => SimpleValue::True.into_cbor(),
            false => SimpleValue::False.into_cbor(),
        }
    }
}

impl FromCbor for bool {
    fn from_cbor(val: Cbor) -> Result<bool> {
        match val {
            Cbor::Major7(_, SimpleValue::True) => Ok(true),
            Cbor::Major7(_, SimpleValue::False) => Ok(false),
            _ => err_at!(FailConvert, msg: "not a bool"),
        }
    }
}

impl IntoCbor for f32 {
    fn into_cbor(self) -> Result<Cbor> {
        SimpleValue::F32(self).into_cbor()
    }
}

impl FromCbor for f32 {
    fn from_cbor(val: Cbor) -> Result<f32> {
        match val {
            Cbor::Major7(_, SimpleValue::F32(val)) => Ok(val),
            _ => err_at!(FailConvert, msg: "not f32"),
        }
    }
}

impl IntoCbor for f64 {
    fn into_cbor(self) -> Result<Cbor> {
        SimpleValue::F64(self).into_cbor()
    }
}

impl FromCbor for f64 {
    fn from_cbor(val: Cbor) -> Result<f64> {
        match val {
            Cbor::Major7(_, SimpleValue::F64(val)) => Ok(val),
            _ => err_at!(FailConvert, msg: "not f64"),
        }
    }
}
macro_rules! convert_neg_num {
    ($($t:ty)*) => {$(
        impl IntoCbor for $t {
            fn into_cbor(self) -> Result<Cbor> {
                let val: i64 = err_at!(FailConvert, i64::try_from(self))?;
                if val >= 0 {
                    Ok(err_at!(FailConvert, u64::try_from(val))?.into_cbor()?)
                } else {
                    let val = err_at!(FailConvert, u64::try_from(val.abs() - 1))?;
                    let info = val.into();
                    Ok(Cbor::Major1(info, val))
                }
            }
        }

        impl FromCbor for $t {
            fn from_cbor(val: Cbor) -> Result<$t> {
                use std::result;

                let val = match val {
                    Cbor::Major0(_, val) => {
                        let val: result::Result<$t, _> = val.try_into();
                        err_at!(FailConvert, val)?
                    }
                    Cbor::Major1(_, val) => {
                        let val: result::Result<$t, _> = (val + 1).try_into();
                        -err_at!(FailConvert, val)?
                    }
                    _ => err_at!(FailConvert, msg: "not a number")?,
                };
                Ok(val)
            }
        }
    )*}
}

convert_neg_num! {i64 i32 i16 i8 isize}

macro_rules! convert_pos_num {
    ($($t:ty)*) => {$(
        impl IntoCbor for $t {
            fn into_cbor(self) -> Result<Cbor> {
                let val = err_at!(FailConvert, u64::try_from(self))?;
                Ok(Cbor::Major0(val.into(), val))
            }
        }

        impl FromCbor for $t {
            fn from_cbor(val: Cbor) -> Result<$t> {
                match val {
                    Cbor::Major0(_, val) => Ok(err_at!(FailConvert, val.try_into())?),
                    _ => err_at!(FailConvert, msg: "not a number"),
                }
            }
        }
    )*}
}

convert_pos_num! {u64 u32 u16 u8 usize}

impl IntoCbor for u128 {
    fn into_cbor(self) -> Result<Cbor> {
        BigInt::from(self).into_cbor()
    }
}

impl FromCbor for u128 {
    fn from_cbor(val: Cbor) -> Result<u128> {
        use num_traits::cast::ToPrimitive;

        let (sign, bytes) = match val {
            Cbor::Major6(_, tag) => match tag {
                Tag::UBigNum(val) => (Sign::Plus, val.into_bytes()?),
                Tag::SBigNum(val) => (Sign::Minus, val.into_bytes()?),
                _ => err_at!(FailConvert, msg: "cbor not a bigint")?,
            },
            _ => err_at!(FailConvert, msg: "cbor not a tag/bigint")?,
        };

        match BigInt::from_bytes_be(sign, &bytes).to_u128() {
            Some(val) => Ok(val),
            None => err_at!(FailConvert, msg: "from bigint to u128"),
        }
    }
}

impl IntoCbor for i128 {
    fn into_cbor(self) -> Result<Cbor> {
        BigInt::from(self).into_cbor()
    }
}

impl FromCbor for i128 {
    fn from_cbor(val: Cbor) -> Result<i128> {
        use num_traits::cast::ToPrimitive;

        let (sign, bytes) = match val {
            Cbor::Major6(_, tag) => match tag {
                Tag::UBigNum(val) => (Sign::Plus, val.into_bytes()?),
                Tag::SBigNum(val) => (Sign::Minus, val.into_bytes()?),
                _ => err_at!(FailConvert, msg: "cbor not a bigint")?,
            },
            _ => err_at!(FailConvert, msg: "cbor not a bigint")?,
        };

        match BigInt::from_bytes_be(sign, &bytes).to_i128() {
            Some(val) => Ok(val),
            None => err_at!(FailConvert, msg: "from bigint to i128"),
        }
    }
}

impl IntoCbor for BigInt {
    fn into_cbor(self) -> Result<Cbor> {
        match self.to_bytes_be() {
            (Sign::Plus, bytes) | (Sign::NoSign, bytes) => {
                let val = Box::new(Cbor::from_bytes(bytes)?);
                Ok(Tag::UBigNum(val).into())
            }
            (Sign::Minus, bytes) => {
                let val = Box::new(Cbor::from_bytes(bytes)?);
                Ok(Tag::SBigNum(val).into())
            }
        }
    }
}

impl FromCbor for BigInt {
    fn from_cbor(val: Cbor) -> Result<BigInt> {
        let (sign, bytes) = match val {
            Cbor::Major6(_, tag) => match tag {
                Tag::UBigNum(val) => (Sign::Plus, val.into_bytes()?),
                Tag::SBigNum(val) => (Sign::Minus, val.into_bytes()?),
                _ => err_at!(FailConvert, msg: "cbor not a bigint")?,
            },
            _ => err_at!(FailConvert, msg: "cbor not a tag/bigint")?,
        };
        Ok(BigInt::from_bytes_be(sign, &bytes))
    }
}

impl<'a> IntoCbor for &'a [u8] {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major2(n.into(), self.to_vec()))
    }
}

impl<T> IntoCbor for Vec<T>
where
    T: IntoCbor,
{
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        let mut arr = vec![];
        for item in self.into_iter() {
            arr.push(item.into_cbor()?)
        }
        Ok(Cbor::Major4(n.into(), arr))
    }
}

impl<T> FromCbor for Vec<T>
where
    T: FromCbor + Sized,
{
    fn from_cbor(val: Cbor) -> Result<Vec<T>> {
        match val {
            Cbor::Major4(_, data) => {
                let mut arr = vec![];
                for item in data.into_iter() {
                    arr.push(T::from_cbor(item)?)
                }
                Ok(arr)
            }
            _ => err_at!(FailConvert, msg: "not a vector"),
        }
    }
}

impl<'a> IntoCbor for &'a str {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major3(n.into(), self.as_bytes().to_vec()))
    }
}

impl IntoCbor for String {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major3(n.into(), self.as_bytes().to_vec()))
    }
}

impl FromCbor for String {
    fn from_cbor(val: Cbor) -> Result<String> {
        use std::str::from_utf8;
        match val {
            Cbor::Major3(_, val) => {
                Ok(err_at!(FailConvert, from_utf8(&val))?.to_string())
            }
            _ => err_at!(FailConvert, msg: "not utf8-string"),
        }
    }
}

impl IntoCbor for ffi::OsString {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major3(n.into(), self.into_vec()))
    }
}

impl FromCbor for ffi::OsString {
    fn from_cbor(val: Cbor) -> Result<ffi::OsString> {
        match val {
            Cbor::Major3(_, val) => Ok(ffi::OsString::from_vec(val)),
            _ => err_at!(FailConvert, msg: "not utf8-string"),
        }
    }
}

impl IntoCbor for Vec<Cbor> {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major4(n.into(), self))
    }
}

impl FromCbor for Vec<Cbor> {
    fn from_cbor(val: Cbor) -> Result<Vec<Cbor>> {
        match val {
            Cbor::Major4(_, data) => Ok(data),
            _ => err_at!(FailConvert, msg: "not a vector"),
        }
    }
}

impl IntoCbor for Vec<(Key, Cbor)> {
    fn into_cbor(self) -> Result<Cbor> {
        let n = err_at!(FailConvert, u64::try_from(self.len()))?;
        Ok(Cbor::Major5(n.into(), self))
    }
}

impl FromCbor for Vec<(Key, Cbor)> {
    fn from_cbor(val: Cbor) -> Result<Vec<(Key, Cbor)>> {
        match val {
            Cbor::Major5(_, data) => Ok(data),
            _ => err_at!(FailConvert, msg: "not a map"),
        }
    }
}

impl<T> IntoCbor for Option<T>
where
    T: IntoCbor,
{
    fn into_cbor(self) -> Result<Cbor> {
        match self {
            Some(val) => val.into_cbor(),
            None => SimpleValue::Null.into_cbor(),
        }
    }
}

impl<T> FromCbor for Option<T>
where
    T: FromCbor + Sized,
{
    fn from_cbor(val: Cbor) -> Result<Option<T>> {
        match val {
            Cbor::Major7(_, SimpleValue::Null) => Ok(None),
            val => Ok(Some(T::from_cbor(val)?)),
        }
    }
}

impl IntoCbor for Key {
    fn into_cbor(self) -> Result<Cbor> {
        let val = match self {
            Key::U64(key) => Cbor::Major0(key.into(), key),
            Key::N64(key) if key >= 0 => {
                err_at!(FailConvert, msg: "Key::N64({}) cannot be positive", key)?
            }
            Key::N64(key) => {
                let val = err_at!(FailConvert, u64::try_from(key.abs() - 1))?;
                Cbor::Major1(val.into(), val)
            }
            Key::Bytes(key) => {
                let val = err_at!(FailConvert, key.len().try_into())?;
                Cbor::Major2(val, key)
            }
            Key::Text(key) => {
                let val = err_at!(FailConvert, key.len().try_into())?;
                Cbor::Major3(val, key.into())
            }
            Key::Bool(true) => SimpleValue::True.into_cbor()?,
            Key::Bool(false) => SimpleValue::False.into_cbor()?,
            Key::F32(key) => SimpleValue::F32(key).into_cbor()?,
            Key::F64(key) => SimpleValue::F64(key).into_cbor()?,
        };

        Ok(val)
    }
}

impl FromCbor for Key {
    fn from_cbor(val: Cbor) -> Result<Key> {
        use std::str::from_utf8;

        let key = match val {
            Cbor::Major0(_, key) => Key::U64(key),
            Cbor::Major1(_, key) => {
                let val = -err_at!(FailConvert, i64::try_from(key + 1))?;
                Key::N64(val)
            }
            Cbor::Major2(_, key) => Key::Bytes(key),
            Cbor::Major3(_, key) => {
                let val = err_at!(FailConvert, from_utf8(&key))?.to_string();
                Key::Text(val)
            }
            Cbor::Major7(_, SimpleValue::True) => Key::Bool(true),
            Cbor::Major7(_, SimpleValue::False) => Key::Bool(false),
            Cbor::Major7(_, SimpleValue::F32(key)) => Key::F32(key),
            Cbor::Major7(_, SimpleValue::F64(key)) => Key::F64(key),
            _ => err_at!(FailCbor, msg: "cbor not a valid key")?,
        };

        Ok(key)
    }
}

macro_rules! convert_key {
    ($(($t:ty, $var:ident))*) => {$(
        impl From<$t> for Key {
            fn from(val: $t) -> Key {
                Key::$var(val)
            }
        }

        impl From<Key> for $t {
            fn from(key: Key) -> $t {
                match key {
                    Key::$var(val) => val,
                    _ => panic!("not a number {:?}", key),
                }
            }
        }
    )*}
}

convert_key! {
    (bool, Bool)
    (i64, N64)
    (u64, U64)
    (f32, F32)
    (f64, F64)
    (Vec<u8>, Bytes)
    (String, Text)
}

impl<'a> From<&'a str> for Key {
    fn from(val: &'a str) -> Key {
        Key::Text(val.to_string())
    }
}

impl<T> FromCbor for Arc<T>
where
    T: FromCbor,
{
    fn from_cbor(val: Cbor) -> Result<Self> {
        T::from_cbor(val).map(Arc::new)
    }
}

impl<T> IntoCbor for Arc<T>
where
    T: IntoCbor + Clone,
{
    fn into_cbor(self) -> Result<Cbor> {
        match Arc::try_unwrap(self) {
            Ok(s) => s.into_cbor(),
            Err(s) => {
                let s: T = s.as_ref().clone();
                s.into_cbor()
            }
        }
    }
}
