// These are the default casing operations, but locale-specific ‘tailored’ casings are possible.

use crate::ucd::{case_ignorable, cased, lowercase_mapping};

// There are a couple documented cases this doesn't handle.
// 1. In Lithuanian small i with an accent still has a dot, which needs to be added back as
//    the codepoint 0307 (combining dot)
// 2. In Turkish the letter 0049 ‘I’ lowercases to 0131 "ı" (small dotless i).
// They're described in SpecialCasing.txt, but can possibly be ignored, since they depend on the
// locale, at which point, it should probably be combined with the CLDR and the whole thing made
// locale-aware.
pub fn to_lowercase(code_points: Vec<u32>) -> Vec<u32> {
    let mut pos = 0;
    let mut out = Vec::with_capacity(code_points.len());
    let len = code_points.len();
    while pos < len {
        let code_point = code_points[pos];
        match code_point {
            0x0130 => out.extend([105, 775]),
            0x03A3 => out.push(if is_final_sigma(&code_points, pos) { 0x03C2 } else { 0x03C3 }),
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
        let mut xs = code_points.iter().rev().skip(len - sigma_pos).skip_while(|cp| case_ignorable(**cp));
        match xs.next() {
            Some(cp) => cased(*cp),
            None => false,
        }
    };
    prev_char_cased && {
        let mut xs = code_points.iter().skip(sigma_pos + 1).skip_while(|cp| case_ignorable(**cp));
        let next_char_cased = match xs.next() {
            Some(cp) => cased(*cp),
            None => false,
        };
        prev_char_cased && !next_char_cased
    }
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
        assert_eq!(to_lowercase(vec![0x0345, 0x03A3, 0x0020]), vec![0x0345, 0x03C3, 0x0020]);
        // ALPHA YPOGEGRAMMENI SIGMA FULL-STOP BETA - cased ignorable sigma ignorable cased - σ
        assert_eq!(to_lowercase(vec![0x0391, 0x0345, 0x03A3, 0x002E, 0x0392]), vec![0x03B1, 0x0345, 0x03C3, 0x002E, 0x03B2]);
        // ALPHA YPOGEGRAMMENI SIGMA SPACE - cased ignorable sigma not-cased - ς
        assert_eq!(to_lowercase(vec![0x0391, 0x0345, 0x03A3, 0x0020]), vec![0x03B1, 0x0345, 0x03C2, 0x0020]);
        // ALPHA YPOGEGRAMMENI SIGMA - cased ignorable sigma - ς
        assert_eq!(to_lowercase(vec![0x0391, 0x0345, 0x03A3]), vec![0x03B1, 0x0345, 0x03C2]);
    }
}
