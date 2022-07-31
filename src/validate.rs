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
                    match CodeUnit::try_from(input[pos + i])? {
                        CodeUnit::Continuation => {}
                        _ => { return Err(DecodeErr::IncompleteCharacter); }
                    }
                }
                let code_point = match code_unit {
                    CodeUnit::DoublePrefix => decode_double(input[pos], input[pos + 1]),
                    CodeUnit::TriplePrefix => decode_triple(input[pos], input[pos + 1], input[pos + 2]),
                    CodeUnit::QuadPrefix => decode_quad(input[pos], input[pos + 1], input[pos + 2], input[pos + 3]),
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
        // 1  Some correct UTF-8 text                                                    |
        // You should see the Greek word 'kosme':       "κόσμε"                          |
        let correct_one: Vec<u8> = vec![206, 186, 225, 189, 185, 207, 131, 206, 188, 206, 181];
        assert!(validate(&correct_one).is_ok());

        // 2  Boundary condition test cases                                              |
        // 2.1  First possible sequence of a certain length                              |
        // 2.1.1  1 byte  (U-00000000):        " "
        let correct_two_one_one: Vec<u8> = vec![0];
        assert!(validate(&correct_two_one_one).is_ok());
        // 2.1.2  2 bytes (U-00000080):        ""                                       |
        let correct_two_one_two: Vec<u8> = vec![194, 128];
        assert!(validate(&correct_two_one_two).is_ok());
        // 2.1.3  3 bytes (U-00000800):        "ࠀ"                                       |
        let correct_two_one_three: Vec<u8> = vec![224, 160, 128];
        assert!(validate(&correct_two_one_three).is_ok());
        // 2.1.4  4 bytes (U-00010000):        "𐀀"                                       |
        let correct_two_one_four: Vec<u8> = vec![240, 144, 128, 128];
        assert!(validate(&correct_two_one_four).is_ok());
        // 2.1.5  5 bytes (U-00200000):        "�����"                                       |
        let correct_two_one_five: Vec<u8> = vec![239, 191, 189, 239, 191, 189, 239, 191, 189, 239, 191, 189, 239, 191, 189];
        assert!(validate(&correct_two_one_five).is_ok());
        // 2.1.6  6 bytes (U-04000000):        "������"                                       |
        let correct_two_one_six: Vec<u8> = vec![239, 191, 189, 239, 191, 189, 239, 191, 189, 239, 191, 189, 239, 191, 189, 239, 191, 189];
        assert!(validate(&correct_two_one_six).is_ok());

        // 2.2  Last possible sequence of a certain length                               |
        // 2.2.1  1 byte  (U-0000007F):        ""
        let correct_two_two_one: Vec<u8> = vec![127];
        assert!(validate(&correct_two_two_one).is_ok());
        // 2.2.2  2 bytes (U-000007FF):        "߿"                                       |
        let correct_two_two_two: Vec<u8> = vec![223,191];
        assert!(validate(&correct_two_two_two).is_ok());
        // 2.2.3  3 bytes (U-0000FFFF):        "￿"                                       |
        let correct_two_two_three: Vec<u8> = vec![239,191,191];
        assert!(validate(&correct_two_two_three).is_ok());
        // 2.2.4  4 bytes (U-001FFFFF):        "����"                                       |
        let correct_two_two_four: Vec<u8> = vec![239,191,189,239,191,189,239,191,189,239,191,189];
        assert!(validate(&correct_two_two_four).is_ok());
        // 2.2.5  5 bytes (U-03FFFFFF):        "�����"                                       |
        let correct_two_two_five: Vec<u8> = vec![239,191,189,239,191,189,239,191,189,239,191,189,239,191,189];
        assert!(validate(&correct_two_two_five).is_ok());
        // 2.2.6  6 bytes (U-7FFFFFFF):        "������"                                       |
        let correct_two_two_six: Vec<u8> = vec![239,191,189,239,191,189,239,191,189,239,191,189,239,191,189,239,191,189];
        assert!(validate(&correct_two_two_six).is_ok());
    }
}
