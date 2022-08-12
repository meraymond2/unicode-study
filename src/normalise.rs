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
    // TODO: this intermediate vec isn't necessary, I've just split up the funcs
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
            // TODO: this fails one of the cases, because the pairs aren't just Starter..next-starter.
            // If there are multiple combining marks in a row that have the same ccc, then that will block it.
            // e.g. 0x61 0x5ae 0x0300 is ok, the first and third compose, but 0x61 0x5ae 0x305 0x0300 won't
            // because the 305 blocks the 300
            let next_starter_offset = decomposed[pos..].iter().skip(1).position(|cp| is_starter(*cp)).map(|offset| offset + 1).unwrap_or(decomposed.len() - pos);
            let char_seq_end = min(next_starter_offset + 1, decomposed.len() - pos);
            decomposed[pos..(pos + next_starter_offset)].sort_by(|a, b| combining_class(*a).cmp(&combining_class(*b)));
            for i in 1..char_seq_end {
                if let Some(composite) = primary_composite(decomposed[pos], decomposed[pos + i]) {
                    decomposed[pos] = composite;
                    decomposed.remove(pos + i);
                    try_compose = true;
                    break;
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
    fn conformance_test(c: Vec<Vec<u32>>) {
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
        let test_cases: Vec<Vec<Vec<u32>>> = std::fs::read_to_string(std::path::Path::new("resources/NormalizationTest.txt"))
            .unwrap()
            .split("\n")
            .filter(|line| !line.starts_with("#") && !line.starts_with("@"))
            .map(parse_line)
            .collect();

        // conformance_test(vec![vec![0x00A0], vec![0x00A0],vec![0x00A0],vec![0x0020],vec![0x0020]]);
        for case in test_cases {
            conformance_test(case);
        }

        //
        //
        // // https://www.unicode.org/Public/14.0.0/ucd/NormalizationTest.txt
        // // 1E0A;1E0A;0044 0307;1E0A;0044 0307; # (Ḋ; Ḋ; D◌̇; Ḋ; D◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE
        // conformance_test(vec![vec![0x1E0A], vec![0x1E0A], vec![0x0044, 0x0307], vec![0x1E0A], vec![0x0044, 0x0307]]);
        //
        // // 1E0C;1E0C;0044 0323;1E0C;0044 0323; # (Ḍ; Ḍ; D◌̣; Ḍ; D◌̣; ) LATIN CAPITAL LETTER D WITH DOT BELOW
        // conformance_test(vec![vec![0x1E0C], vec![0x1E0C], vec![0x0044, 0x0323], vec![0x1E0C], vec![0x0044, 0x0323]]);
        //
        // // 1E0A 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (Ḋ◌̣; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING DOT BELOW
        // conformance_test(vec![vec![0x1E0A, 0x0323], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]]);
        //
        // // 1E0C 0307;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (Ḍ◌̇; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING DOT ABOVE
        // conformance_test(vec![vec![0x1E0C, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]]);
        //
        // // 0044 0307 0323;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (D◌̇◌̣; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING DOT ABOVE, COMBINING DOT BELOW
        // conformance_test(vec![vec![0x0044, 0x0307, 0x0323], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]]);
        //
        // // 0044 0323 0307;1E0C 0307;0044 0323 0307;1E0C 0307;0044 0323 0307; # (D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; Ḍ◌̇; D◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING DOT BELOW, COMBINING DOT ABOVE
        // conformance_test(vec![vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307], vec![0x1E0C, 0x0307], vec![0x0044, 0x0323, 0x0307]]);
        //
        // // 1E0A 031B;1E0A 031B;0044 031B 0307;1E0A 031B;0044 031B 0307; # (Ḋ◌̛; Ḋ◌̛; D◌̛◌̇; Ḋ◌̛; D◌̛◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING HORN
        // conformance_test(vec![vec![0x1E0A, 0x031B], vec![0x1E0A, 0x031B], vec![0x0044, 0x031B, 0x0307], vec![0x1E0A, 0x031B], vec![0x0044, 0x031B, 0x0307]]);
        //
        // // 1E0C 031B;1E0C 031B;0044 031B 0323;1E0C 031B;0044 031B 0323; # (Ḍ◌̛; Ḍ◌̛; D◌̛◌̣; Ḍ◌̛; D◌̛◌̣; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING HORN
        // conformance_test(vec![vec![0x1E0C, 0x031B], vec![0x1E0C, 0x031B], vec![0x0044, 0x031B, 0x0323], vec![0x1E0C, 0x031B], vec![0x0044, 0x031B, 0x0323]]);
        //
        // // 1E0A 031B 0323;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (Ḋ◌̛◌̣; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT ABOVE, COMBINING HORN, COMBINING DOT BELOW
        // conformance_test(vec![vec![0x1E0A, 0x031B, 0x0323], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]]);
        //
        // // 1E0C 031B 0307;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (Ḍ◌̛◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D WITH DOT BELOW, COMBINING HORN, COMBINING DOT ABOVE
        // conformance_test(vec![vec![0x1E0C, 0x031B, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]]);
        //
        // // 0044 031B 0307 0323;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (D◌̛◌̇◌̣; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING HORN, COMBINING DOT ABOVE, COMBINING DOT BELOW
        // conformance_test(vec![vec![0x0044, 0x031B, 0x0307, 0x0323], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]]);
        //
        // // 0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307;1E0C 031B 0307;0044 031B 0323 0307; # (D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; Ḍ◌̛◌̇; D◌̛◌̣◌̇; ) LATIN CAPITAL LETTER D, COMBINING HORN, COMBINING DOT BELOW, COMBINING DOT ABOVE
        // conformance_test(vec![vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307], vec![0x1E0C, 0x031B, 0x0307], vec![0x0044, 0x031B, 0x0323, 0x0307]]);
        //
        // // 00C8;00C8;0045 0300;00C8;0045 0300; # (È; È; E◌̀; È; E◌̀; ) LATIN CAPITAL LETTER E WITH GRAVE
        // conformance_test(vec![vec![0x00C8], vec![0x00C8], vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300]]);
        //
        // // 0112;0112;0045 0304;0112;0045 0304; # (Ē; Ē; E◌̄; Ē; E◌̄; ) LATIN CAPITAL LETTER E WITH MACRON
        // conformance_test(vec![vec![0x0112], vec![0x0112], vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304]]);
        //
        // // 0045 0300;00C8;0045 0300;00C8;0045 0300; # (E◌̀; È; E◌̀; È; E◌̀; ) LATIN CAPITAL LETTER E, COMBINING GRAVE ACCENT
        // conformance_test(vec![vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300], vec![0x00C8], vec![0x0045, 0x0300]]);
        //
        // // 0045 0304;0112;0045 0304;0112;0045 0304; # (E◌̄; Ē; E◌̄; Ē; E◌̄; ) LATIN CAPITAL LETTER E, COMBINING MACRON
        // conformance_test(vec![vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304], vec![0x0112], vec![0x0045, 0x0304]]);
        //
        // // 1E14;1E14;0045 0304 0300;1E14;0045 0304 0300; # (Ḕ; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E WITH MACRON AND GRAVE
        // conformance_test(vec![vec![0x1E14], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]]);
        //
        // // 0112 0300;1E14;0045 0304 0300;1E14;0045 0304 0300; # (Ē◌̀; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E WITH MACRON, COMBINING GRAVE ACCENT
        // conformance_test(vec![vec![0x0112, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]]);
        //
        // // 1E14 0304;1E14 0304;0045 0304 0300 0304;1E14 0304;0045 0304 0300 0304; # (Ḕ◌̄; Ḕ◌̄; E◌̄◌̀◌̄; Ḕ◌̄; E◌̄◌̀◌̄; ) LATIN CAPITAL LETTER E WITH MACRON AND GRAVE, COMBINING MACRON
        // conformance_test(vec![vec![0x1E14, 0x0304], vec![0x1E14, 0x0304], vec![0x0045, 0x0304, 0x0300, 0x0304], vec![0x1E14, 0x0304], vec![0x0045, 0x0304, 0x0300, 0x0304]]);
        //
        // // 0045 0304 0300;1E14;0045 0304 0300;1E14;0045 0304 0300; # (E◌̄◌̀; Ḕ; E◌̄◌̀; Ḕ; E◌̄◌̀; ) LATIN CAPITAL LETTER E, COMBINING MACRON, COMBINING GRAVE ACCENT
        // conformance_test(vec![vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300], vec![0x1E14], vec![0x0045, 0x0304, 0x0300]]);
        //
        // // 0045 0300 0304;00C8 0304;0045 0300 0304;00C8 0304;0045 0300 0304; # (E◌̀◌̄; È◌̄; E◌̀◌̄; È◌̄; E◌̀◌̄; ) LATIN CAPITAL LETTER E, COMBINING GRAVE ACCENT, COMBINING MACRON
        // conformance_test(vec![vec![0x0045, 0x0300, 0x0304], vec![0x00C8, 0x0304], vec![0x0045, 0x0300, 0x0304], vec![0x00C8, 0x0304], vec![0x0045, 0x0300, 0x0304]]);
        //
        // // 05B8 05B9 05B1 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F;05B1 05B8 05B9 0591 05C3 05B0 05AC 059F; # (◌ָ◌ֹ◌ֱ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ◌ֱ◌ָ◌ֹ◌֑׃◌ְ◌֬◌֟; ) HEBREW POINT QAMATS, HEBREW POINT HOLAM, HEBREW POINT HATAF SEGOL, HEBREW ACCENT ETNAHTA, HEBREW PUNCTUATION SOF PASUQ, HEBREW POINT SHEVA, HEBREW ACCENT ILUY, HEBREW ACCENT QARNEY PARA
        // conformance_test(vec![vec![0x05B8, 0x05B9, 0x05B1, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F], vec![0x05B1, 0x05B8, 0x05B9, 0x0591, 0x05C3, 0x05B0, 0x05AC, 0x059F]]);
        //
        // // 0592 05B7 05BC 05A5 05B0 05C0 05C4 05AD;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4;05B0 05B7 05BC 05A5 0592 05C0 05AD 05C4; # (◌֒◌ַ◌ּ◌֥◌ְ׀◌ׄ◌֭; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ◌ְ◌ַ◌ּ◌֥◌֒׀◌֭◌ׄ; ) HEBREW ACCENT SEGOL, HEBREW POINT PATAH, HEBREW POINT DAGESH OR MAPIQ, HEBREW ACCENT MERKHA, HEBREW POINT SHEVA, HEBREW PUNCTUATION PASEQ, HEBREW MARK UPPER DOT, HEBREW ACCENT DEHI
        // conformance_test(vec![vec![0x0592, 0x05B7, 0x05BC, 0x05A5, 0x05B0, 0x05C0, 0x05C4, 0x05AD], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4], vec![0x05B0, 0x05B7, 0x05BC, 0x05A5, 0x0592, 0x05C0, 0x05AD, 0x05C4]]);
        //
        // // 1100 AC00 11A8;1100 AC01;1100 1100 1161 11A8;1100 AC01;1100 1100 1161 11A8; # (ᄀ각; ᄀ각; ᄀ각; ᄀ각; ᄀ각; ) HANGUL CHOSEONG KIYEOK, HANGUL SYLLABLE GA, HANGUL JONGSEONG KIYEOK
        // conformance_test(vec![vec![0x1100, 0xAC00, 0x11A8], vec![0x1100, 0xAC01], vec![0x1100, 0x1100, 0x1161, 0x11A8], vec![0x1100, 0xAC01], vec![0x1100, 0x1100, 0x1161, 0x11A8]]);
        //
        // // 1100 AC00 11A8 11A8;1100 AC01 11A8;1100 1100 1161 11A8 11A8;1100 AC01 11A8;1100 1100 1161 11A8 11A8; # (ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ᄀ각ᆨ; ) HANGUL CHOSEONG KIYEOK, HANGUL SYLLABLE GA, HANGUL JONGSEONG KIYEOK, HANGUL JONGSEONG KIYEOK
        // conformance_test(vec![vec![0x1100, 0xAC00, 0x11A8, 0x11A8], vec![0x1100, 0xAC01, 0x11A8], vec![0x1100, 0x1100, 0x1161, 0x11A8, 0x11A8], vec![0x1100, 0xAC01, 0x11A8], vec![0x1100, 0x1100, 0x1161, 0x11A8, 0x11A8]]);
    }
}
