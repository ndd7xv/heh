#![allow(clippy::cast_possible_wrap)]
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
pub(crate) struct LabelHandler {
    pub(crate) signed_eight: String,
    pub(crate) signed_sixteen: String,
    pub(crate) signed_thirtytwo: String,
    pub(crate) signed_sixtyfour: String,
    pub(crate) unsigned_eight: String,
    pub(crate) unsigned_sixteen: String,
    pub(crate) unsigned_thirtytwo: String,
    pub(crate) unsigned_sixtyfour: String,
    pub(crate) float_thirtytwo: String,
    pub(crate) float_sixtyfour: String,
    pub(crate) binary: String,
    pub(crate) octal: String,
    pub(crate) hexadecimal: String,
    stream_length: usize,
    stream_length_string: String,
    pub(crate) offset: String,
    pub(crate) notification: String,
}

impl Index<&str> for LabelHandler {
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

impl LabelHandler {
    pub(crate) fn new(bytes: &[u8]) -> Self {
        let mut labels = LabelHandler { ..Default::default() };
        labels.update_stream_length(8);
        labels.update_all(bytes);
        labels.offset = String::from("0x0");
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
    pub(crate) fn get_stream_length(&self) -> usize {
        self.stream_length
    }
    fn update_signed_eight(&mut self, bytes: &[u8]) {
        self.signed_eight = (bytes[0] as i8).to_string();
    }
    fn update_signed_sixteen(&mut self, bytes: &[u8]) {
        self.signed_sixteen = i16::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_signed_thirtytwo(&mut self, bytes: &[u8]) {
        self.signed_thirtytwo = i32::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_signed_sixtyfour(&mut self, bytes: &[u8]) {
        self.signed_sixtyfour = i64::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_unsigned_eight(&mut self, bytes: &[u8]) {
        self.unsigned_eight = (bytes[0] as u8).to_string();
    }
    fn update_unsigned_sixteen(&mut self, bytes: &[u8]) {
        self.unsigned_sixteen = u16::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_unsigned_thirtytwo(&mut self, bytes: &[u8]) {
        self.unsigned_thirtytwo = u32::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_unsigned_sixtyfour(&mut self, bytes: &[u8]) {
        self.unsigned_sixtyfour = u64::from_le_bytes(bytes.try_into().unwrap()).to_string();
    }
    fn update_float_thirtytwo(&mut self, bytes: &[u8]) {
        self.float_thirtytwo = format!("{:e}", f32::from_le_bytes(bytes.try_into().unwrap()));
    }
    fn update_float_sixtyfour(&mut self, bytes: &[u8]) {
        self.float_sixtyfour = format!("{:e}", f64::from_le_bytes(bytes.try_into().unwrap()));
    }
    fn update_binary(&mut self, bytes: &[u8]) {
        self.binary = bytes
            .iter()
            .map(|byte| format!("{byte:08b}"))
            .collect::<String>()
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
