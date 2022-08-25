use crate::normalise::to_nfd;
use crate::trie::TrieMatch;
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
        let mut s: Vec<u32> = vec![nfd[pos]];
        // S2.1 Find the longest initial substring S at each point that has a match in the collation element table.
        if let Some(true) = nfd.get(pos + 1).map(|cp| is_starter(*cp)) {
            while let Some(cp) = nfd.get(pos + 1) {
                s.push(*cp);
                match collation_elements(&s) {
                    TrieMatch::Match(_) => {
                        nfd.remove(pos + 1);
                    }
                    TrieMatch::PartialMatch => {
                        todo!()
                    }
                    TrieMatch::NoMatch => {
                        s.pop();
                        break;
                    }
                }
            }
        }
        // S2.1.1 If there are any non-starters following S, process each non-starter C.
        // Try to consume a contiguous string of non-starters, allowing partial matches. If we
        // encounter a non-match, or a blocked char, then we reset, and try discontiguous matches.
        if let Some(false) = nfd.get(pos + 1).map(|cp| is_starter(*cp)) {
            let mut last_cc = 0;
            let mut offset = 1;
            let starting_s = s.clone();
            let mut mid_partial = false;
            while let Some(cp) = nfd.get(pos + offset) {
                let cc = combining_class(*cp);
                let unblocked_non_starter = !is_starter(*cp) && cc > last_cc;
                if unblocked_non_starter {
                    s.push(*cp);
                    match collation_elements(&s) {
                        TrieMatch::Match(_) => {
                            mid_partial = false;
                            offset += 1;
                        }
                        TrieMatch::PartialMatch => {
                            mid_partial = true;
                            offset += 1;
                        }
                        TrieMatch::NoMatch => {
                            s.pop();
                            break;
                        }
                    }
                    last_cc = cc;
                } else {
                    break;
                }
            }
            if mid_partial {
                s = starting_s;
            } else {
                pos += offset - 1;
            }
        }
        // See if there are any discontiguous matches.
        if let Some(false) = nfd.get(pos + 1).map(|cp| is_starter(*cp)) {
            let mut last_cc = 0;
            let mut offset = 1;
            while let Some(cp) = nfd.get(pos + offset) {
                let cc = combining_class(*cp);
                // S2.1.2 If C is an unblocked non-starter with respect to S, find if S + C has a match in the collation element table.
                let unblocked_non_starter = !is_starter(*cp) && cc > last_cc;
                if unblocked_non_starter {
                    s.push(*cp);
                    match collation_elements(&s) {
                        TrieMatch::Match(_) => {
                            // For this one, we want to rearrange it in the array, and we know
                            // that we don't need to reset it, because we've already handled
                            // possible partial matches.
                            nfd.remove(pos + offset);
                        }
                        TrieMatch::PartialMatch => {
                            s.pop();
                            offset += 1;
                        }
                        TrieMatch::NoMatch => {
                            s.pop();
                            offset += 1;
                        }
                    }
                } else {
                    break;
                }
                last_cc = cc;
            }
        }
        pos += 1;
        // https://perldoc.perl.org/Unicode::Collate#long_contraction
        // There's a comment there, which is the best explanation I've found of
        // this terrible, terrible spec.
        // S2.2 Fetch the corresponding collation element(s) from the table if there is a match. If
        // there is no match, synthesize a collation element as described in Section 10.1, Derived Collation Elements.
        let mut s_collation_elements = match collation_elements(&s) {
            TrieMatch::Match(es) => es,
            _ => derive_collation_elements(s),
        };
        // S2.3 Process collation elements according to the variable-weight setting, as described in Section 4, Variable Weighting.
        apply_variable_weighting(&mut s_collation_elements, variable_weighting);
        // S2.4 Append the collation element(s) to the collation element array.
        acc_collation_elements.extend(s_collation_elements);
        // S2.5 Proceed to the next point in the string (past S).
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
        0x17000..=0x18AFF => (0xFB00, (cp - 0x17000) | 0x8000),
        // # Tangut Supplement
        0x18D00..=0x18D8F => (0xFB00, (cp - 0x17000) | 0x8000),
        // # Nushu
        0x1B170..=0x1B2FF => (0xFB01, (cp - 0x1B170) | 0x8000),
        // # Khitan Small Script
        0x18B00..=0x18CFF => (0xFB02, (cp - 0x18B00) | 0x8000),
        // Unified_Ideograph=True AND ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
        0x4E00..=0x9FFF if unified_ideograph(*cp) => (0xFB40 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        0xF900..=0xFAFF if unified_ideograph(*cp) => (0xFB40 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        // Unified_Ideograph=True AND NOT ((Block=CJK_Unified_Ideograph) OR (Block=CJK_Compatibility_Ideographs))
        _ if unified_ideograph(*cp) => (0xFB80 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
        _ => (0xFBC0 + (cp >> 15), (cp & 0x7FFF) | 0x8000),
    };
    // [.AAAA.0020.0002][.BBBB.0000.0000]
    vec![
        CollationElement {
            weights: vec![aaaa as u16, 0x0020, 0x0002],
            variable: false,
        },
        CollationElement {
            weights: vec![bbbb as u16, 0x0000, 0x0000],
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
        // with stupid implementation, 14 seconds to test the first 1000 cases
        let mut i = 0;
        for (code_points, expected_sort_key) in load_test_cases() {
            if i > 10000 {
                break;
            }
            assert_eq!(
                sort_key(&code_points, &VariableWeighting::NonIgnorable),
                expected_sort_key
            );
            i += 1;
        }
    }
}
