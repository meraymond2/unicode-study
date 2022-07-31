use crate::helpers::*;

fn bytes_remaining(code_unit: &CodeUnit) -> usize {
    match code_unit {
        CodeUnit::SingleByte => 0,
        CodeUnit::DoublePrefix => 1,
        CodeUnit::TriplePrefix => 2,
        CodeUnit::QuadPrefix => 3,
        _ => unreachable!()
    }
}

pub fn validate(input: &[u8]) -> Result<(), DecodeErr> {
    let mut pos = 0;
    let len = input.len();
    while pos < len {
        let code_unit = CodeUnit::try_from(input[pos])?;
        match code_unit {
            CodeUnit::SingleByte => { pos += 1; }
            CodeUnit::Continuation => { return Err(DecodeErr::UnexpectedContinuation); }
            _ => {
                let remaining = bytes_remaining(&code_unit);
                if pos + remaining >= len {
                    return Err(DecodeErr::IncompleteCharacter);
                }
                for i in 1..=remaining {
                    if let Ok(CodeUnit::Continuation) = CodeUnit::try_from(input[pos + 1]) {
                        return Err(DecodeErr::IncompleteCharacter);
                    }
                }
                let code_point = match code_unit {
                    CodeUnit::DoublePrefix => decode_double(input[pos], input[pos + 1]),
                    CodeUnit::TriplePrefix => decode_triple(input[pos], input[pos + 1], input[pos + 2]),
                    CodeUnit::QuadPrefix => decode_quad(input[pos], input[pos + 1], input[pos + 3], input[pos + 4]),
                    _ => unreachable!()
                };
                if !is_valid_codepoint(code_point) {
                    return Err(DecodeErr::InvalidCodePoint);
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
        let s = std::fs::read(std::path::Path::new("/home/michael/dev/jpp/citylots.json")).unwrap();
        let start = std::time::Instant::now();
        assert!(validate(&s).is_ok());
        let elapsed_time = start.elapsed();
        eprintln!("Running slow_function() took {} ms.", elapsed_time.as_millis()); // 130-150 ms
    }
}
