//! Decoder utilities.

use std::str::from_utf8;

use crate::character::{Category, RichChar, Type, CHARACTER_FILL, CHARACTER_UNKNOWN};

struct LossyASCIIDecoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> From<&'a [u8]> for LossyASCIIDecoder<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        Self { bytes, cursor: 0 }
    }
}

impl<'a> Iterator for LossyASCIIDecoder<'a> {
    type Item = (char, Type);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.bytes.len() {
            let byte = self.bytes[self.cursor];
            self.cursor += 1;
            if byte.is_ascii() {
                Some((byte as char, Type::Ascii))
            } else {
                Some((CHARACTER_UNKNOWN, Type::Unknown))
            }
        } else {
            None
        }
    }
}

struct LossyUTF8Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> From<&'a [u8]> for LossyUTF8Decoder<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        LossyUTF8Decoder { bytes, cursor: 0 }
    }
}

impl<'a> Iterator for LossyUTF8Decoder<'a> {
    type Item = (char, Type);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.bytes.len() {
            let typ = match self.bytes[self.cursor] {
                0x00..=0x7F => Type::Ascii,
                0xC0..=0xDF => Type::Unicode(2),
                0xE0..=0xEF => Type::Unicode(3),
                0xF0..=0xF7 => Type::Unicode(4),
                _ => {
                    self.cursor += 1;
                    return Some((CHARACTER_UNKNOWN, Type::Unknown));
                }
            };

            let new_cursor = self.bytes.len().min(self.cursor + typ.size());
            let chunk = &self.bytes[self.cursor..new_cursor];

            if let Ok(mut chars) = from_utf8(chunk).map(str::chars) {
                let char = chars.next().expect("the string must contain exactly one character");
                debug_assert!(
                    chars.next().is_none(),
                    "the string must contain exactly one character"
                );
                self.cursor += typ.size();
                Some((char, typ))
            } else {
                self.cursor += 1;
                Some((CHARACTER_UNKNOWN, Type::Unknown))
            }
        } else {
            None
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Encoding {
    Ascii,
    Utf8,
}

pub(crate) struct ByteAlignedDecoder<D: Iterator<Item = (char, Type)>> {
    decoder: D,
    to_fill: usize,
}

type BoxedDecoder<'a> = Box<dyn Iterator<Item = (char, Type)> + 'a>;

impl<'a> ByteAlignedDecoder<BoxedDecoder<'a>> {
    pub(crate) fn new(bytes: &'a [u8], encoding: Encoding) -> Self {
        match encoding {
            Encoding::Ascii => Box::new(LossyASCIIDecoder::from(bytes)) as BoxedDecoder,
            Encoding::Utf8 => Box::new(LossyUTF8Decoder::from(bytes)) as BoxedDecoder,
        }
        .into()
    }
}

impl<D: Iterator<Item = (char, Type)>> From<D> for ByteAlignedDecoder<D> {
    fn from(decoder: D) -> Self {
        Self { decoder, to_fill: 0 }
    }
}

impl<D: Iterator<Item = (char, Type)>> Iterator for ByteAlignedDecoder<D> {
    type Item = RichChar;

    fn next(&mut self) -> Option<Self::Item> {
        if self.to_fill == 0 {
            let (character, typ) = self.decoder.next()?;
            let category = match typ {
                Type::Unknown => Category::Unknown,
                _ => Category::from(&character),
            };
            self.to_fill = typ.size() - 1;
            Some(RichChar::new(character, category))
        } else {
            self.to_fill -= 1;
            Some(RichChar::new(CHARACTER_FILL, Category::Fill))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_BYTES: &[u8] = b"text, controls \n \r\n, space \t, unicode \xC3\xA4h \xC3\xA0 la \xF0\x9F\x92\xA9, null \x00, invalid \xC0\xF8\xEE";

    #[test]
    fn test_decoder_ascii() {
        let decoder = ByteAlignedDecoder::new(TEST_BYTES, Encoding::Ascii);
        let characters: Vec<_> = decoder.collect();

        assert_eq!(TEST_BYTES.len(), characters.len());
        assert_eq!(
            characters.iter().map(RichChar::escape).map(char::from).collect::<String>(),
            "text, controls _ __, space _, unicode ��h �� la ����, null 0, invalid ���"
        );
    }

    #[test]
    fn test_decoder_utf8() {
        let decoder = ByteAlignedDecoder::new(TEST_BYTES, Encoding::Utf8);
        let characters: Vec<_> = decoder.collect();

        assert_eq!(TEST_BYTES.len(), characters.len());
        assert_eq!(
            characters.iter().map(RichChar::escape).map(char::from).collect::<String>(),
            "text, controls _ __, space _, unicode ä•h à• la 💩•••, null 0, invalid ���"
        );
    }
}
