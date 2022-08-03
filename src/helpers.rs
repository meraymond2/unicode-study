#[derive(Debug, PartialEq)]
pub enum CodeUnit {
    SingleByte,
    DoublePrefix,
    TriplePrefix,
    QuadPrefix,
    Continuation,
}

impl CodeUnit {
    pub fn len(&self) -> usize {
        match self {
            CodeUnit::SingleByte => 1,
            CodeUnit::DoublePrefix => 2,
            CodeUnit::TriplePrefix => 3,
            CodeUnit::QuadPrefix => 4,
            CodeUnit::Continuation => unreachable!(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum DecodeErr {
    IncompleteCharacter,
    InvalidCodePoint,
    InvalidCodeUnit,
    OverlongEncoding,
    UnexpectedContinuation,
}

impl TryFrom<u8> for CodeUnit {
    type Error = DecodeErr;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0..=0b0111_1111 => Ok(CodeUnit::SingleByte),
            0b1000_0000..=0b1011_1111 => Ok(CodeUnit::Continuation),
            0b1100_0000..=0b1101_1111 => Ok(CodeUnit::DoublePrefix),
            0b1110_0000..=0b1110_1111 => Ok(CodeUnit::TriplePrefix),
            0b1111_0000..=0b1111_0111 => Ok(CodeUnit::QuadPrefix),
            _ => Err(DecodeErr::InvalidCodeUnit)
        }
    }
}

pub fn is_valid_codepoint(code_point: u32) -> bool {
    // todo: non-characters
    let below_max_code_point = code_point <= 0x10FFFF;
    let not_half_of_utf16_surrogate_pair = code_point < 0xD800 || code_point > 0xDFFF;
    return below_max_code_point && not_half_of_utf16_surrogate_pair;
}


const CLEAR_12: u8 = 0b0011_1111;
const CLEAR_1234: u8 = 0b0000_1111;
const CLEAR_12345: u8 = 0b0000_0111;
const CLEAR_123456: u8 = 0b0000_0011;
const CLEAR_12378: u8 = 0b0001_1100;
const CLEAR_345678: u8 = 0b1100_0000;
const CLEAR_5678: u8 = 0b1111_0000;

pub fn decode_double(first: u8, second: u8) -> u32 {
    // 110a_aaaa 10bb_bbbb -> 0000_0aaa aabb_bbbb
    let high_byte = first >> 2 & CLEAR_12345;
    let low_byte = (first << 6 & CLEAR_345678) | (second & CLEAR_12);
    return u32::from_be_bytes([0, 0, high_byte, low_byte]);
}

pub fn decode_triple(first: u8, second: u8, third: u8) -> u32 {
    // 1110_aaaa 10bb_bbbb 10cc_cccc -> aaaa_bbbb bbcc_cccc
    let high_byte = (first << 4 & CLEAR_5678) | (second >> 2 & CLEAR_1234);
    let low_byte = (second << 6 & CLEAR_345678) | (third & CLEAR_12);
    return u32::from_be_bytes([0, 0, high_byte, low_byte]);
}

pub fn decode_quad(first: u8, second: u8, third: u8, fourth: u8) -> u32 {
    // 1111_0aaa 10bb_bbbb 10cc_cccc 10dd_dddd -> 000a_aabb bbbb_cccc ccdd_dddd
    let high_byte = (first << 2 & CLEAR_12378) | (second >> 4 & CLEAR_123456);
    let middle_byte = (second << 4 & CLEAR_5678) | (third >> 2 & CLEAR_1234);
    let low_byte = (third << 6 & CLEAR_345678) | (fourth & CLEAR_12);
    return u32::from_be_bytes([0, high_byte, middle_byte, low_byte]);
}


pub fn encode(code_point: u32) -> Vec<u8> {
    let bytes = code_point.to_be_bytes();
    match code_point {
        0..=0x007F => vec![code_point as u8],
        0x0080..=0x07FF => {
            // 0000_0aaa aabb_bbbb -> 110a_aaaa 10bb_bbbb
            let first_byte = 0b1100_0000 | (bytes[0] << 2) | (bytes[1] >> 6);
            let second_byte = bytes[1] | 0b1000_0000 & 0b1011_1111;
            vec![first_byte, second_byte]
        }
        0x0800..=0xFFFF => {
            // aaaa_bbbb bbcc_cccc -> 1110_aaaa 10bb_bbbb 10cc_cccc
            let first_byte = 0b1110_0000 | (bytes[0] >> 4);
            let second_byte = 0b1000_0000 | (bytes[0] << 2 & 0b00111100) | (bytes[1] >> 6);
            let third_byte = 0b1000_0000 | (bytes[1] & 0b0011_1111);
            vec![first_byte, second_byte, third_byte]
        }
        0x10000..=0x10FFFF => todo!(),
        _ => unimplemented!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codepoint_from_u8() {
        assert_eq!(CodeUnit::try_from(0), Ok(CodeUnit::SingleByte));
        assert_eq!(CodeUnit::try_from(127), Ok(CodeUnit::SingleByte));

        assert_eq!(CodeUnit::try_from(128), Ok(CodeUnit::Continuation));
        assert_eq!(CodeUnit::try_from(191), Ok(CodeUnit::Continuation));

        assert_eq!(CodeUnit::try_from(192), Ok(CodeUnit::DoublePrefix));
        assert_eq!(CodeUnit::try_from(223), Ok(CodeUnit::DoublePrefix));

        assert_eq!(CodeUnit::try_from(224), Ok(CodeUnit::TriplePrefix));
        assert_eq!(CodeUnit::try_from(239), Ok(CodeUnit::TriplePrefix));

        assert_eq!(CodeUnit::try_from(240), Ok(CodeUnit::QuadPrefix));
        assert_eq!(CodeUnit::try_from(247), Ok(CodeUnit::QuadPrefix));

        assert_eq!(CodeUnit::try_from(248), Err(DecodeErr::InvalidCodeUnit));
    }

    #[test]
    fn test_is_valid_codepoint() {
        assert!(is_valid_codepoint(0x0024));
        assert!(is_valid_codepoint(0x00A3));
        assert!(is_valid_codepoint(0x0939));
        assert!(is_valid_codepoint(0x20AC));
        assert!(is_valid_codepoint(0xD55C));
        assert!(is_valid_codepoint(0x10348));
        assert_eq!(is_valid_codepoint(0x110000), false);
        assert_eq!(is_valid_codepoint(0xD800), false);
        assert_eq!(is_valid_codepoint(0xDABC), false);
        assert_eq!(is_valid_codepoint(0xDFFF), false);
    }

    #[test]
    fn test_decode_double() {
        assert_eq!(decode_double(0b11000010, 0b10100011), 0xA3);
    }

    #[test]
    fn test_decode_triple() {
        assert_eq!(decode_triple(0xEF, 0xBF, 0xBD), 0xFFFD);
        assert_eq!(decode_triple(0b11100000, 0b10100100, 0b10111001), 0x939);
        assert_eq!(decode_triple(0b11100010, 0b10000010, 0b10101100), 0x20AC);
        assert_eq!(decode_triple(0b11101101, 0b10010101, 0b10011100), 0xD55C);
    }

    #[test]
    fn test_decode_quad() {
        assert_eq!(decode_quad(0b11110000, 0b10010000, 0b10001101, 0b10001000), 0x10348);
    }
}
