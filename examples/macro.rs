//! Using cbordata macros

use cbordata::{Cbor, Cborize, FromCbor, IntoCbor};

#[derive(Cborize, Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[allow(non_snake_case)]
struct Parent {
    field1: u8,
    field2: i8,
    field3: u16,
    field4: i16,
    field5: u32,
    field6: i32,
    field7: u64,
    field8: i64,
    field11: bool,
    field12: usize,
    field13: isize,
    field14: String,
    field15: Vec<u8>,
}

impl Parent {
    const ID: &'static str = "floats";
}

#[derive(Cborize, Default, Clone, Debug)]
#[allow(non_snake_case)]
struct Floats {
    field1: f32,
    field2: f64,
}

impl Floats {
    const ID: &'static str = "floats";
}

fn main() {
    let p_ref = Parent {
        field1: 10,
        field2: -10,
        field3: 100,
        field4: -100,
        field5: 1000,
        field6: -1000,
        field7: 10000,
        field8: -10000,
        field11: true,
        field12: 100,
        field13: 102,
        field14: "hello world".to_string(),
        field15: vec![1, 2, 3, 4],
    };

    let val: Cbor = p_ref.clone().into_cbor().unwrap();
    let p: Parent = Parent::from_cbor(val.clone()).unwrap();
    println!("{:?}", p);
    println!("{:?}", p_ref);
    assert_eq!(p_ref, p);
}
