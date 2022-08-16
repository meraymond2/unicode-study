// These are the default casing operations, but locale-specific ‘tailored’ casings are possible.

use crate::normalise::{decompose, to_nfd};
use crate::ucd::{case_folding, case_ignorable, cased, lowercase_mapping, uppercase_mapping};
use std::cmp::Ordering;

// There are a couple documented cases this doesn't handle.
// 1. In Lithuanian small i with an accent still has a dot, which needs to be added back as
//    the codepoint 0307 (combining dot)
// 2. In Turkish the letter 0049 ‘I’ lowercases to 0131 "ı" (small dotless i).
// They're described in SpecialCasing.txt, but can possibly be ignored, since they depend on the
// locale, at which point, it should probably be combined with the CLDR and the whole thing made
// locale-aware.
pub fn to_lowercase(code_points: Vec<u32>) -> Vec<u32> {
    let mut pos = 0;
    let len = code_points.len();
    let mut out = Vec::with_capacity(len);
    while pos < len {
        let code_point = code_points[pos];
        match code_point {
            0x0130 => out.extend([105, 775]),
            0x03A3 => out.push(if is_final_sigma(&code_points, pos) {
                0x03C2
            } else {
                0x03C3
            }),
            _ => out.push(lowercase_mapping(code_point).unwrap_or(code_point)),
        }
        pos += 1;
    }
    out
}

// 03A3 GREEK CAPITAL LETTER SIGMA has a different lowercase if it occurs at the end
// of a word. The way this is checked is (Table 3-17):
// > C is preceded by a sequence consisting of a cased letter and then zero or
// > more case-ignorable characters, and C is not followed by a sequence consisting
// > of zero or more case-ignorable characters and then a cased letter.
fn is_final_sigma(code_points: &Vec<u32>, sigma_pos: usize) -> bool {
    let len = code_points.len();
    let prev_char_cased = {
        let mut xs = code_points
            .iter()
            .rev()
            .skip(len - sigma_pos)
            .skip_while(|cp| case_ignorable(**cp));
        match xs.next() {
            Some(cp) => cased(*cp),
            None => false,
        }
    };
    prev_char_cased && {
        let mut xs = code_points
            .iter()
            .skip(sigma_pos + 1)
            .skip_while(|cp| case_ignorable(**cp));
        let next_char_cased = match xs.next() {
            Some(cp) => cased(*cp),
            None => false,
        };
        prev_char_cased && !next_char_cased
    }
}

// I'm not aware of any edge cases for upper-casing, at least none that aren't locale-specific.
pub fn to_uppercase(code_points: Vec<u32>) -> Vec<u32> {
    let mut out = Vec::with_capacity(code_points.len());
    for code_point in code_points {
        out.extend(uppercase_mapping(code_point).unwrap_or(vec![code_point]));
    }
    out
}

// > D145 A string X is a canonical caseless match for a string Y if and only if:
// >      NFD(toCasefold(NFD(X))) = NFD(toCasefold(NFD(Y)))
// The initial decomposition is to get around an edge case with combining greek ypogegrammeni, but
// it doesn't really explain what the issue is. It suggests avoiding the extra decomp by just
// handling that char, and any that have it in their decomposition. That check would have to go in
// case fold, because the point is to avoid an additional iteration. And it doesn't matter if we
// only decompose some chars, because case folding doesn't guarantee normalisation .
pub fn canonical_caseless_match(x: Vec<u32>, y: Vec<u32>) -> Ordering {
    to_nfd(&case_fold(&x)).cmp(&to_nfd(&case_fold(&y)))
}

