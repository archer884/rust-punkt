// Copyright 2016 rust-punkt developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use std::{
    cell::Cell,
    hash::{Hash, Hasher},
    ops::Deref,
};

use crate::prelude::LetterCase;

// These 6 flags only use the lower 8 bits.
const HAS_FINAL_PERIOD: u16 = 0b0000000000000001;
const IS_ELLIPSIS: u16 = 0b0000000000000010;
const IS_ABBREV: u16 = 0b0000000000000100;
const IS_SENTENCE_BREAK: u16 = 0b0000000000001000;
const IS_PARAGRAPH_START: u16 = 0b0000000000010000;
const IS_NEWLINE_START: u16 = 0b0000000000100000;
const IS_UPPERCASE: u16 = 0b0000000001000000;
const IS_LOWERCASE: u16 = 0b0000000010000000;

// These flags only use the upper 8 bits.
const IS_INITIAL: u16 = 0b1000000000000000;
const IS_NUMERIC: u16 = 0b0100000000000000;
const IS_NON_PUNCT: u16 = 0b0010000000000000;
const IS_ALPHABETIC: u16 = 0b0000010000000000;

#[derive(Eq)]
pub struct Token {
    inner: String,
    flags: Cell<u16>,
}

impl Token {
    pub fn new(slice: &str, is_el: bool, is_pg: bool, is_nl: bool) -> Token {
        debug_assert!(!slice.is_empty());

        let first = slice.chars().next().unwrap();
        let mut has_punct = false;

        // Add a period to any tokens without a period. This is an optimization
        // to avoid creating an entirely new token when using as a key.
        let mut tok = if slice.as_bytes()[slice.len() - 1] == b'.' {
            let tok = Token {
                inner: String::with_capacity(slice.len()),
                flags: 0x00.into(),
            };

            tok.set_has_final_period(true);
            tok
        } else {
            Token {
                inner: String::with_capacity(slice.len() + 1),
                flags: 0x00.into(),
            }
        };

        if is_str_numeric(slice) {
            tok.set_is_numeric(true);
        } else if is_str_initial(slice) {
            tok.set_is_initial(true);
        }

        for c in slice.chars() {
            for c0 in c.to_lowercase() {
                tok.inner.push(c0);
            }

            if c.is_alphabetic() || c == '_' {
                tok.set_is_non_punct(true);
            } else if !c.is_ascii_digit() {
                has_punct = true;
            }
        }

        if !tok.has_final_period() {
            tok.inner.push('.');
        }

        if first.is_uppercase() {
            tok.set_is_uppercase(true);
        } else if first.is_lowercase() {
            tok.set_is_lowercase(true);
        }

        tok.set_is_alphabetic(!has_punct);
        tok.set_is_ellipsis(is_el);
        tok.set_is_paragraph_start(is_pg);
        tok.set_is_newline_start(is_nl);

        tok
    }

    /// Returns the normalized original token (which can be reconstructed from
    /// the inner representation of the token, and the flags on the token).
    #[inline(always)]
    pub fn tok(&self) -> &str {
        if self.has_final_period() {
            &self.inner[..]
        } else {
            &self.inner[..self.inner.len() - 1]
        }
    }

    /// Returns the token with any ending period truncated.
    #[inline(always)]
    pub fn tok_without_period(&self) -> &str {
        if self.has_final_period() {
            &self.tok()[..self.len() - 1]
        } else {
            self.tok()
        }
    }

    /// Returns the type of the token. If the token is numeric (determined by flags),
    /// returns `##number##`, otherwise returns the normalized token.
    #[inline(always)]
    pub fn typ(&self) -> &str {
        if self.is_numeric() {
            "##number##"
        } else {
            self.tok()
        }
    }

    /// Returns the type of the token with a period appended to it. Returns
    /// `##number##.` if the token is numeric (determined by flags), otherwise
    /// returns the original token with a period appended to it.
    #[inline(always)]
    pub fn typ_with_period(&self) -> &str {
        if self.is_numeric() {
            "##number##."
        } else {
            &self.inner[..]
        }
    }

