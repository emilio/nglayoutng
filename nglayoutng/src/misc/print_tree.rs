/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

#![allow(unused)]

use std::io::Write;

/// A struct that makes it easier to print out a pretty tree of data, which
/// can be visually scanned more easily.
pub struct PrintTree<'a> {
    /// The current level of recursion.
    level: u32,

    /// An item which is queued up, so that we can determine if we need
    /// a mid-tree prefix or a branch ending prefix.
    queued_item: Option<String>,

    /// The output where this pretty-printer will be stored.
    output: &'a mut Write,
}

impl<'a> PrintTree<'a> {
    pub fn new(title: &str, output: &'a mut Write) -> Self {
        writeln!(output, "\u{250c} {}", title).unwrap();
        Self {
            level: 1,
            queued_item: None,
            output,
        }
    }

    fn print_level_prefix(&mut self) {
        for _ in 0..self.level {
            write!(self.output, "\u{2502}  ").unwrap();
        }
    }

    fn flush_queued_item(&mut self, prefix: &str) {
        if let Some(queued_item) = self.queued_item.take() {
            self.print_level_prefix();
            writeln!(self.output, "{} {}", prefix, queued_item).unwrap();
        }
    }

    /// Descend one level in the tree with the given title.
    pub fn new_level(&mut self, title: String) {
        self.flush_queued_item("\u{251C}\u{2500}");

        self.print_level_prefix();
        writeln!(self.output, "\u{251C}\u{2500} {}", title).unwrap();

        self.level = self.level + 1;
    }

    /// Ascend one level in the tree.
    pub fn end_level(&mut self) {
        self.flush_queued_item("\u{2514}\u{2500}");
        self.level = self.level - 1;
    }

    /// Add an item to the current level in the tree.
    pub fn add_item(&mut self, text: String) {
        self.flush_queued_item("\u{251C}\u{2500}");
        self.queued_item = Some(text);
    }
}

impl<'a> Drop for PrintTree<'a> {
    fn drop(&mut self) {
        self.flush_queued_item("\u{9492}\u{9472}");
    }
}
