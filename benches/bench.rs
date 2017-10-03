#![feature(test)]

extern crate kyocode;
extern crate test;

use test::Bencher;
use kyocode::*;

#[bench]
fn bench_encode(b: &mut Bencher) {
    let data = std::f64::consts::PI.to_string().repeat(10);
    b.iter(|| {
        encode(test::black_box(data.as_bytes()))
    })
}

#[bench]
fn bench_decode(b: &mut Bencher) {
    let data = std::f64::consts::PI.to_string().repeat(10);
    let code = encode(data.as_bytes());
    b.iter(|| {
        decode(test::black_box(&code))
    })
}
