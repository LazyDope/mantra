//! A currency tracker for the LANCER TTRPG system, combination of Manna and Tracker.
//! Provides summarizing, filtering, and multi-pilot support

use std::{fmt::Display, ops::Deref};

use crossterm::event::KeyModifiers;
use xdg::BaseDirectories;

pub mod app;
pub mod config;
/// This module interfaces with the local sqlite database
pub mod storage;

/// grabs the XDG dirs
pub fn base_dirs() -> Result<BaseDirectories, xdg::BaseDirectoriesError> {
    BaseDirectories::with_prefix("mantra")
}

/// Returns an appropriatly scaled value given the held modifier keys
pub fn value_from_modifiers(modifiers: KeyModifiers) -> i32 {
    let mut value = 10;
    if modifiers.contains(KeyModifiers::SHIFT) {
        value = 1;
    }
    if modifiers.contains(KeyModifiers::CONTROL) {
        value *= 5;
    }
    if modifiers.contains(KeyModifiers::ALT) {
        value *= 20;
    }

    value
}

/// A String with a cursor character based position for editing
/// The cursor is always considered 'in front' of the character with the same index
#[derive(Default)]
pub struct CursoredString {
    buf: String,
    index: usize,
    pub inserting: bool,
}

impl CursoredString {
    /// Creates a new empty CursoredString
    pub fn new() -> Self {
        Self::default()
    }

    /// Gets the text from the internal buffer
    pub fn as_str(&self) -> &str {
        self
    }

    /// Gets the current index for the cursor
    pub fn cursor_index(&self) -> usize {
        self.index
    }

    /// Move the cursor to the right
    pub fn next(&mut self) {
        self.index = self.index.saturating_add(1).clamp(0, self.buf.len())
    }

    /// Move the cursor to the left
    pub fn prev(&mut self) {
        self.index = self.index.saturating_sub(1).clamp(0, self.buf.len())
    }

    /// Remove a character from behind the cursor
    pub fn remove_behind(&mut self) {
        // can't delete behind index 0
        if self.index > 0 {
            let old_len = self.buf.len();
            let mut index = 0;
            // retain is used to modify in place
            self.buf.retain(|_| {
                index += 1;
                index != self.index
            });
            // length change indicates successful deletion
            if self.buf.len() < old_len {
                self.index -= 1;
            };
        }
    }

    /// Removes a character ahead (same index) of the cursor
    pub fn remove_ahead(&mut self) {
        if self.index < self.buf.chars().count() {
            let mut index = 0;
            self.buf.retain(|_| {
                index += 1;
                if index - 1 == self.index {
                    return false;
                }
                true
            })
        }
    }

    /// Inserts a character at the current position, moving existing characters after the cursor ahead.
    /// Replaces the current character if insert mode is enabled.
    pub fn insert(&mut self, value: char) {
        if self.inserting {
            self.remove_ahead();
        }
        // String.insert is indexed by byte so we get the byte index from the char index
        let byte_index = self
            .buf
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.index)
            .unwrap_or(self.buf.len());

        self.buf.insert(byte_index, value);
        self.index += 1
    }
}

impl Display for CursoredString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.buf.fmt(f)
    }
}

impl From<CursoredString> for String {
    fn from(value: CursoredString) -> Self {
        value.buf
    }
}

impl Deref for CursoredString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.buf.as_str()
    }
}

impl From<String> for CursoredString {
    fn from(value: String) -> Self {
        Self {
            buf: value,
            index: 0,
            inserting: false,
        }
    }
}
