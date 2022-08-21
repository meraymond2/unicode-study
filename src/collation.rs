use crate::normalise::to_nfd;
use crate::ucd::{
    collation_elements, combining_class, is_starter, unified_ideograph, CollationElement,
};

// https://unicode.org/reports/tr10/#Main_Algorithm
// Normalize each input string.
// Produce an array of collation elements for each string.
// Produce a sort key for each string from the arrays of collation elements.
// Compare the two sort keys with a binary comparison operation.
pub fn sort_key(code_points: &Vec<u32>, variable_weighting: &VariableWeighting) -> Vec<u16> {
    let mut nfd = to_nfd(code_points);
    let collation_elements = to_collation_elements(&mut nfd, variable_weighting);
    to_sort_key(collation_elements)
}

fn to_collation_elements(
    nfd: &mut Vec<u32>,
    variable_weighting: &VariableWeighting,
) -> Vec<CollationElement> {
    let mut acc_collation_elements = Vec::new();
    let mut pos = 0;
    while pos < nfd.len() {
        // S2.1 Find the longest initial substring S at each point that has a match in the collation element table.
        // S is either a series of contiguous code points, or a starter and zero or more (dis)contiguous non-starters.
        let mut s: Vec<u32> = vec![nfd[pos]];
        match nfd.get(pos + 1).map(|cp| is_starter(*cp)) {
            None => {}
            Some(true) => {
                while let Some(cp) = nfd.get(pos + 1) {
                    s.push(*cp);
                    if collation_elements(&s).is_some() {
                        nfd.remove(pos + 1);
                    } else {
                        s.pop();
                        break;
                    }
                }
            }
            Some(false) => {
                // S2.1.1 If there are any non-starters following S, process each non-starter C.
                let mut last_cc = 0;
                let mut offset = pos + 1;
                while let Some(cp) = nfd.get(pos + offset) {
                    // S2.1.2 If C is an unblocked non-starter with respect to S, find if S + C has a match in the collation element table.
                    let cc = combining_class(*cp);
                    let unblocked_non_starter = !is_starter(*cp) && cc <= last_cc;
                    if unblocked_non_starter {
                        s.push(*cp);
                        // S2.1.3 If there is a match, replace S by S + C, and remove C.
                        if collation_elements(&s).is_some() {
                            nfd.remove(pos + offset);
                        } else {
                            s.pop();
                            offset += 1;
                        }
                    } else {
                        break;
                    }
                    last_cc = cc;
                }
            }
        }
        // S2.2 Fetch the corresponding collation element(s) from the table if there is a match. If
        // there is no match, synthesize a collation element as described in Section 10.1, Derived Collation Elements.
        let mut s_collation_elements =
            collation_elements(&s).unwrap_or(derive_collation_elements(s));
        // S2.3 Process collation elements according to the variable-weight setting, as described in Section 4, Variable Weighting.
        apply_variable_weighting(&mut s_collation_elements, variable_weighting);
        // S2.4 Append the collation element(s) to the collation element array.
        acc_collation_elements.extend(s_collation_elements);
        // S2.5 Proceed to the next point in the string (past S).
        pos += 1;
    }
    acc_collation_elements
}

fn to_sort_key(ces: Vec<CollationElement>) -> Vec<u16> {
    let weight_count = ces[0].weights.len();
    let level_separator = 0;
    let mut sort_key = Vec::new();
    for level in 0..weight_count {
        for ce in ces.iter() {
            let weight = ce.weights[level];
            if weight > 0 {
                sort_key.push(weight);
            }
        }
        sort_key.push(level_separator);
    }
    // sort_key.pop(); // remove trailing separator // undo, test doesn't like this
    sort_key
}

pub enum VariableWeighting {
    NonIgnorable, // sort punctuation as distinct chars
    Blanked,      // ignore punctuation
    Shifted,
    ShiftTrimmed,
}

