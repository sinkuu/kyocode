extern crate arrayvec;
#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate ring;
#[macro_use]
extern crate try_opt;

use ring::digest::{digest, SHA256};
use arrayvec::ArrayVec;

mod chars;
pub use chars::KYOCODE_CHARS;

fn div_roundup(lhs: usize, rhs: usize) -> usize {
    lhs / rhs + if lhs % rhs > 0 { 1 } else { 0 }
}

pub fn encode(bs: &[u8]) -> String {
    let mut res = String::with_capacity(
        (5
            + div_roundup(
                bs.len().checked_mul(8 / 2).expect("length overflow"),
                10 / 2,
            )).checked_mul(3)
            .expect("length overflow"),
    );
    // let cap = res.capacity();

    let last = bs.len().checked_mul(8).expect("length overflow") % 10;
    let padded = 0 < last && last <= 2; // padded more than a byte
    res.push(KYOCODE_CHARS[if padded { 1 } else { 0 }]);

    let hash = digest(&SHA256, bs);
    let hash = &hash.as_ref()[0..5];
    push_bytes(hash, &mut res);

    push_bytes(bs, &mut res);

    // debug_assert!(res.capacity() == cap);
    res
}

fn push_bytes(bs: &[u8], res: &mut String) {
    if bs.len() == 1 {
        res.push(KYOCODE_CHARS[(bs[0] as usize) << 2]);
    } else {
        let mut consumed = 0;
        for (i, s) in bs.windows(2).enumerate() {
            if i < bs.len() - 2 {
                if consumed == 8 {
                    consumed = 0;
                    continue;
                }

                res.push(
                    KYOCODE_CHARS[(((s[0] as usize) << (2 + consumed)) & ((1 << 10) - 1))
                                      | (s[1] >> (6 - consumed)) as usize],
                );
                consumed += 2;
            } else {
                if consumed == 8 {
                    res.push(KYOCODE_CHARS[(s[1] as usize) << 2]);
                    break;
                }

                res.push(
                    KYOCODE_CHARS[(((s[0] as usize) << (2 + consumed)) & ((1 << 10) - 1))
                                      | (s[1] >> (6 - consumed)) as usize],
                );
                consumed += 2;

                if consumed != 8 {
                    res.push(KYOCODE_CHARS[((s[1] as usize) << (2 + consumed)) & ((1 << 10) - 1)]);
                }
            }
        }
    }
}

pub fn decode(s: &str) -> Option<Vec<u8>> {
    // debug_assert!(KYOCODE_CHARS.iter().all(|c| {
    //     std::iter::once(c).collect::<String>().len() == 3
    // }));

    if s.len() < 5 * 3 {
        // No header available
        return None;
    }

    let header = &s[0..3 * 5];
    let header = header
        .chars()
        .map(|c| KYOCODE_CHARS.binary_search(&c).ok())
        .collect::<Option<ArrayVec<[usize; 5]>>>();
    let header = try_opt!(header);
    let header = header.as_slice();

    if header[0] & (((1 << 10) - 1) ^ 1) != 0 {
        return None;
    }

    let padded = header[0] & 1 == 1; // padded more than a byte

    let len = s.len() / 3 - 5;
    let mut res = Vec::with_capacity(
        try_opt!(len.checked_mul(10 / 2)) / (8 / 2) - if padded { 1 } else { 0 },
    );
    // let cap = res.capacity();

    let mut rem = 8;
    let mut acc = 0u8;
    for c in s.chars().skip(5) {
        let i = try_opt!(KYOCODE_CHARS.binary_search(&c).ok()) as u16;

        let mut nxt = 10 - rem;
        acc |= (i >> nxt) as u8;
        res.push(acc);
        if nxt > 8 {
            res.push((i >> (nxt - 8)) as u8);
            nxt -= 8;
        }
        rem = 8 - nxt;
        acc = (i << rem) as u8;
    }

    if rem == 0 && !padded {
        res.push(acc);
    }

    let hash_res = digest(&SHA256, &res);
    let hash = &[
        (header[1] >> 2) as u8,
        (header[1] << 6 | header[2] >> 4) as u8,
        (header[2] << 4 | header[3] >> 6) as u8,
        (header[3] << 2 | header[4] >> 8) as u8,
        header[4] as u8,
    ];

    if &hash_res.as_ref()[0..5] != hash {
        return None;
    }

    // debug_assert!(res.capacity() == cap);
    Some(res)
}

#[cfg(test)]
mod test {
    use super::*;
    use quickcheck::TestResult;

    #[test]
    fn sample() {
        assert!(encode(b"Hello world") == "一患的桟春宴沈紺刊剤袋附液悔");
        assert!(decode("一患的桟春宴沈紺刊剤袋附液悔").unwrap() == b"Hello world");
        assert!(decode("一患的桟春宴沈紺刊時袋附液悔").is_none());
        assert!(decode("時患的桟春宴沈紺刊剤袋附液悔").is_none());
        assert!(decode("一患的桟春宴沈紺☃剤袋附液悔").is_none());
        assert!(decode("☃患的桟春宴沈紺刊剤袋附液悔").is_none());
        assert!(decode("一患☃桟春宴沈紺刊剤袋附液悔").is_none());
    }

    quickcheck! {
        fn identity(bs: Vec<u8>) -> bool {
            decode(&encode(&bs)).unwrap() == bs
        }

        fn checksum(bs: Vec<u8>, idx: usize, add: usize) -> TestResult {
            if add == KYOCODE_CHARS.len() - 1 {
                return TestResult::discard();
            }

            let mut code = encode(&bs).chars().collect::<Vec<char>>();
            let idx = idx % (code.len() - 1);
            code[idx] =
                KYOCODE_CHARS[(KYOCODE_CHARS.binary_search(&code[idx]).unwrap() + add + 1)
                              % KYOCODE_CHARS.len()];
            let code = code.iter().collect::<String>();

            TestResult::from_bool(decode(&code).is_none())
        }
    }
}