    /// Returns the type of the token without a period appended to it. Will return
    /// `.`, if it is the only character in the string; otherwise, will slice type
    /// to exclude the final period.
    #[inline(always)]
    pub fn typ_without_period(&self) -> &str {
        if self.tok().len() > 1 && self.has_final_period() {
            &self.typ_with_period()[..self.typ_with_period().len() - 1]
        } else {
            self.typ()
        }
    }

    /// Returns the type of the token without a break or period if it had one originally
    /// at the end.
    #[inline(always)]
    pub fn typ_without_break_or_period(&self) -> &str {
        if self.is_sentence_break() {
            self.typ_without_period()
        } else {
            self.typ()
        }
    }

    #[inline(always)]
    pub fn first_case(&self) -> LetterCase {
        if self.is_uppercase() {
            LetterCase::Upper
        } else if self.is_lowercase() {
            LetterCase::Lower
        } else {
            LetterCase::Unknown
        }
    }

    #[inline(always)]
    pub fn is_uppercase(&self) -> bool {
        self.flags.get() & IS_UPPERCASE != 0
    }

    #[inline(always)]
    pub fn is_lowercase(&self) -> bool {
        self.flags.get() & IS_LOWERCASE != 0
    }

    #[inline(always)]
    pub fn is_ellipsis(&self) -> bool {
        self.flags.get() & IS_ELLIPSIS != 0
    }

    #[inline(always)]
    pub fn is_abbrev(&self) -> bool {
        self.flags.get() & IS_ABBREV != 0
    }

    #[inline(always)]
    pub fn is_sentence_break(&self) -> bool {
        self.flags.get() & IS_SENTENCE_BREAK != 0
    }

    #[inline(always)]
    pub fn has_final_period(&self) -> bool {
        self.flags.get() & HAS_FINAL_PERIOD != 0
    }

    #[inline(always)]
    pub fn is_paragraph_start(&self) -> bool {
        self.flags.get() & IS_PARAGRAPH_START != 0
    }

    #[inline(always)]
    pub fn is_newline_start(&self) -> bool {
        self.flags.get() & IS_NEWLINE_START != 0
    }

    #[inline(always)]
    pub fn is_numeric(&self) -> bool {
        self.flags.get() & IS_NUMERIC != 0
    }

    #[inline(always)]
    pub fn is_initial(&self) -> bool {
        self.flags.get() & IS_INITIAL != 0
    }

    // The NLTK docs note that all numeric tokens are considered to be contain
    // only punctuation, because they are converted to `##number##`, which clearly
    // has alphabetic characters.
    #[inline(always)]
    pub fn is_non_punct(&self) -> bool {
        (self.flags.get() & IS_NON_PUNCT != 0) || self.is_numeric()
    }

    #[inline(always)]
    pub fn is_alphabetic(&self) -> bool {
        self.flags.get() & IS_ALPHABETIC != 0
    }

    #[inline(always)]
    pub fn set_is_ellipsis(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_ELLIPSIS);
        } else if self.is_ellipsis() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_ELLIPSIS);
        }
    }

    #[inline(always)]
    pub fn set_is_abbrev(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_ABBREV);
        } else if self.is_abbrev() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_ABBREV);
        }
    }

    #[inline(always)]
    pub fn set_is_sentence_break(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_SENTENCE_BREAK);
        } else if self.is_sentence_break() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_SENTENCE_BREAK);
        }
    }

    #[inline(always)]
    pub fn set_has_final_period(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | HAS_FINAL_PERIOD);
        } else if self.has_final_period() {
            let value = self.flags.get();
            self.flags.set(value ^ HAS_FINAL_PERIOD);
        }
    }

    #[inline(always)]
    pub fn set_is_paragraph_start(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_PARAGRAPH_START);
        } else if self.is_paragraph_start() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_PARAGRAPH_START);
        }
    }

    #[inline(always)]
    pub fn set_is_newline_start(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_NEWLINE_START);
        } else if self.is_newline_start() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_NEWLINE_START);
        }
    }

    #[inline(always)]
    pub fn set_is_uppercase(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_UPPERCASE);
        } else if self.is_uppercase() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_UPPERCASE);
        }
    }

    #[inline(always)]
    pub fn set_is_lowercase(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_LOWERCASE);
        } else if self.is_lowercase() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_LOWERCASE);
        }
    }

    #[inline(always)]
    pub fn set_is_numeric(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_NUMERIC);
        } else if self.is_numeric() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_NUMERIC);
        }
    }

    #[inline(always)]
    pub fn set_is_initial(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_INITIAL);
        } else if self.is_initial() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_INITIAL);
        }
    }

    #[inline(always)]
    pub fn set_is_non_punct(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_NON_PUNCT);
        } else if self.is_non_punct() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_NON_PUNCT);
        }
    }

    #[inline(always)]
    pub fn set_is_alphabetic(&self, b: bool) {
        if b {
            let value = self.flags.get();
            self.flags.set(value | IS_ALPHABETIC);
        } else if self.is_alphabetic() {
            let value = self.flags.get();
            self.flags.set(value ^ IS_ALPHABETIC);
        }
    }
}

