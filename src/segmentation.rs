// Implementation of default, non-locale specific grapheme cluster boundaries.

use crate::ucd::{grapheme_cluster_break, GraphemeClusterBreak};

pub struct GraphemeIter<'a> {
    code_points: &'a Vec<u32>,
    pos: usize,
}

impl<'a> GraphemeIter<'a> {
    pub fn new(code_points: &'a Vec<u32>) -> Self {
        GraphemeIter {
            code_points,
            pos: 0,
        }
    }
}

impl<'a> Iterator for GraphemeIter<'a> {
    type Item = &'a [u32];

    // https://unicode.org/reports/tr29/#Grapheme_Cluster_Boundary_Rules
    // At each char, we decide whether to break or keep going.
    fn next(&mut self) -> Option<Self::Item> {
        use GraphemeClusterBreak::*;
        // It's done.
        if self.pos >= self.code_points.len() {
            return None;
        }
        // At the last char, so just return that.
        if self.pos == self.code_points.len() - 1 {
            let start = self.pos;
            self.pos += 1;
            return Some(&self.code_points[start..]);
        }
        let start = self.pos;
        let mut ri_count = 0;
        while self.pos < self.code_points.len() - 1 {
            let cp = grapheme_cluster_break(self.code_points[self.pos]);
            let next = grapheme_cluster_break(self.code_points[self.pos + 1]);
            ri_count = if cp == RI { ri_count + 1 } else { 0 };
            match (cp, next) {
                (CR, LF) => self.pos += 1,                      // GB3
                (CN, _) => break,                               // GB4
                (CR, _) => break,                               // GB4
                (LF, _) => break,                               // GB4
                (_, CN) => break,                               // GB5
                (_, CR) => break,                               // GB5
                (_, LF) => break,                               // GB5
                (L, L) => self.pos += 1,                        // GB6
                (L, V) => self.pos += 1,                        // GB6
                (L, LV) => self.pos += 1,                       // GB6
                (L, LVT) => self.pos += 1,                      // GB6
                (LV, V) => self.pos += 1,                       // GB7
                (LV, T) => self.pos += 1,                       // GB7
                (V, V) => self.pos += 1,                        // GB7
                (V, T) => self.pos += 1,                        // GB7
                (LVT, T) => self.pos += 1,                      // GB8
                (T, T) => self.pos += 1,                        // GB8
                (_, EX) => self.pos += 1,                      // GB9
                (_, ZWJ) => self.pos += 1,                      // GB9
                (_, SM) => self.pos += 1,                       // GB9a
                (PP, _) => self.pos += 1,                       // GB9b
                (RI, RI) if ri_count % 2 == 0 => break,         // GB12/3
                (RI, RI) if ri_count % 2 == 1 => self.pos += 1, // GB12/3
                _ => break,
            }
        }
        self.pos += 1;
        return Some(&self.code_points[start..self.pos]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // https://www.unicode.org/Public/UCD/latest/ucd/auxiliary/GraphemeBreakTest.txt
    // The test cases are already split, but we can test the code by concatentating them, and then
    // checking that my split is the same as the original.
    // ÷ 0020 ÷ 0020 ÷	#  ÷ [0.2] SPACE (Other) ÷ [999.0] SPACE (Other) ÷ [0.3]
    fn parse_line(line: &str) -> Vec<Vec<u32>> {
        line.split_once("#")
            .unwrap()
            .0
            .trim()
            .split("÷")
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.trim()
                    .split("×")
                    .map(|cp| u32::from_str_radix(cp.trim(), 16).unwrap())
                    .collect()
            })
            .collect()
    }

    fn load_test_cases() -> Vec<Vec<Vec<u32>>> {
        std::fs::read_to_string(std::path::Path::new("resources/GraphemeBreakTest.txt"))
            .unwrap()
            .split("\n")
            .filter(|line| !line.is_empty() && !line.starts_with("#"))
            .map(parse_line)
            .collect()
    }

    #[test]
    fn test_grapheme_iter() {
        for expected in load_test_cases() {
            let to_split = expected.concat();
            let actual: Vec<&[u32]> = GraphemeIter::new(&to_split).into_iter().collect();
            assert_eq!(&actual, &expected);
        }
    }
}
