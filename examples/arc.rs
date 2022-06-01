//! Using cbordata macros

use std::sync::Arc;

extern crate cbordata;
use cbordata::{Cborize, FromCbor, IntoCbor};

fn main() {
    println!("test_example_arc");

    #[derive(Debug, Clone, Eq, PartialEq, Cborize)]
    struct MyType {
        name: String,
        a: u32,
    }

    impl MyType {
        const ID: u32 = 0;
    }

    let val = Arc::new(MyType { name: "hello world".to_string(), a: 0 });

    let cbor_val = val.clone().into_cbor().unwrap();
    let ret_val = Arc::<MyType>::from_cbor(cbor_val).unwrap();

    assert_eq!(val, ret_val, "{:?}\n{:?}", val, ret_val);
}