fn derive_collation_elements(s: Vec<u32>) -> Vec<CollationElement> {
    let cp = s.first().unwrap();
    let (aaaa, bbbb) = match cp {
        // # Tangut and Tangut Components
        0x17000..=0x18AFF => (0xFB00, cp | 0x8000),
        // # Tangut Supplement
        0x18D00..=0x18D8F => (0xFB00, cp | 0x8000),
        // # Nushu
        0x1B170..=0x1B2FF => (0xFB01, cp | 0x8000),
        // # Khitan Small Script
        0x18B00..=0x18CFF => (0xFB02, cp | 0x8000),
        // Unified_Ideograph=True AND ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
        0x4E00..=0x9FFF if unified_ideograph(*cp) => (0xFB40 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        0xF900..=0xFAFF if unified_ideograph(*cp) => (0xFB40 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        // Unified_Ideograph=True AND NOT ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
        _ if unified_ideograph(*cp) => (0xFB80 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        _ => (0xFBC0 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
    };
    // [.AAAA.0020.0002][.BBBB.0000.0000]
    let a_bytes = aaaa.to_be_bytes();
    let a1 = u16::from_be_bytes([a_bytes[0], a_bytes[1]]);
    let a2 = u16::from_be_bytes([a_bytes[2], a_bytes[3]]);
    let b_bytes = bbbb.to_be_bytes();
    let b1 = u16::from_be_bytes([b_bytes[0], b_bytes[1]]);
    let b2 = u16::from_be_bytes([b_bytes[2], b_bytes[3]]);
    vec![
        CollationElement {
            weights: vec![a1, a2, 0x00, 0x20, 0x00, 0x02],
            variable: false,
        },
        CollationElement {
            weights: vec![b1, b2, 0x00, 0x00, 0x00, 0x00],
            variable: false,
        },
    ]
}

fn apply_variable_weighting(
    ces: &mut Vec<CollationElement>,
    variable_weighting: &VariableWeighting,
) {
    match variable_weighting {
        VariableWeighting::NonIgnorable => {}
        VariableWeighting::Blanked => {
            let mut blanking = false;
            for ce in ces.iter_mut() {
                if ce.variable {
                    ce.weights = vec![0; ce.weights.len()];
                    blanking = true;
                } else if blanking && ignorable(ce) {
                    ce.weights = vec![0; ce.weights.len()];
                } else {
                    blanking = false;
                }
            }
        }
        VariableWeighting::Shifted => todo!(),
        VariableWeighting::ShiftTrimmed => todo!(),
    }
}

// UTS10-D14. Ignorable Collation Element: A collation element which is not a primary collation element.
fn ignorable(ce: &CollationElement) -> bool {
    ce.weights[0] == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_line(line: &str) -> (Vec<u32>, Vec<u16>) {
        let cps: Vec<u32> = line
            .split_once(";")
            .unwrap()
            .0
            .split_whitespace()
            .map(|s| u32::from_str_radix(s, 16).unwrap())
            .collect();
        let second_split: Vec<&str> = line.split("[").collect();
        let mut sort_keys_str = second_split.last().unwrap().to_string();
        sort_keys_str.pop(); // pop "]"
        let sort_keys = sort_keys_str
            .replace("|", "0000")
            .split_whitespace()
            .map(|s| u16::from_str_radix(s, 16).unwrap())
            .collect();
        (cps, sort_keys)
    }

    fn load_test_cases() -> Vec<(Vec<u32>, Vec<u16>)> {
        std::fs::read_to_string(std::path::Path::new(
            "resources/CollationTest_NON_IGNORABLE.txt",
        ))
        .unwrap()
        .split("\n")
        .filter(|line| !line.is_empty() && !line.starts_with("#"))
        .map(parse_line)
        .collect()
    }

    #[test]
    fn test_sort_key() {
        let mut i = 0;
        for (code_points, expected_sort_key) in load_test_cases() {
            assert_eq!(
                sort_key(&code_points, &VariableWeighting::NonIgnorable),
                expected_sort_key
            );
            i += 1;
            // println!("{}", i);
        }
    }
}
