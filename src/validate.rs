use crate::helpers::*;

fn bytes_remaining(code_unit: &CodeUnit) -> usize {
    match code_unit {
        CodeUnit::SingleByte => 0,
        CodeUnit::DoublePrefix => 1,
        CodeUnit::TriplePrefix => 2,
        CodeUnit::QuadPrefix => 3,
        _ => unreachable!(),
    }
}

fn is_overlong(code_point: u32, code_unit: &CodeUnit) -> bool {
    match code_unit {
        CodeUnit::SingleByte => false,
        CodeUnit::DoublePrefix => code_point < 0x0080,
        CodeUnit::TriplePrefix => code_point < 0x0800,
        CodeUnit::QuadPrefix => code_point < 0x10000,
        CodeUnit::Continuation => unreachable!(),
    }
}

pub fn validate(input: &[u8]) -> Result<(), (DecodeErr, usize)> {
    let mut pos = 0;
    let len = input.len();
    while pos < len {
        let code_unit = CodeUnit::try_from(input[pos]).map_err(|de| (de, pos))?;
        match code_unit {
            CodeUnit::SingleByte => {
                pos += 1;
            }
            CodeUnit::Continuation => {
                return Err((DecodeErr::UnexpectedContinuation, pos));
            }
            _ => {
                let remaining = bytes_remaining(&code_unit);
                if pos + remaining >= len {
                    return Err((DecodeErr::IncompleteCharacter, pos));
                }
                for i in 1..=remaining {
                    match CodeUnit::try_from(input[pos + i]) {
                        Ok(CodeUnit::Continuation) => {}
                        _ => {
                            return Err((DecodeErr::IncompleteCharacter, pos));
                        }
                    }
                }
                let code_point = match code_unit {
                    CodeUnit::DoublePrefix => decode_double(input[pos], input[pos + 1]),
                    CodeUnit::TriplePrefix => {
                        decode_triple(input[pos], input[pos + 1], input[pos + 2])
                    }
                    CodeUnit::QuadPrefix => {
                        decode_quad(input[pos], input[pos + 1], input[pos + 2], input[pos + 3])
                    }
                    _ => unreachable!(),
                };
                if !is_valid_codepoint(code_point) {
                    return Err((DecodeErr::InvalidCodePoint, pos));
                }
                if is_overlong(code_point, &code_unit) {
                    return Err((DecodeErr::OverlongEncoding, pos));
                }
                pos += 1 + remaining;
            }
        }
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        // https://github.com/rust-lang/rust/blob/master/library/alloc/tests/str.rs
        let xs = b"hello";
        assert!(validate(xs).is_ok());
        let xs = "ศไทย中华Việt Nam".as_bytes();
        assert!(validate(xs).is_ok());
        let xs = b"hello\xFF";
        assert!(validate(xs).is_err());

        assert_eq!(
            validate(&[0xF0, 0x80, 0x80, 0x41]),
            Err((DecodeErr::IncompleteCharacter, 0))
        );
        assert_eq!(
            validate(&[0xC2, 0x41, 0x42]),
            Err((DecodeErr::IncompleteCharacter, 0))
        );

        // invalid prefix
        assert!((validate(&[0x80]).is_err()));
        // invalid 2 byte prefix
        assert!((validate(&[0xc0]).is_err()));
        assert!((validate(&[0xc0, 0x10]).is_err()));
        // invalid 3 byte prefix
        assert!((validate(&[0xe0]).is_err()));
        assert!((validate(&[0xe0, 0x10]).is_err()));
        assert!((validate(&[0xe0, 0xff, 0x10]).is_err()));
        // invalid 4 byte prefix
        assert!((validate(&[0xf0]).is_err()));
        assert!((validate(&[0xf0, 0x10]).is_err()));
        assert!((validate(&[0xf0, 0xff, 0x10]).is_err()));
        assert!((validate(&[0xf0, 0xff, 0xff, 0x10]).is_err()));

        // deny overlong encodings
        assert!(validate(&[0xc0, 0x80]).is_err());
        assert!(validate(&[0xc0, 0xae]).is_err());
        assert!(validate(&[0xe0, 0x80, 0x80]).is_err());
        assert!(validate(&[0xe0, 0x80, 0xaf]).is_err());
        assert!(validate(&[0xe0, 0x81, 0x81]).is_err());
        assert!(validate(&[0xf0, 0x82, 0x82, 0xac]).is_err());
        assert!(validate(&[0xf4, 0x90, 0x80, 0x80]).is_err());

        // deny surrogates
        assert!(validate(&[0xED, 0xA0, 0x80]).is_err());
        assert!(validate(&[0xED, 0xBF, 0xBF]).is_err());

        assert!(validate(&[0xC2, 0x80]).is_ok());
        assert!(validate(&[0xDF, 0xBF]).is_ok());
        assert!(validate(&[0xE0, 0xA0, 0x80]).is_ok());
        assert!(validate(&[0xED, 0x9F, 0xBF]).is_ok());
        assert!(validate(&[0xEE, 0x80, 0x80]).is_ok());
        assert!(validate(&[0xEF, 0xBF, 0xBF]).is_ok());
        assert!(validate(&[0xF0, 0x90, 0x80, 0x80]).is_ok());
        assert!(validate(&[0xF4, 0x8F, 0xBF, 0xBF]).is_ok());
    }
}
