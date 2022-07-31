use crate::helpers::*;

pub enum DecodeErr {
    InvalidPrefix,
    UnexpectedContinuation,
    IncompleteCharacter,
    OverlongEncoding,
    InvalidCodePoint,
}

pub fn validate(input: &[u8]) -> Result<(), DecodeErr> {
    let mut pos = 0;
    let len = input.len();
    while pos < len {
        let first = input[pos];

        if is_single(first) {
            pos += 1;
        } else {
            let remaining = if is_double(first) { 1 } else if is_triple(first) { 2 } else if is_quad(first) { 3 } else { 9999 };
            if pos + remaining >= len {
                return Err(DecodeErr::IncompleteCharacter);
            }
            for i in 1..=remaining {
                if !is_continuation(input[pos + i]) {
                    return Err(DecodeErr::IncompleteCharacter);
                }
            }
            let code_point = match remaining {
                1 => decode_double(first, input[pos + 1]),
                2 => decode_triple(first, input[pos + 1], input[pos + 2]),
                3 => decode_quad(first, input[pos + 1], input[pos + 3], input[pos + 4]),
                _ => unreachable!()
            };
            if !is_valid_codepoint(code_point) {
                return Err(DecodeErr::InvalidCodePoint);
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
        let s = std::fs::read(std::path::Path::new("/home/michael/dev/jpp/citylots.json")).unwrap();
        let start = std::time::Instant::now();
        assert!(validate(&s).is_ok());
        let elapsed_time = start.elapsed();
        eprintln!("Running slow_function() took {} ms.", elapsed_time.as_millis()); // 130-150 ms
    }
}
