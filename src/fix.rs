use crate::helpers::*;
use crate::validate::validate;

const REPLACEMENT: &[u8] = &[0xEF, 0xBF, 0xBD];


pub fn fix(input: Vec<u8>) -> Vec<u8> {
    match validate(&input) {
        Ok(_) => input,
        Err(_) => {
            let mut fixed = Vec::with_capacity(input.len());
            let len = input.len();
            let mut pos = 0;

            while let Err((decode_err, err_pos)) = validate(&input[pos..len]) {
                // err_pos is relative to the slice we pass to validate, but we're indexing input from the beginning
                let err_pos = err_pos + pos;
                fixed.extend_from_slice(&input[pos..err_pos]);
                pos = err_pos;
                match decode_err {
                    DecodeErr::InvalidCodeUnit => {
                        fixed.extend_from_slice(REPLACEMENT);
                        pos += 1;
                    }
                    DecodeErr::IncompleteCharacter => {
                        fixed.extend_from_slice(REPLACEMENT);
                        let code_unit = CodeUnit::try_from(input[pos]).unwrap();
                        let expected_continuations = &input[(pos + 1)..(pos + code_unit.len())];
                        let end = expected_continuations.iter().position(|c_u| CodeUnit::try_from(*c_u) != Ok(CodeUnit::Continuation)).unwrap();
                        pos += 1 + end;
                    }
                    DecodeErr::InvalidCodePoint => {
                        fixed.extend_from_slice(REPLACEMENT);
                        let code_unit = CodeUnit::try_from(input[pos]).unwrap();
                        pos += code_unit.len();
                    }
                    DecodeErr::OverlongEncoding(code_point) => {
                        fixed.extend_from_slice(&encode(code_point));
                        let code_unit = CodeUnit::try_from(input[pos]).unwrap();
                        pos += code_unit.len();
                    }
                    DecodeErr::UnexpectedContinuation => { pos += 1; }
                }
            }
            fixed.extend_from_slice(&input[pos..len]);
            return fixed;
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix() {
        assert_eq!(fix(vec![0xc0, 0x80]), vec![0x0]);
        assert_eq!(fix(vec![0xc0, 0xAE]), vec![0x2E]);
        assert_eq!(fix(vec![0xF0, 0x80, 0x80, 0x41]), vec![0xEF, 0xBF, 0xBE, 0x41]);

        let xs = b"hello".to_vec();
        let ys = "hello".as_bytes();
        assert_eq!(fix(xs), ys);

        let xs = "ศไทย中华Việt Nam".as_bytes().to_vec();
        let ys = "ศไทย中华Việt Nam".as_bytes();
        assert_eq!(fix(xs), ys);

        let xs = b"Hello\xC2 There\xFF Goodbye".to_vec();
        let ys = "Hello\u{FFFD} There\u{FFFD} Goodbye".as_bytes();
        assert_eq!(fix(xs), ys);

        // let xs = b"Hello\xC0\x80 There\xE6\x83 Goodbye";
        // assert_eq!(
        //     fix(xs),
        //     String::from("Hello\u{FFFD}\u{FFFD} There\u{FFFD} Goodbye").as_bytes()
        // );
        //
        // let xs = b"\xF5foo\xF5\x80bar";
        // assert_eq!(
        //     fix(xs),
        //     String::from("\u{FFFD}foo\u{FFFD}\u{FFFD}bar").as_bytes()
        // );
        //
        // let xs = b"\xF1foo\xF1\x80bar\xF1\x80\x80baz";
        // assert_eq!(
        //     fix(xs),
        //     String::from("\u{FFFD}foo\u{FFFD}bar\u{FFFD}baz").as_bytes()
        // );
        //
        // let xs = b"\xF4foo\xF4\x80bar\xF4\xBFbaz";
        // assert_eq!(
        //     fix(xs),
        //     String::from("\u{FFFD}foo\u{FFFD}bar\u{FFFD}\u{FFFD}baz").as_bytes()
        // );
        //
        // let xs = b"\xF0\x80\x80\x80foo\xF0\x90\x80\x80bar";
        // assert_eq!(
        //     fix(xs),
        //     String::from("\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}foo\u{10000}bar").as_bytes()
        // );
        //
        // // surrogates
        // let xs = b"\xED\xA0\x80foo\xED\xBF\xBFbar";
        // assert_eq!(
        //     fix(xs),
        //     String::from("\u{FFFD}\u{FFFD}\u{FFFD}foo\u{FFFD}\u{FFFD}\u{FFFD}bar").as_bytes()
        // );
    }
}
