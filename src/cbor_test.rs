use arbitrary::Unstructured;
use rand::{prelude::random, rngs::SmallRng, Rng, SeedableRng};

use super::*;

#[test]
fn test_simple_value() {
    use SimpleValue::*;

    let seed: u128 = random();
    println!("test_simple_value seed:{}", seed);
    let mut rng = {
        let mut rng_seed = [0; 32];
        rng_seed[0..16].copy_from_slice(&seed.to_le_bytes());
        SmallRng::from_seed(rng_seed)
    };

    for _ in 0..100 {
        let sval: SimpleValue = {
            let bytes = rng.gen::<[u8; 32]>();
            let mut uns = Unstructured::new(&bytes);
            uns.arbitrary().unwrap()
        };

        match (sval.to_type_order(), &sval) {
            (4, Unassigned)
            | (8, True)
            | (12, False)
            | (16, Null)
            | (20, Undefined)
            | (24, Reserved24(_))
            | (28, F16(_))
            | (32, F32(_))
            | (36, F64(_))
            | (40, Break) => (),
            (order, sval) => panic!("{} {:?}", order, sval),
        }

        let val: Cbor = match (&sval, sval.into_cbor()) {
            (Unassigned, Err(_)) => continue,
            (Undefined, Err(_)) => continue,
            (Reserved24(_), Err(_)) => continue,
            (F16(_), Err(_)) => continue,
            (Break, Err(_)) => continue,
            (_, val) => val.unwrap(),
        };

        let mut buf: Vec<u8> = vec![];
        let n = val.encode(&mut buf).unwrap();
        let (nval, m) = Cbor::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(n, m);
        assert_eq!(val, nval);
    }
}

#[test]
fn test_cbor() {
    let seed: u128 = random();
    // let seed: u128 = 106952773668701652133737084585647538146;
    println!("test_cbor seed:{}", seed);
    let mut rng = {
        let mut rng_seed = [0; 32];
        rng_seed[0..16].copy_from_slice(&seed.to_le_bytes());
        SmallRng::from_seed(rng_seed)
    };

    for _i in 0..10000 {
        let val: Cbor = {
            let bytes: Vec<u8> = (0..100)
                .map(|_| rng.gen::<[u8; 32]>().to_vec())
                .flatten()
                .collect();
            let mut uns = Unstructured::new(&bytes);
            uns.arbitrary().unwrap()
        };

        // println!("test_cbor val:{:?}", val);
        let mut buf: Vec<u8> = vec![];
        let n = val.encode(&mut buf).unwrap();
        let (nval, m) = Cbor::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(n, m);
        assert_eq!(val, nval);
    }
}
