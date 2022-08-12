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

fn decompose_cp(cp: u32) -> Vec<u32> {
    match decomposition_mapping(cp) {
        None => vec![cp],
        Some(dm) => dm.into_iter().flat_map(decompose_cp).collect()
    }
}

pub fn to_nfc_str(bytes: Vec<u8>) -> Vec<u8> {
    // TODO: this intermediate vec isn't necessary, I've just split up the funcs
    // because the provided test cases are in code points, not utf-8
    let cps: Vec<u32> = CodePointIter::new(bytes).collect();
    to_nfc(&cps).into_iter().flat_map(encode_utf8).collect()
}

fn to_nfc(cps: &Vec<u32>) -> Vec<u32> {
    let decomposed: Vec<u32> = cps.into_iter()
        .fold(Vec::new(), |mut acc, cp| {
            acc.extend(decompose_cp(*cp));
            acc
        });

    let mut normalised_code_points: Vec<u32> = Vec::with_capacity(decomposed.len());
    let mut pos = 0;
    while pos < decomposed.len() {
        let seq_start_pos = pos;
        pos += 1;
        while pos < decomposed.len() && !is_starter(decomposed[pos]) {
            pos += 1;
        }
        let seq_end_pos = pos;
        let mut seq = decomposed[seq_start_pos..seq_end_pos].to_vec();
        seq.sort_by(|a, b| combining_class(*a).cmp(&combining_class(*b)));
        'outer: loop {
            for i in 0..seq.len() {
                if let Some(composite) = primary_composite(seq[0], seq[i]) {
                    seq[0] = composite;
                    seq.remove(i);
                    continue 'outer;
                }
            }
            break 'outer;
        }
        normalised_code_points.extend(seq);
    }

    normalised_code_points
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

    #[test]
    /*
    # CONFORMANCE:
    # 1. The following invariants must be true for all conformant implementations
    #
    #    NFC
    #      c2 ==  toNFC(c1) ==  toNFC(c2) ==  toNFC(c3)
    #      c4 ==  toNFC(c4) ==  toNFC(c5)
    */
    fn test_to_nfc() {
        // 1E0A;1E0A;0044 0307;1E0A;0044 0307; # (Ḋ; Ḋ; D◌̇; Ḋ; D◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE
        let c: Vec<Vec<u32>> = vec![vec![0x1E0A], vec![0x1E0A], vec![0x0044, 0x0307], vec![0x1E0A], vec![0x0044, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0C;1E0C;0044 0323;1E0C;0044 0323; # (Ḍ; Ḍ; D◌̣; Ḍ; D◌̣; ) LATIN CAPITAL LETTER D WITH DOT BELOW
        let c: Vec<Vec<u32>> = vec![vec![0x1E0C], vec![0x1E0C], vec![0x0044, 0x0323], vec![0x1E0C], vec![0x0044, 0x0323]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0A 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (Ḋ◌̣; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING DOT BELOW
        let c: Vec<Vec<u32>> = vec![vec![0x1E0A, 0x0323], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0C 0307;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (Ḍ◌̇; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING DOT ABOVE
        let c: Vec<Vec<u32>> = vec![vec![0x1E0C, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0044 0307 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (D◌̇◌̣; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING DOT ABOVE, COMBINING DOT BELOW
        let c: Vec<Vec<u32>> = vec![vec![0x0044, 0x0307, 0x0323], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0044 0323 0307;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING DOT BELOW, COMBINING DOT ABOVE
        let c: Vec<Vec<u32>> = vec![vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0A 031B;1E0A 031B;0044 031B 0307;1E0A 031B;0044 031B 0307; # (Ḋ◌̛; Ḋ◌̛; D◌̛◌̇; Ḋ◌̛; D◌̛◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING HORN
        let c: Vec<Vec<u32>> = vec![vec![0x1E0A, 0x031B], vec![0x1E0A, 0x031B], vec![0x0044, 0x031B, 0x0307], vec![0x1E0A, 0x031B], vec![0x0044, 0x031B, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0C 031B;1E0C 031B;0044 031B 0323;1E0C 031B;0044 031B 0323; # (Ḍ◌̛; Ḍ◌̛; D◌̛◌̣; Ḍ◌̛; D◌̛◌̣; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING HORN
        let c: Vec<Vec<u32>> = vec![vec![0x1E0C, 0x031B], vec![0x1E0C, 0x031B], vec![0x0044, 0x031B, 0x0323], vec![0x1E0C, 0x031B], vec![0x0044, 0x031B, 0x0323]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0A 031B 0323;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (Ḋ◌̛◌̣; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING HORN, COMBINING DOT BELOW
        let c: Vec<Vec<u32>> = vec![vec![0x1E0A, 0x031B, 0x0323], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E0C 031B 0307;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (Ḍ◌̛◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING HORN, COMBINING DOT ABOVE
        let c: Vec<Vec<u32>> = vec![vec![0x1E0C, 0x031B, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0044 031B 0307 0323;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (D◌̛◌̇◌̣; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING HORN, COMBINING DOT ABOVE, COMBINING DOT BELOW
        let c: Vec<Vec<u32>> = vec![vec![0x0044, 0x031B, 0x0307, 0x0323], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING HORN, COMBINING DOT BELOW, COMBINING DOT ABOVE
        let c: Vec<Vec<u32>> = vec![vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 00C8;00C8;0045 0300;00C8;0045 0300; # (È; È; E◌̀; È; E◌̀; ) LATIN CAPITAL LETTER E WITH GRAVE
        let c: Vec<Vec<u32>> = vec![vec![0x00C8], vec![0x00C8], vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0112;0112;0045 0304;0112;0045 0304; # (Ē; Ē; E◌̄; Ē; E◌̄; ) LATIN CAPITAL LETTER E WITH MACRON
        let c: Vec<Vec<u32>> = vec![vec![0x0112], vec![0x0112], vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0045 0300;00C8;0045 0300;00C8;0045 0300; # (E◌̀; È; E◌̀; È; E◌̀; ) LATIN CAPITAL LETTER E, COMBINING GRAVE ACCENT
        let c: Vec<Vec<u32>> = vec![vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0045 0304;0112;0045 0304;0112;0045 0304; # (E◌̄; Ē; E◌̄; Ē; E◌̄; ) LATIN CAPITAL LETTER E, COMBINING MACRON
        let c: Vec<Vec<u32>> = vec![vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E14;1E14;0045 0304 0300;1E14;0045 0304 0300; # (Ḕ; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E WITH MACRON AND GRAVE
        let c: Vec<Vec<u32>> = vec![vec![0x1E14], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0112 0300;1E14;0045 0304 0300;1E14;0045 0304 0300; # (Ē◌̀; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E WITH MACRON, COMBINING GRAVE ACCENT
        let c: Vec<Vec<u32>> = vec![vec![0x0112, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1E14 0304;1E14 0304;0045 0304 0300 0304;1E14 0304;0045 0304 0300 0304; # (Ḕ◌̄; Ḕ◌̄; E◌̄◌̀◌̄; Ḕ◌̄; E◌̄◌̀◌̄; ) LATIN CAPITAL LETTER E WITH MACRON AND GRAVE, COMBINING MACRON
        let c: Vec<Vec<u32>> = vec![vec![0x1E14, 0x0304], vec![0x1E14, 0x0304], vec![0x0045, 0x0304, 0x0300, 0x0304], vec![0x1E14, 0x0304], vec![0x0045, 0x0304, 0x0300, 0x0304]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0045 0304 0300;1E14;0045 0304 0300;1E14;0045 0304 0300; # (E◌̄◌̀; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E, COMBINING MACRON, COMBINING GRAVE ACCENT
        let c: Vec<Vec<u32>> = vec![vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0045 0300 0304;00C8 0304;0045 0300 0304;00C8 0304;0045 0300 0304; # (E◌̀◌̄; È◌̄; E◌̀◌̄; È◌̄; E◌̀◌̄; ) LATIN CAPITAL LETTER E, COMBINING GRAVE ACCENT, COMBINING MACRON
        let c: Vec<Vec<u32>> = vec![vec![0x0045, 0x0300, 0x0304], vec![0x00C8, 0x0304], vec![0x0045, 0x0300, 0x0304], vec![0x00C8, 0x0304], vec![0x0045, 0x0300, 0x0304]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 05B8 05B9 05B1 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F; # (◌ָ◌ֹ◌ֱ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ) HEBREW POINT QAMATS, HEBREW POINT HOLAM, HEBREW POINT HATAF SEGOL, HEBREW ACCENT ETNAHTA, HEBREW PUNCTUATION SOF PASUQ, HEBREW POINT SHEVA, HEBREW ACCENT ILUY, HEBREW ACCENT QARNEY PARA
        let c: Vec<Vec<u32>> = vec![vec![0x05B8, 0x05B9, 0x05B1, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 0592 05B7 05BC 05A5 05B0 05C0 05C4 05AD;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4; # (◌֒◌ַ◌ּ◌֥◌ְ׀◌ׄ◌֭; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ) HEBREW ACCENT SEGOL, HEBREW POINT PATAH, HEBREW POINT DAGESH OR MAPIQ, HEBREW ACCENT MERKHA, HEBREW POINT SHEVA, HEBREW PUNCTUATION PASEQ, HEBREW MARK UPPER DOT, HEBREW ACCENT DEHI
        let c: Vec<Vec<u32>> = vec![vec![0x0592, 0x05B7, 0x05BC, 0x05A5, 0x05B0, 0x05C0, 0x05C4, 0x05AD], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1100 AC00 11A8;1100 AC01;1100 1100 1161 11A8;1100 AC01;1100 1100 1161 11A8; # (ᄀ각; ᄀ각; ᄀ각; ᄀ각; ᄀ각; ) HANGUL CHOSEONG KIYEOK, HANGUL SYLLABLE GA, HANGUL JONGSEONG KIYEOK
        let c: Vec<Vec<u32>> = vec![vec![0x1100, 0xAC00, 0x11A8], vec![0x1100, 0xAC01], vec![0x1100, 0x1100, 0x1161, 0x11A8], vec![0x1100, 0xAC01], vec![0x1100, 0x1100, 0x1161, 0x11A8]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));

        // 1100 AC00 11A8 11A8;1100 AC01 11A8;1100 1100 1161 11A8 11A8;1100 AC01 11A8;1100 1100 1161 11A8 11A8; # (ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ) HANGUL CHOSEONG KIYEOK, HANGUL SYLLABLE GA, HANGUL JONGSEONG KIYEOK, HANGUL JONGSEONG KIYEOK
        let c: Vec<Vec<u32>> = vec![vec![0x1100, 0xAC00, 0x11A8, 0x11A8], vec![0x1100, 0xAC01, 0x11A8], vec![0x1100, 0x1100, 0x1161, 0x11A8, 0x11A8], vec![0x1100, 0xAC01, 0x11A8], vec![0x1100, 0x1100, 0x1161, 0x11A8, 0x11A]];
        assert_eq!(c[1], to_nfc(&c[0]));
        assert_eq!(c[1], to_nfc(&c[1]));
        assert_eq!(c[1], to_nfc(&c[2]));
        assert_eq!(c[3], to_nfc(&c[3]));
        assert_eq!(c[3], to_nfc(&c[4]));
    }
}
