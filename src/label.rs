//! Labels in the bottom half of the terminal UI that provide information based on cursor position.

#![allow(clippy::cast_possible_wrap)]

use std::fmt::Formatter;
use std::fmt::{self, Write};
use std::ops::Index;

pub(crate) static LABEL_TITLES: [&str; 16] = [
    "Signed 8 bit",
    "Unsigned 8 bit",
    "Signed 16 bit",
    "Unsigned 16 bit",
    "Signed 32 bit",
    "Unsigned 32 bit",
    "Signed 64 bit",
    "Unsigned 64 bit",
    "Hexadecimal",
    "Octal",
    "Binary",
    "Stream Length",
    "Float 32 bit",
    "Float 64 bit",
    "Offset",
    "Notifications",
];

#[derive(Default)]
pub(crate) enum Endianness {
    #[default]
    LittleEndian,
    BigEndian,
}

impl fmt::Display for Endianness {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Endianness::LittleEndian => write!(f, "Little Endian"),
            Endianness::BigEndian => write!(f, "Big Endian"),
        }
    }
}

#[derive(Default)]
pub struct Handler {
    signed_eight: String,
    signed_sixteen: String,
    signed_thirtytwo: String,
    signed_sixtyfour: String,
    unsigned_eight: String,
    unsigned_sixteen: String,
    unsigned_thirtytwo: String,
    unsigned_sixtyfour: String,
    float_thirtytwo: String,
    float_sixtyfour: String,
    binary: String,
    octal: String,
    hexadecimal: String,
    stream_length: usize,
    stream_length_string: String,
    pub(crate) offset: String,
    pub notification: String,
    pub(crate) endianness: Endianness,
}

impl Index<&str> for Handler {
    type Output = String;

    fn index(&self, index: &str) -> &Self::Output {
        match index {
            "Signed 8 bit" => &self.signed_eight,
            "Unsigned 8 bit" => &self.unsigned_eight,
            "Signed 16 bit" => &self.signed_sixteen,
            "Unsigned 16 bit" => &self.unsigned_sixteen,
            "Signed 32 bit" => &self.signed_thirtytwo,
            "Unsigned 32 bit" => &self.unsigned_thirtytwo,
            "Signed 64 bit" => &self.signed_sixtyfour,
            "Unsigned 64 bit" => &self.unsigned_sixtyfour,
            "Hexadecimal" => &self.hexadecimal,
            "Octal" => &self.octal,
            "Binary" => &self.binary,
            "Stream Length" => &self.stream_length_string,
            "Float 32 bit" => &self.float_thirtytwo,
            "Float 64 bit" => &self.float_sixtyfour,
            "Offset" => &self.offset,
            "Notifications" => &self.notification,
            _ => panic!(),
        }
    }
}

