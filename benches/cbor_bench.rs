#![feature(test)]
extern crate test;

use test::Bencher;

use cbordata::{Cbor, IntoCbor, Key, SimpleValue};

#[bench]
fn bench_null(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];

    b.iter(|| {
        let val: Cbor = SimpleValue::Null.into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_bool(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];

    b.iter(|| {
        let val: Cbor = true.into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_num(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];

    b.iter(|| {
        let val: Cbor = 123121.2234234.into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_string(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];
    let s = r#""汉语 / 漢語; Hàn\b \tyǔ ""#;

    b.iter(|| {
        let val: Cbor = s.into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_array(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];
    let arr = vec![
        SimpleValue::Null.into_cbor().unwrap(),
        true.into_cbor().unwrap(),
        false.into_cbor().unwrap(),
        "tru\"e".into_cbor().unwrap(),
    ];

    b.iter(|| {
        let val: Cbor = arr.clone().into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_map(b: &mut Bencher) {
    let mut n = 0;
    let mut buf: Vec<u8> = vec![];
    let map = vec![
        (Key::from("a"), SimpleValue::Null.into_cbor().unwrap()),
        (Key::from("b"), true.into_cbor().unwrap()),
        (Key::from("c"), false.into_cbor().unwrap()),
        (Key::from("d"), (-10E-1).into_cbor().unwrap()),
        (Key::from("e"), "tru\"e".into_cbor().unwrap()),
    ];
    b.iter(|| {
        let val: Cbor = map.clone().into_cbor().unwrap();
        buf.truncate(0);
        n += val.encode(&mut buf).unwrap();
    });
}

#[bench]
fn bench_null_to_cbor(b: &mut Bencher) {
    let mut buf: Vec<u8> = vec![];
    let val: Cbor = SimpleValue::Null.into_cbor().unwrap();
    val.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}

#[bench]
fn bench_bool_to_cbor(b: &mut Bencher) {
    let val: Cbor = true.into_cbor().unwrap();
    let mut buf: Vec<u8> = vec![];
    val.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}

#[bench]
fn bench_num_to_cbor(b: &mut Bencher) {
    let mut buf: Vec<u8> = vec![];
    let val: Cbor = 123121.2234234.into_cbor().unwrap();
    val.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}

#[bench]
fn bench_string_to_cbor(b: &mut Bencher) {
    let mut buf: Vec<u8> = vec![];
    let val = r#""汉语 / 漢語; Hàn\b \tyǔ ""#.into_cbor().unwrap();
    val.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}

#[bench]
fn bench_array_to_cbor(b: &mut Bencher) {
    let mut buf: Vec<u8> = vec![];
    let arr = vec![
        SimpleValue::Null.into_cbor().unwrap(),
        true.into_cbor().unwrap(),
        false.into_cbor().unwrap(),
        "tru\"e".into_cbor().unwrap(),
    ]
    .into_cbor()
    .unwrap();
    arr.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}

#[bench]
fn bench_map_to_cbor(b: &mut Bencher) {
    let mut buf: Vec<u8> = vec![];
    let map = vec![
        (Key::from("a"), SimpleValue::Null.into_cbor().unwrap()),
        (Key::from("b"), true.into_cbor().unwrap()),
        (Key::from("c"), false.into_cbor().unwrap()),
        (Key::from("d"), (-10E-1).into_cbor().unwrap()),
        (Key::from("e"), "tru\"e".into_cbor().unwrap()),
    ]
    .into_cbor()
    .unwrap();
    map.encode(&mut buf).unwrap();

    b.iter(|| Cbor::decode(&mut buf.as_slice()).unwrap());
}
