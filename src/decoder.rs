use std::str::from_utf8;

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
                // TODO test ranges
                0x00..=0x7F => CharType::Ascii,
                0xC0..=0xDF => CharType::Unicode(2),
                0xE0..=0xEF => CharType::Unicode(3),
                0xF0..=0xF7 => CharType::Unicode(4),
                _ => CharType::Unknown,
            };

            let new_cursor = self.bytes.len().min(self.cursor + info.size());
            let chunk = &self.bytes[self.cursor..new_cursor];

            if let Ok(mut chars) = from_utf8(chunk).map(str::chars) {
                let char = chars.next().unwrap();
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


pub(crate) struct ByteAlignedDecoder<'a> {
    decoder: LossyUTF8Decoder<'a>,
    to_fill: usize,
}

impl<'a> From<LossyUTF8Decoder<'a>> for ByteAlignedDecoder<'a> {
    fn from(decoder: LossyUTF8Decoder<'a>) -> Self {
        Self {
            decoder,
            to_fill: 0,
        }
    }
}

impl<'a> Iterator for ByteAlignedDecoder<'a> {
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