impl Deref for Token {
    type Target = str;

    #[inline(always)]
    fn deref(&self) -> &str {
        &self.inner[..]
    }
}

impl PartialEq for Token {
    #[inline(always)]
    fn eq(&self, other: &Token) -> bool {
        self.typ() == other.typ()
    }
}

impl Hash for Token {
    #[inline(always)]
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        self.typ().hash(state)
    }
}

/// A number can start with a negative sign ('-'), and be followed by digits
/// or isolated periods, commas, or dashes.
/// Note: It's assumed that multi-chars are taken out of the input when creating word
/// tokens, so a numeric word token SHOULD not have a multi-char within it as
/// its received. This assumption should be fulfilled by the parser generating
/// these word tokens. If it isn't some weird outputs are possible (such as "5.4--5").
#[inline]
fn is_str_numeric(tok: &str) -> bool {
    let mut digit_found = false;
    let mut pos = 0;

    for c in tok.chars() {
        match c {
            // A digit was found. Note this to confirm later if punctuation
            // within the number is valid or not.
            _ if c.is_ascii_digit() => digit_found = true,
            // A delimeter was found. This is valid as long as
            // a digit was also found prior.
            ',' | '.' | '-' if digit_found => (),
            // A comma or period was found as the first character, or
            // after a negative sign. This is a valid token.
            ',' | '.' if pos == 0 || pos == 1 => (),
            // A negative sign is found.
            '-' if pos == 0 => (),
            // A non numeric token was encountered in the string that
            // isn't a valid one. Return false.
            _ => return false,
        }

        pos += c.len_utf8();
    }

    digit_found
}

/// Tests if the token is an initial. An initial is a 2 character grouping
/// where the first character is a letter (non-digit, non-symbol), and the
/// next is a period.
#[inline]
fn is_str_initial(tok: &str) -> bool {
    let mut iter = tok.chars();

    match (iter.next(), iter.next()) {
        (Some(c), Some('.')) if c.is_alphabetic() => iter.next().is_none(),
        _ => false,
    }
}

#[test]
fn test_token_flags() {
    macro_rules! perform_flag_test(
    ($tok:expr, $f:ident, $t:ident) => (
      {
        $tok.$f(true);
        assert!($tok.$t());
        $tok.$f(false);
        assert!(!$tok.$t());
      }
    )
  );

    let tok = Token::new("test", false, false, false);

    tok.set_is_non_punct(false);
    tok.set_is_lowercase(false);
    tok.set_is_alphabetic(false);

    assert_eq!(tok.flags.get(), 0);

    perform_flag_test!(tok, set_is_ellipsis, is_ellipsis);
    perform_flag_test!(tok, set_is_abbrev, is_abbrev);
    perform_flag_test!(tok, set_has_final_period, has_final_period);
    perform_flag_test!(tok, set_is_paragraph_start, is_paragraph_start);
    perform_flag_test!(tok, set_is_newline_start, is_newline_start);
    perform_flag_test!(tok, set_is_uppercase, is_uppercase);
    perform_flag_test!(tok, set_is_lowercase, is_lowercase);
    perform_flag_test!(tok, set_is_numeric, is_numeric);
    perform_flag_test!(tok, set_is_initial, is_initial);
    perform_flag_test!(tok, set_is_non_punct, is_non_punct);
    perform_flag_test!(tok, set_is_alphabetic, is_alphabetic);
}
