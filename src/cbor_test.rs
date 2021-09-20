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
    // let seed: u128 = 193689849637978864038196716223816071251;
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

        let mut buf: Vec<u8> = vec![];
        let n = val.encode(&mut buf).unwrap();
        let (nval, m) = Cbor::decode(&mut buf.as_slice()).unwrap();
        assert_eq!(n, m);
        assert_eq!(val, nval);
    }
}

#[test]
fn test_bigint() {
    let seed: u128 = random();
    // let seed: u128 = 193689849637978864038196716223816071251;
    println!("test_bigint seed:{}", seed);
    let mut rng = {
        let mut rng_seed = [0; 32];
        rng_seed[0..16].copy_from_slice(&seed.to_le_bytes());
        SmallRng::from_seed(rng_seed)
    };

    for _i in 0..10000 {
        let vals = {
            let bytes: Vec<u8> = (0..100)
                .map(|_| rng.gen::<[u8; 32]>().to_vec())
                .flatten()
                .collect();
            let mut uns = Unstructured::new(&bytes);
            let a = uns.arbitrary::<u128>().unwrap();
            let b = uns.arbitrary::<i128>().unwrap();
            let c = uns.arbitrary::<BigInt>().unwrap();
            let vals = vec![
                a.clone().into_cbor().unwrap(),
                b.clone().into_cbor().unwrap(),
                c.clone().into_cbor().unwrap(),
            ];
            assert_eq!(u128::from_cbor(vals[0].clone()).unwrap(), a);
            assert_eq!(i128::from_cbor(vals[1].clone()).unwrap(), b);
            assert_eq!(BigInt::from_cbor(vals[2].clone()).unwrap(), c);
            vals
        };

        for val in vals.into_iter() {
            let mut buf: Vec<u8> = vec![];
            let n = val.encode(&mut buf).unwrap();
            let (nval, m) = Cbor::decode(&mut buf.as_slice()).unwrap();
            assert_eq!(n, m);
            assert_eq!(val, nval);
        }
    }
}
