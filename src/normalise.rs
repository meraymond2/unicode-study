use std::cmp::min;
use crate::cp_iter::CodePointIter;
use crate::helpers::encode_utf8;
use crate::ucd::{combining_class, decomposition_mapping, is_starter, nfc_is_allowed, primary_composite, QuickCheckVal};

// https://www.unicode.org/reports/tr15/#Detecting_Normalization_Forms

#[derive(Debug, PartialEq)]
pub enum IsNormalised {
    Yes,
    No,
    Maybe,
}

pub fn quick_check(bytes: Vec<u8>) -> IsNormalised {
    let code_points = CodePointIter::new(bytes);
    let mut last_canonical_class: u8 = 0;
    let mut result: IsNormalised = IsNormalised::Yes;
    for code_point in code_points.into_iter() {
        let ccc = combining_class(code_point);
        if last_canonical_class > ccc && ccc != 0 {
            return IsNormalised::No;
        }
        match nfc_is_allowed(code_point) {
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

fn decompose(cp: u32) -> Vec<u32> {
    match decomposition_mapping(cp) {
        None => vec![cp],
        Some(dm) => dm.into_iter().flat_map(decompose).collect()
    }
}

pub fn to_nfc_str(bytes: Vec<u8>) -> Vec<u8> {
    // Note: this intermediate vec isn't necessary, I've just split up the funcs
    // because the provided test cases are in code points, not utf-8
    let cps: Vec<u32> = CodePointIter::new(bytes).collect();
    to_nfc(&cps).into_iter().flat_map(encode_utf8).collect()
}

fn to_nfc(cps: &Vec<u32>) -> Vec<u32> {
    let mut decomposed: Vec<u32> = cps.into_iter()
        .fold(Vec::new(), |mut acc, cp| {
            acc.extend(decompose(*cp));
            acc
        });

    let mut pos = 0;
    let mut try_compose = true;
    loop {
        if try_compose {
            try_compose = false;
            let next_starter_offset = decomposed[pos..].iter().skip(1).position(|cp| is_starter(*cp)).map(|offset| offset + 1).unwrap_or(decomposed.len() - pos);
            let char_seq_end = min(next_starter_offset + 1, decomposed.len() - pos);
            decomposed[pos..(pos + next_starter_offset)].sort_by(|a, b| combining_class(*a).cmp(&combining_class(*b)));

            let mut last_ccc = 0;
            for i in 1..char_seq_end {
                let ccc = combining_class(decomposed[pos + i]);
                if let Some(composite) = primary_composite(decomposed[pos], decomposed[pos + i]) {
                    // A starter-combining mark pair may be blocked by an intervening combining mark
                    // if it has the same ccc.
                    if ccc > 0 && ccc == last_ccc { break; }
                    // A starter-starter pair is blocked if there are combining marks in between.
                    if ccc < last_ccc { break; }
                    decomposed[pos] = composite;
                    decomposed.remove(pos + i);
                    try_compose = true;
                    break;
                } else {
                    last_ccc = ccc;
                }
            }
        } else {
            match decomposed[pos..].iter().skip(1).position(|cp| is_starter(*cp)) {
                Some(offset) => {
                    pos += offset + 1;
                    try_compose = true;
                }
                None => break
            }
        }
    }
    decomposed
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quick_check() {
        // "å"
        assert_eq!(quick_check(vec![0xc3, 0xa5]), IsNormalised::Yes);
        // "å" decomposed, quick check says maybe, because there are combining marks
        // it's actually not normalised, but those code points could make up a normalised string
        assert_eq!(quick_check(vec![0x61, 0xcc, 0x8a]), IsNormalised::Maybe);
    }


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

    fn parse_line(line: &str) -> Vec<Vec<u32>> {
        line.split(";").take(5).map(|block| block.split_whitespace().map(|s| u32::from_str_radix(s, 16).unwrap()).collect()).collect()
    }

    #[test]
    fn test_to_nfc() {
        // https://www.unicode.org/Public/14.0.0/ucd/NormalizationTest.txt
        let test_cases: Vec<Vec<Vec<u32>>> = std::fs::read_to_string(std::path::Path::new("resources/NormalizationTest.txt"))
            .unwrap()
            .split("\n")
            .filter(|line| !line.is_empty() && !line.starts_with("#") && !line.starts_with("@"))
            .map(parse_line)
            .collect();

        for case in test_cases {
            nfc_conformance_test(case);
        }
    }
}
