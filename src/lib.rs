//! Module implement simple and easy CBOR serialization.
//!
//! _Why use custom cbor implementation while there are off-the-self solutions ?_
//!
//! Because [CBOR][cbor] specification itself is open-ended, and custom
//! implementation means, we can mold it to the needs of distributed apps.
//! This implementation is also tuned for big-data and document databases.
//!
//! Features
//! ========
//!
//! **`arbitrary`** feature must be enabled, for [Cbor] and [Key[ types to implement
//! the `arbitrary::Arbitrary` trait.
//!
//! [cbor]: https://tools.ietf.org/html/rfc7049

#![feature(total_cmp)]

#[cfg(any(feature = "arbitrary", test))]
extern crate arbitrary;
extern crate cbordata_derive;
extern crate num_bigint;
extern crate num_traits;
#[cfg(test)]
extern crate rand;

use std::{error, fmt, result};

/// Short form to compose Error values.
///
/// Here are few possible ways:
///
/// ```ignore
/// use crate::Error;
/// err_at!(ParseError, msg: format!("bad argument"));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(ParseError, std::io::read(buf));
/// ```
///
/// ```ignore
/// use crate::Error;
/// err_at!(ParseError, std::fs::read(file_path), format!("read failed"));
/// ```
///
#[macro_export]
macro_rules! err_at {
    ($v:ident, msg: $($arg:expr),+) => {{
        let prefix = format!("{}:{}", file!(), line!());
        Err(Error::$v(prefix, format!($($arg),+)))
    }};
    ($v:ident, $e:expr) => {{
        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                Err(Error::$v(prefix, format!("{}", err)))
            }
        }
    }};
    ($v:ident, $e:expr, $($arg:expr),+) => {{
        match $e {
            Ok(val) => Ok(val),
            Err(err) => {
                let prefix = format!("{}:{}", file!(), line!());
                let msg = format!($($arg),+);
                Err(Error::$v(prefix, format!("{} {}", err, msg)))
            }
        }
    }};
}

/// Error variants that can be returned by this package's API.
///
/// Each variant carries a prefix, typically identifying the
/// error location.
pub enum Error {
    Fatal(String, String),
    FailConvert(String, String),
    IOError(String, String),
    FailCbor(String, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        use Error::*;

        match self {
            Fatal(p, msg) => write!(f, "{} Fatal: {}", p, msg),
            FailConvert(p, msg) => write!(f, "{} FailConvert: {}", p, msg),
            IOError(p, msg) => write!(f, "{} IOError: {}", p, msg),
            FailCbor(p, msg) => write!(f, "{} FailCbor: {}", p, msg),
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> result::Result<(), fmt::Error> {
        write!(f, "{}", self)
    }
}

impl error::Error for Error {}

#[doc(hidden)]
pub use cbordata_derive::*;

mod cbor;
mod types;

pub use cbor::{pretty_print, Cbor, Info, Key, SimpleValue, Tag, RECURSION_LIMIT};

/// Convert rust-native value to [Cbor], which can then be encoded into bytes
/// using Cbor.
///
/// Refer to [FromCbor] the reverse transformation of a type to [Cbor] value.
pub trait IntoCbor {
    /// Convert implementing type's value into [Cbor].
    fn into_cbor(self) -> Result<Cbor>;
}

/// Convert from Cbor, the cbor value is typically obtained by
/// decoding it from bytes.
///
/// Refer to [IntoCbor] the reverse transformation of [Cbor] value into type's value.
pub trait FromCbor: Sized {
    /// Convert value from [Cbor] into type's value.
    fn from_cbor(val: Cbor) -> Result<Self>;
}

/// Result type, for jsondata functions and methods, that require a
/// success or failure variant.
pub type Result<T> = std::result::Result<T, Error>;
