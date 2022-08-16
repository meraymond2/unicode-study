use crate::cp_iter::CodePointIter;
use crate::helpers::encode_utf8;
use crate::ucd::{
    combining_class, decomposition_mapping, is_allowed, is_starter, primary_composite,
    QuickCheckVal,
};
use std::cmp::min;

// https://www.unicode.org/reports/tr15/#Detecting_Normalization_Forms

#[derive(Debug, PartialEq)]
pub enum IsNormalised {
    Yes,
    No,
    Maybe,
}

pub enum Normalisation {
    NFC,
    NFD,
    // skipping these for now, I'm not that interested in the compatability equivalence,
    // it would make more sense if one was doing search
    // NFKC,
    // NFKD,
}

pub fn quick_check(code_points: &Vec<u32>, normalisation: Normalisation) -> IsNormalised {
    let mut last_canonical_class: u8 = 0;
    let mut result: IsNormalised = IsNormalised::Yes;
    for code_point in code_points.into_iter() {
        let ccc = combining_class(*code_point);
        if last_canonical_class > ccc && ccc != 0 {
            return IsNormalised::No;
        }
        match is_allowed(*code_point, &normalisation) {
            QuickCheckVal::Yes => {}
            QuickCheckVal::No => {
                return IsNormalised::No;
            }
            QuickCheckVal::Maybe => {
                result = IsNormalised::Maybe;
            }
        }
        last_canonical_class = ccc;
    }

    return result;
}

// These functions are inefficient in that each one iterates through the entire string and each
// intermediate step produces a separate vector. They are done this way to make it easier to see
// how the different stages build on each other, and because it's easier to test.
// In a real application, we could easily skip a few of the intermediate vectors.
pub fn to_nfc_str(bytes: Vec<u8>) -> Vec<u8> {
    let cps: Vec<u32> = CodePointIter::new(bytes).collect();
    to_nfc(&cps).into_iter().flat_map(encode_utf8).collect()
}

// The important bit here is that decompose is recursive.
pub fn decompose(cp: u32) -> Vec<u32> {
    match decomposition_mapping(cp) {
        None => vec![cp],
        Some(dm) => dm.into_iter().flat_map(decompose).collect(),
    }
}

// Decompose and canonically order the code points. Canonical ordering needs to use a stable sort,
// which luckily Rust's default sort is.
pub fn to_nfd(code_points: &Vec<u32>) -> Vec<u32> {
    let mut decomposed: Vec<u32> = code_points.into_iter().fold(Vec::new(), |mut acc, cp| {
        acc.extend(decompose(*cp));
        acc
    });
    let mut pos = 0;
    while pos < decomposed.len() {
        let next_starter_offset = decomposed[pos..]
            .iter()
            .skip(1)
            .position(|cp| is_starter(*cp))
            .map(|offset| offset + 1)
            .unwrap_or(decomposed.len() - pos);
        decomposed[pos..(pos + next_starter_offset)]
            .sort_by(|a, b| combining_class(*a).cmp(&combining_class(*b)));
        pos += next_starter_offset
    }
    decomposed
}