impl Handler {
    pub(crate) fn new(bytes: &[u8], offset: usize) -> Self {
        let mut labels = Self { ..Default::default() };
        labels.update_stream_length(8);
        labels.update_all(&bytes[offset..]);
        labels.offset = format!("{offset:#X?}");
        labels
    }
    pub(crate) fn update_all(&mut self, bytes: &[u8]) {
        let filled_bytes = fill_slice(bytes, 8);
        self.update_signed_eight(&filled_bytes[0..1]);
        self.update_signed_sixteen(&filled_bytes[0..2]);
        self.update_signed_thirtytwo(&filled_bytes[0..4]);
        self.update_signed_sixtyfour(&filled_bytes[0..8]);

        self.update_unsigned_eight(&filled_bytes[0..1]);
        self.update_unsigned_sixteen(&filled_bytes[0..2]);
        self.update_unsigned_thirtytwo(&filled_bytes[0..4]);
        self.update_unsigned_sixtyfour(&filled_bytes[0..8]);

        self.update_float_thirtytwo(&filled_bytes[0..4]);
        self.update_float_sixtyfour(&filled_bytes[0..8]);

        self.update_streams(bytes);
    }
    pub(crate) fn update_streams(&mut self, bytes: &[u8]) {
        let mut filled_bytes = fill_slice(bytes, self.stream_length / 8);
        let remaining_bits = self.stream_length % 8;
        if remaining_bits != 0 {
            let bits_to_clear = 8 - remaining_bits;
            filled_bytes.push(
                bytes.get(self.stream_length / 8).unwrap_or(&0) >> bits_to_clear << bits_to_clear,
            );
        }

        self.update_binary(&filled_bytes);
        self.update_octal(&filled_bytes);
        self.update_hexadecimal(&filled_bytes);
    }
    pub(crate) fn update_stream_length(&mut self, length: usize) {
        self.stream_length = length;
        self.stream_length_string = self.stream_length.to_string();
    }
    pub(crate) fn switch_endianness(&mut self) {
        self.endianness = match self.endianness {
            Endianness::LittleEndian => Endianness::BigEndian,
            Endianness::BigEndian => Endianness::LittleEndian,
        };
    }
    pub(crate) const fn get_stream_length(&self) -> usize {
        self.stream_length
    }
    fn update_signed_eight(&mut self, bytes: &[u8]) {
        self.signed_eight = (bytes[0] as i8).to_string();
    }
    fn update_signed_sixteen(&mut self, bytes: &[u8]) {
        self.signed_sixteen = match self.endianness {
            Endianness::LittleEndian => i16::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => i16::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_signed_thirtytwo(&mut self, bytes: &[u8]) {
        self.signed_thirtytwo = match self.endianness {
            Endianness::LittleEndian => i32::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => i32::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_signed_sixtyfour(&mut self, bytes: &[u8]) {
        self.signed_sixtyfour = match self.endianness {
            Endianness::LittleEndian => i64::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => i64::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_unsigned_eight(&mut self, bytes: &[u8]) {
        self.unsigned_eight = (bytes[0]).to_string();
    }
    fn update_unsigned_sixteen(&mut self, bytes: &[u8]) {
        self.unsigned_sixteen = match self.endianness {
            Endianness::LittleEndian => u16::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => u16::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_unsigned_thirtytwo(&mut self, bytes: &[u8]) {
        self.unsigned_thirtytwo = match self.endianness {
            Endianness::LittleEndian => u32::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => u32::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_unsigned_sixtyfour(&mut self, bytes: &[u8]) {
        self.unsigned_sixtyfour = match self.endianness {
            Endianness::LittleEndian => u64::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => u64::from_be_bytes(bytes.try_into().unwrap()),
        }
        .to_string();
    }
    fn update_float_thirtytwo(&mut self, bytes: &[u8]) {
        let value = match self.endianness {
            Endianness::LittleEndian => f32::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => f32::from_be_bytes(bytes.try_into().unwrap()),
        };
        self.float_thirtytwo = format!("{value:e}");
    }
    fn update_float_sixtyfour(&mut self, bytes: &[u8]) {
        let value = match self.endianness {
            Endianness::LittleEndian => f64::from_le_bytes(bytes.try_into().unwrap()),
            Endianness::BigEndian => f64::from_be_bytes(bytes.try_into().unwrap()),
        };
        self.float_sixtyfour = format!("{value:e}");
    }
    fn update_binary(&mut self, bytes: &[u8]) {
        self.binary = bytes
            .iter()
            .fold(String::new(), |mut binary, byte| {
                let _ = write!(&mut binary, "{byte:08b}");
                binary
            })
            .chars()
            .take(self.stream_length)
            .collect();
    }
    fn update_octal(&mut self, bytes: &[u8]) {
        self.octal =
            bytes.iter().map(|byte| format!("{byte:03o}")).collect::<Vec<String>>().join(" ");
    }
    fn update_hexadecimal(&mut self, bytes: &[u8]) {
        self.hexadecimal =
            bytes.iter().map(|byte| format!("{byte:02X}")).collect::<Vec<String>>().join(" ");
    }
}

fn fill_slice(bytes: &[u8], len: usize) -> Vec<u8> {
    if bytes.len() < len {
        let mut fill = vec![0; len];
        for (i, byte) in bytes.iter().enumerate() {
            fill[i] = *byte;
        }
        return fill;
    }
    bytes[0..len].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_label() {
        // Given a label handler with the content 'hello' and offset of 0
        let content = "hello".as_bytes();
        let mut label_handler = Handler::new(content, 0);
        // The binary label should contain the binary veresion of the first character
        assert!(label_handler.binary.eq("01101000"));

        // When the stream_length is changed to include 8 more binary digits,
        label_handler.stream_length = 16;
        label_handler.update_binary(content);

        // The second character should also be represented
        assert!(label_handler.binary.eq("0110100001100101"));
    }
}
