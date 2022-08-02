use crate::helpers::*;
use crate::validate::validate;

const REPLACEMENT: &[u8] = &[0xEF, 0xBF, 0xBE];

fn encode(code_point: u32) -> Vec<u8> {
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


pub fn fix(input: Vec<u8>) -> Vec<u8> {
    match validate(&input) {
        Ok(_) => input,
        Err(_) => {
            let mut fixed = Vec::with_capacity(input.len());
            let len = input.len();
            let mut pos = 0;

            while let Err((decode_err, err_pos)) = validate(&input[pos..len]) {
                fixed.extend_from_slice(&input[pos..err_pos]);
                pos = err_pos;
                match decode_err {
                    DecodeErr::InvalidCodeUnit => { pos += 1; }
                    DecodeErr::IncompleteCharacter => {
                        let code_unit = CodeUnit::try_from(input[pos]).unwrap();
                        let expected_continuations = &input[(pos + 1)..(pos + code_unit.len())];
                        let end = expected_continuations.iter().position(|c_u| CodeUnit::try_from(*c_u) != Ok(CodeUnit::Continuation)).unwrap();
                        fixed.extend_from_slice(REPLACEMENT);
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
    }
}
