use crate::normalise::to_nfd;
use crate::ucd::{combining_class, is_starter};
use std::cmp::min;

// https://unicode.org/reports/tr10/#Main_Algorithm
// Normalize each input string.
// Produce an array of collation elements for each string.
// Produce a sort key for each string from the arrays of collation elements.
// Compare the two sort keys with a binary comparison operation.
pub fn collate(code_points: &Vec<u32>) -> Vec<u32> {
    let mut nfd = to_nfd(code_points);
    // let mut collation_elements = Vec::new();
    // // S2.1 Find the longest initial substring S at each point that has a match in the collation element table.
    // let mut pos = 0;
    // while pos < nfd.len() {
    //     let s_end = nfd[pos..]
    //         .into_iter()
    //         .skip(1) // skip the current starter
    //         .position(|cp| is_starter(*cp)) // find the next starter, idx is from pos.skip(1) not pos
    //         .map(|offset| min(offset + 1, nfd.len() - pos)) // add one for beginning
    //         .unwrap_or(nfd.len() - pos); // if no starters left in string, return the end
    // }
    Vec::new()
}
