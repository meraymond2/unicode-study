use crate::helpers::{CodeUnit, decode_double, decode_quad, decode_triple};

pub struct CodePointIter {
    bytes: Vec<u8>,
    pos: usize,
}

impl CodePointIter {
    pub fn new(bytes: Vec<u8>) -> Self {
        CodePointIter { bytes, pos: 0 }
    }
}

impl Iterator for CodePointIter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.bytes.len() {
            None
        } else {
            match CodeUnit::try_from(self.bytes[self.pos]).unwrap() {
                CodeUnit::SingleByte => {
                    let code_point = self.bytes[self.pos];
                    self.pos += 1;
                    Some(code_point as u32)
                }
                CodeUnit::DoublePrefix => {
                    let code_point = decode_double(self.bytes[self.pos], self.bytes[self.pos + 1]);
                    self.pos += 2;
                    Some(code_point)
                }
                CodeUnit::TriplePrefix => {
                    let code_point = decode_triple(self.bytes[self.pos], self.bytes[self.pos + 1], self.bytes[self.pos + 2]);
                    self.pos += 3;
                    Some(code_point)
                }
                CodeUnit::QuadPrefix => {
                    let code_point = decode_quad(self.bytes[self.pos], self.bytes[self.pos + 1], self.bytes[self.pos + 2], self.bytes[self.pos + 3]);
                    self.pos += 4;
                    Some(code_point)
                }
                CodeUnit::Continuation => unreachable!()
            }
        }
    }
}