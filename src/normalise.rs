use crate::cp_iter::CodePointIter;
use crate::ucd::{canonical_class, nfc_is_allowed, QuickCheckVal};

#[derive(Debug, PartialEq)]
pub enum IsNormalised {
    Yes,
    No,
    Maybe,
}

pub fn quick_check(bytes: Vec<u8>) -> IsNormalised {
    let mut code_points = CodePointIter::new(bytes);
    let mut last_canonical_class: u8 = 0;
    let mut result: IsNormalised = IsNormalised::Yes;
    for code_point in code_points.into_iter() {
        let ccc = canonical_class(code_point);
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
}
