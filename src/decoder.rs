use std::marker::PhantomData;
use std::str::from_utf8;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CharType {
    Ascii,
    Unicode(usize),
    Unknown,
}

impl CharType {
    fn size(&self) -> usize {
        match self {
            CharType::Ascii => 1,
            CharType::Unicode(size) => *size,
            CharType::Unknown => 1,
        }
    }
}

pub(crate) trait LossyDecoder<'a>: From<&'a [u8]> + Iterator<Item=(char, CharType)> {}

pub(crate) struct LossyUTF8Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> From<&'a [u8]> for LossyUTF8Decoder<'a> {
    fn from(bytes: &'a [u8]) -> Self {
        LossyUTF8Decoder {
            bytes,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for LossyUTF8Decoder<'a> {
    type Item = (char, CharType);

    fn next(&mut self) -> Option<Self::Item> {
        if self.cursor < self.bytes.len() {
            let info = match self.bytes[self.cursor] {
                0x00..=0x7F => CharType::Ascii,
                0xC0..=0xDF => CharType::Unicode(2),
                0xE0..=0xEF => CharType::Unicode(3),
                0xF0..=0xF7 => CharType::Unicode(4),
                _ => {
                    self.cursor += 1;
                    return Some(('�', CharType::Unknown));
                }
            };

            let new_cursor = self.bytes.len().min(self.cursor + info.size());
            let chunk = &self.bytes[self.cursor..new_cursor];

            if let Ok(mut chars) = from_utf8(chunk).map(str::chars) {
                let char = chars.next().expect("the string must contain exactly one character");
                debug_assert!(chars.next().is_none(), "the string must contain exactly one character");
                self.cursor += info.size();
                Some((char, info))
            } else {
                self.cursor += 1;
                Some(('�', CharType::Unknown))
            }
        } else {
            None
        }
    }
}


impl<'a> LossyDecoder<'a> for LossyUTF8Decoder<'a> {}


pub(crate) struct ByteAlignedDecoder<'a, D: LossyDecoder<'a>> {
    decoder: D,
    to_fill: usize,
    phantom: PhantomData<&'a D>,
}

impl<'a, D: LossyDecoder<'a>> From<D> for ByteAlignedDecoder<'a, D> {
    fn from(decoder: D) -> Self {
        Self {
            decoder,
            to_fill: 0,
            phantom: PhantomData::default(),
        }
    }
}

impl<'a, D: LossyDecoder<'a>> Iterator for ByteAlignedDecoder<'a, D> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        if self.to_fill == 0 {
            let (c, info) = self.decoder.next()?;
            self.to_fill = info.size() - 1;
            Some(c)
        } else {
            self.to_fill -= 1;
            Some('•')
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoder_utf8() {
        let bytes = b"text, controls \n \r\n, space \t, unicode \xC3\xA4h \xC3\xA0 la \xF0\x9F\x92\xA9, null \x00, invalid \xC0\xF8\xEE";
        let decoder = ByteAlignedDecoder::from(LossyUTF8Decoder::from(&bytes[..]));
        let characters: Vec<_> = decoder.collect();
        let decoded = String::from_iter(&characters);

        assert_eq!(bytes.len(), characters.len());
        assert_eq!(&decoded, "text, controls \n \r\n, space \t, unicode ä•h à• la 💩•••, null \0, invalid ���");
    }
}