// Case folding is a way to case-insensitively compare strings. Why not just lower/uppercase them?
// It also irons out some of the edge cases. Context-sensitive casings are gone, so the position
// of the sigma doesn't matter. The ß folds to ss, so you don't have to worry about matching it
// against the capital SS. It's pretty simple, because it's casing without the edge cases.
// This isn't actually enough to do string comparison, it's a first step, but it doesn't produce
// normalised strings, so that has to happen afterwards.
pub fn case_fold(code_points: &Vec<u32>) -> Vec<u32> {
    let mut out = Vec::with_capacity(code_points.len());
    let ypogegrammenic = vec![
        0x1F80, 0x1F81, 0x1F82, 0x1F83, 0x1F84, 0x1F85, 0x1F86, 0x1F87, 0x1F88, 0x1F89, 0x1F8A,
        0x1F8B, 0x1F8C, 0x1F8D, 0x1F8E, 0x1F8F, 0x1F90, 0x1F91, 0x1F92, 0x1F93, 0x1F94, 0x1F95,
        0x1F96, 0x1F97, 0x1F98, 0x1F99, 0x1F9A, 0x1F9B, 0x1F9C, 0x1F9D, 0x1F9E, 0x1F9F, 0x1FA0,
        0x1FA1, 0x1FA2, 0x1FA3, 0x1FA4, 0x1FA5, 0x1FA6, 0x1FA7, 0x1FA8, 0x1FA9, 0x1FAA, 0x1FAB,
        0x1FAC, 0x1FAD, 0x1FAE, 0x1FAF, 0x1FB2, 0x1FB3, 0x1FB4, 0x1FB7, 0x1FBC, 0x1FC2, 0x1FC3,
        0x1FC4, 0x1FC7, 0x1FCC, 0x1FF2, 0x1FF3, 0x1FF4, 0x1FF7, 0x1FFC,
    ];
    for code_point in code_points {
        if ypogegrammenic.contains(code_point) {
            for cp in decompose(*code_point) {
                out.extend(case_folding(cp).unwrap_or(vec![cp]));
            }
        } else {
            out.extend(case_folding(*code_point).unwrap_or(vec![*code_point]));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_lowercase() {
        assert_eq!(to_lowercase(vec![0x0130]), vec![105, 775]);
        assert_eq!(to_lowercase(vec![0x011E]), vec![0x011F]);

        // SIGMA - σ
        assert_eq!(to_lowercase(vec![0x03A3]), vec![0x03C3]);
        // YPOGEGRAMMENI (ignorable) SIGMA SPACE - ignorable sigma not-cased - σ
        assert_eq!(
            to_lowercase(vec![0x0345, 0x03A3, 0x0020]),
            vec![0x0345, 0x03C3, 0x0020]
        );
        // ALPHA YPOGEGRAMMENI SIGMA FULL-STOP BETA - cased ignorable sigma ignorable cased - σ
        assert_eq!(
            to_lowercase(vec![0x0391, 0x0345, 0x03A3, 0x002E, 0x0392]),
            vec![0x03B1, 0x0345, 0x03C3, 0x002E, 0x03B2]
        );
        // ALPHA YPOGEGRAMMENI SIGMA SPACE - cased ignorable sigma not-cased - ς
        assert_eq!(
            to_lowercase(vec![0x0391, 0x0345, 0x03A3, 0x0020]),
            vec![0x03B1, 0x0345, 0x03C2, 0x0020]
        );
        // ALPHA YPOGEGRAMMENI SIGMA - cased ignorable sigma - ς
        assert_eq!(
            to_lowercase(vec![0x0391, 0x0345, 0x03A3]),
            vec![0x03B1, 0x0345, 0x03C2]
        );
    }

    #[test]
    fn test_to_uppercase() {
        // ß -> SS
        assert_eq!(to_uppercase(vec![0x00DF]), vec![0x0053, 0x0053]);
        // ŉ -> ʼN
        assert_eq!(to_uppercase(vec![0x0149]), vec![0x02BC, 0x004E]);
        // ǰ -> J̌
        assert_eq!(to_uppercase(vec![0x01F0]), vec![0x004A, 0x030C]);
        // ΐ -> Ϊ́
        assert_eq!(to_uppercase(vec![0x0390]), vec![0x0399, 0x0308, 0x0301]);
        // ΰ -> Ϋ́
        assert_eq!(to_uppercase(vec![0x03B0]), vec![0x03A5, 0x0308, 0x0301]);
        // և -> ԵՒ
        assert_eq!(to_uppercase(vec![0x0587]), vec![0x0535, 0x0552]);
        // ẖ -> H̱
        assert_eq!(to_uppercase(vec![0x1E96]), vec![0x0048, 0x0331]);
        // ﬄ -> FFL
        assert_eq!(to_uppercase(vec![0xFB04]), vec![0x0046, 0x0046, 0x004C]);
        // . -> .
        assert_eq!(to_uppercase(vec![0x002E]), vec![0x002E]);
    }

    #[test]
    fn test_case_fold() {}
}
