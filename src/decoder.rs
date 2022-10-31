use std::fmt::Debug;
use std::iter::Peekable;
use std::str::{Chars, from_utf8_unchecked};
use std::str::from_utf8;

#[derive(Clone, Debug)]
pub(crate) struct SubString<'a> {
    string: &'a str,
    offset: usize,
    length: usize,
}

pub(crate) struct LossyUTF8Decoder<'a> {
    bytes: &'a [u8],
    cursor: usize,
}

impl<'a> LossyUTF8Decoder<'a> {
    pub(crate) fn from_bytes(bytes: &'a [u8]) -> Self {
        LossyUTF8Decoder {
            bytes,
            cursor: 0,
        }
    }
}

impl<'a> Iterator for LossyUTF8Decoder<'a> {
    type Item = SubString<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.bytes.len() {
            let offset = self.cursor;
            let window = &self.bytes[offset..];

            match from_utf8(window) {
                Ok(s) => {
                    self.cursor = self.bytes.len();
                    return Some(SubString { string: s, offset, length: self.bytes.len() - offset });
                }
                Err(e) => {
                    let length = e.valid_up_to();
                    if length > 0 {
                        let string = unsafe { from_utf8_unchecked(&window[..length]) };
                        self.cursor += length + e.error_len().unwrap_or(0);
                        return Some(SubString { string, offset, length });
                    } else {
                        self.cursor += 1;
                    }
                }
            }
        }

        None
    }
}


pub(crate) struct Decoder<'a> {
    decoder: LossyUTF8Decoder<'a>,
    chars: Peekable<Chars<'a>>,
    cursor: usize,
    offset: usize,
    to_fill: usize,
}


impl<'a> From<LossyUTF8Decoder<'a>> for Decoder<'a> {
    fn from(decoder: LossyUTF8Decoder<'a>) -> Self {
        Self {
            decoder,
            chars: "".chars().peekable(),
            cursor: 0,
            offset: 0,
            to_fill: 0,
        }
    }
}


impl<'a> Iterator for Decoder<'a> {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        // find next decodable substring
        while self.to_fill == 0 && self.cursor >= self.offset && self.chars.peek().is_none() {
            let SubString { string, offset, length: _ } = self.decoder.next()?;
            self.chars = string.chars().peekable();
            self.offset = offset;
        }

        self.cursor += 1;

        // emit dummy characters for remaining bytes of a utf-8 character
        if self.to_fill > 0 {
            self.to_fill -= 1;
            return Some('•'.to_string());
        }

        // emit dummy characters for non decodable bytes
        if self.cursor <= self.offset {
            return Some('�'.to_string());
        }

        // emit decodable characters
        let char = self.chars.next().unwrap();
        let string = if char == ' ' {
            " ".to_string()
        } else if char.is_whitespace() {
            "_".to_string()
        } else if char.is_control() {
            " ".to_string()
        } else {
            char.to_string()
        };
        let size = string.len();

        self.offset += size;
        self.to_fill = size - 1;

        Some(string)
    }
}