fn to_nfc(code_points: &Vec<u32>) -> Vec<u32> {
    let mut nfd = to_nfd(code_points);
    let mut pos = 0;
    let mut try_compose = true;
    loop {
        if try_compose {
            try_compose = false;
            let char_seq_end = nfd[pos..]
                .into_iter()
                .skip(1) // skip the current starter
                .position(|cp| is_starter(*cp)) // find the next starter, idx is from pos.skip(1) not pos
                .map(|offset| min(offset + 2, nfd.len() - pos)) // add one for beginning and ending starter
                .unwrap_or(nfd.len() - pos); // if no starters left in string, return the end

            let mut last_ccc = 0;
            for i in 1..char_seq_end {
                let ccc = combining_class(nfd[pos + i]);
                if let Some(composite) = primary_composite(nfd[pos], nfd[pos + i]) {
                    // A starter-combining mark pair may be blocked by an intervening combining mark
                    // if they have equal combining classes. Given A B C, if A + C form a pair, but
                    // both B and C have the combining class 200, C is blocked by B.
                    if ccc > 0 && ccc == last_ccc {
                        break;
                    }
                    // A starter-starter pair is blocked if there are combining marks in between.
                    if ccc < last_ccc {
                        break;
                    }
                    // If not blocked, replace the starter with the composite, remove the other half,
                    // and go back and try again. It's necessary to retry in place, because you can
                    // have A B C, where A and B produce D, and D and C produce E. So we don't just
                    // continue after having made a composite.
                    nfd[pos] = composite;
                    nfd.remove(pos + i);
                    try_compose = true;
                    break;
                } else {
                    last_ccc = ccc;
                }
            }
        } else {
            match nfd[pos..]
                .iter()
                .skip(1)
                .position(|cp| is_starter(*cp))
                .map(|offset| offset + 1)
            {
                Some(offset) => {
                    pos += offset;
                    try_compose = true;
                }
                None => break,
            }
        }
    }
    nfd
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://www.unicode.org/Public/14.0.0/ucd/NormalizationTest.txt
    /*
    # CONFORMANCE:
    # 1. The following invariants must be true for all conformant implementations
    #
    #    NFC
    #      c2 ==  toNFC(c1) ==  toNFC(c2) ==  toNFC(c3)
    #      c4 ==  toNFC(c4) ==  toNFC(c5)
    */
    fn nfc_conformance_test(c: Vec<Vec<u32>>) {
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));
    }

    /*
    #    NFD
    #      c3 ==  toNFD(c1) ==  toNFD(c2) ==  toNFD(c3)
    #      c5 ==  toNFD(c4) ==  toNFD(c5)
    #
    */
    fn nfd_conformance_test(c: Vec<Vec<u32>>) {
        assert_eq!(c[2], to_nfd(&c[0]));
        assert_eq!(c[2], to_nfd(&c[1]));
        assert_eq!(c[2], to_nfd(&c[2]));
        assert_eq!(c[4], to_nfd(&c[3]));
        assert_eq!(c[4], to_nfd(&c[4]));
    }

    fn parse_line(line: &str) -> Vec<Vec<u32>> {
        line.split(";")
            .take(5)
            .map(|block| {
                block
                    .split_whitespace()
                    .map(|s| u32::from_str_radix(s, 16).unwrap())
                    .collect()
            })
            .collect()
    }

    fn load_test_cases() -> Vec<Vec<Vec<u32>>> {
        std::fs::read_to_string(std::path::Path::new("resources/NormalizationTest.txt"))
            .unwrap()
            .split("\n")
            .filter(|line| !line.is_empty() && !line.starts_with("#") && !line.starts_with("@"))
            .map(parse_line)
            .collect()
    }

    #[test]
    fn test_quick_check() {
        // "å"
        assert_eq!(
            quick_check(&vec![0x00E5], Normalisation::NFC),
            IsNormalised::Yes
        );
        // "å" decomposed, quick check says maybe, because there are combining marks
        // it's actually not normalised, but those code points could make up a normalised string
        assert_eq!(
            quick_check(&vec![0x61, 0x030A], Normalisation::NFC),
            IsNormalised::Maybe
        );

        for case in load_test_cases() {
            // This is the NFC normalised case, so quick check can only identify that it's not not normalised.
            assert_ne!(quick_check(&case[1], Normalisation::NFC), IsNormalised::No);
            // NFD normalised case. There's no maybe decomposed, so we know this is yes.
            assert_eq!(quick_check(&case[2], Normalisation::NFD), IsNormalised::Yes);
        }
    }

    #[test]
    fn test_to_nfc() {
        for case in load_test_cases() {
            nfc_conformance_test(case);
        }
    }

    #[test]
    fn test_to_nfd() {
        for case in load_test_cases() {
            nfd_conformance_test(case)
        }
    }
}
