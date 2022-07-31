fn is_single(byte: u8) -> bool {
    // matches bit pattern 0xxx_xxxx
    return byte <= 0b0111_1111;
}

fn is_double(byte: u8) -> bool {
    // matches bit pattern 110x_xxxx
    return byte >= 0b1100_0000 && byte <= 0b1101_1111;
}

fn is_triple(byte: u8) -> bool {
    // matches bit pattern 1110_xxxx
    return byte >= 0b1110_0000 && byte <= 0b1110_1111;
}

fn is_quad(byte: u8) -> bool {
    // matches bit pattern 1111_0xxx
    return byte >= 0b1111_0000 && byte <= 0b1111_0111;
}

fn is_continuation(byte: u8) -> bool {
    // matches bit pattern 10xx_xxxx
    return byte >= 0b1000_0000 && byte <= 0b1011_1111;
}

fn is_valid_codepoint(code_point: u32) -> bool {
    // todo: non-characters
    let below_max_code_point = code_point <= 0x10FFFF;
    let not_half_of_utf16_surrogate_pair = code_point < 0xD800 || code_point > 0xDFFF;
    return below_max_code_point && not_half_of_utf16_surrogate_pair;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_single() {
        assert!(is_single(0));
        assert!(is_single(127));
        assert_eq!(is_single(128), false);
    }

    #[test]
    fn test_is_double() {
        assert!(is_double(192));
        assert!(is_double(223));
        assert_eq!(is_double(191), false);
        assert_eq!(is_double(224), false);
    }

    #[test]
    fn test_is_triple() {
        assert!(is_triple(224));
        assert!(is_triple(239));
        assert_eq!(is_triple(223), false);
        assert_eq!(is_triple(240), false);
    }

    #[test]
    fn test_is_quad() {
        assert!(is_quad(240));
        assert!(is_quad(247));
        assert_eq!(is_quad(239), false);
        assert_eq!(is_quad(248), false);
    }

    #[test]
    fn test_is_continuation() {
        assert!(is_continuation(128));
        assert!(is_continuation(191));
        assert_eq!(is_continuation(127), false);
        assert_eq!(is_continuation(192), false);
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
}
