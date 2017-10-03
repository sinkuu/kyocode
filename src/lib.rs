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
    let mut res = String::with_capacity(3 * (5 + div_roundup(bs.len() * 8, 10)));

    let hash = digest(&SHA256, bs);
    let hash = &hash.as_ref()[0..5];
    push_bytes(hash, &mut res);

    let last = (bs.len() * 8) % 10;
    res.push(KYOCODE_CHARS[if 0 < last && last <= 2 { 1 } else { 0 }]);

    push_bytes(bs, &mut res);

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
    //     <String as std::iter::FromIterator<_>>::from_iter(std::iter::once(c)).len() == 3
    // }));

    if s.len() < 5 {
        // No header available
        return None;
    }

    let len = (s.len() - 5) / 3;
    let mut res = Vec::with_capacity(len / 8 * 10 + (len % 8) * 10 / 8);

    let header = &s[0..3 * 5];
    let header = header
        .chars()
        .map(|c| KYOCODE_CHARS.binary_search(&c).ok())
        .collect::<Option<ArrayVec<[usize; 5]>>>();
    let header = try_opt!(header);
    let header = header.as_slice();

    let mut rem = 8;
    let mut acc = 0u8;
    for c in s.chars().skip(5) {
        let i = try_opt!(KYOCODE_CHARS.binary_search(&c)) as u16;

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
