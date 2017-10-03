#[cfg(test)]
#[macro_use]
extern crate quickcheck;
extern crate ring;

use ring::digest::{digest, SHA256};

mod chars;
pub use chars::KYOCODE_CHARS;

const BITS_PER_CODE: u8 = 10;
const BITS_PER_BYTE: u8 = 8;

pub fn encode(bs: &[u8]) -> String {
    let mut res =
        String::with_capacity(3 * (5 + bs.len() * 8 / 10 + if (bs.len() * 8) % 10 > 0 { 1 } else { 0 }));

    let hash = digest(&SHA256, bs);
    let hash = &hash.as_ref()[0..5];
    res.push(KYOCODE_CHARS[(hash[0] as usize) << 2 | (hash[1] as usize) >> 6]);
    res.push(KYOCODE_CHARS[(hash[1] as usize & ((1 << 6) - 1)) << 4 | (hash[2] as usize) >> 4]);
    res.push(KYOCODE_CHARS[(hash[2] as usize & ((1 << 4) - 1)) << 6 | (hash[3] as usize) >> 2]);
    res.push(KYOCODE_CHARS[(hash[3] as usize & ((1 << 2) - 1)) << 8 | (hash[4] as usize)]);
    let last = (bs.len() * 8) % 10;
    res.push(KYOCODE_CHARS[if 0 < last && last <= 2 { 1 } else { 0 }]);

    let mut acc = 0u16;
    let mut rem = BITS_PER_CODE;
    for &b in bs {
        if rem == 8 {
            acc |= b as u16;
            res.push(KYOCODE_CHARS[acc as usize]);
            rem = BITS_PER_CODE;
            acc = 0;
        } else if rem < 8 {
            acc |= b as u16 >> (8 - rem);
            res.push(KYOCODE_CHARS[acc as usize]);
            acc = (b << rem >> rem) as u16;
            rem = BITS_PER_CODE - (8 - rem);
            acc <<= rem;
        } else {
            acc |= b as u16;
            rem -= 8;
            acc <<= rem;
        }
        // println!("{:02X} {:010b} {:2} {}", b, acc, rem, res);
    }

    if rem % BITS_PER_CODE > 0 {
        res.push(KYOCODE_CHARS[acc as usize]);
        // println!("__ {:010b} __ {}", acc, res);
    }

    res
}

pub fn decode(s: &str) -> Option<Vec<u8>> {
    // debug_assert!(KYOCODE_CHARS.iter().all(|c| {
    //     <String as std::iter::FromIterator<_>>::from_iter(std::iter::once(c)).len() == 3
    // }));

    if s.len() < 5 {
        // No header available
        return None;
    }

    let len = (s.len() - 5) / 3;
    let mut res = Vec::with_capacity(len / 8 * 10 + (len % 8) * 10 / 8);

    let header = &s[0..3 * 5];

    let mut rem = 8;
    let mut acc = 0u8;
    for c in s.chars().skip(5) {
        if let Ok(i) = KYOCODE_CHARS.binary_search(&c) {
            let i = i as u16;

            let mut nxt = BITS_PER_CODE - rem;
            acc |= (i >> nxt) as u8;
            res.push(acc);
            if nxt > BITS_PER_BYTE {
                res.push((i >> (nxt - BITS_PER_BYTE)) as u8);
                nxt -= BITS_PER_BYTE;
            }
            rem = BITS_PER_BYTE - nxt;
            acc = (i << rem) as u8;
        } else {
            return None;
        }
        // println!("{} {:010b} {:2}", c, acc, rem);
    }

    let header = {
        let mut header = header.chars();
        &[
            KYOCODE_CHARS
                .binary_search(&header.next().unwrap())
                .unwrap(),
            KYOCODE_CHARS
                .binary_search(&header.next().unwrap())
                .unwrap(),
            KYOCODE_CHARS
                .binary_search(&header.next().unwrap())
                .unwrap(),
            KYOCODE_CHARS
                .binary_search(&header.next().unwrap())
                .unwrap(),
            KYOCODE_CHARS
                .binary_search(&header.next().unwrap())
                .unwrap(),
        ]
    };

    if rem == 0 && header[4] & 1 == 0 {
        res.push(acc);
    }

    let hash_res = digest(&SHA256, &res);
    let hash = &[
        (header[0] >> 2) as u8,
        (header[0] << 6 | header[1] >> 4) as u8,
        (header[1] << 4 | header[2] >> 6) as u8,
        (header[2] << 2 | header[3] >> 8) as u8,
        header[3] as u8,
    ];

    if &hash_res.as_ref()[0..5] != hash {
        return None;
    }

    Some(res)
}

#[cfg(test)]
mod test {
    use super::*;

    quickcheck! {
        fn identity(bs: Vec<u8>) -> bool {
            decode(&encode(&bs)).unwrap() == bs
        }
    }
}
