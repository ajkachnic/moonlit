use ropey::iter::Bytes;
use tree_sitter::{Language, Parser, Query, QueryCursor, TextProvider, Tree};

use ringbuffer::{AllocRingBuffer, RingBuffer, RingBufferWrite};

/// A single edit to the document.
///
/// Used for undo/redo and tree-sitter incremental parsing.
pub enum Edit {
    Insert {
        char_idx: usize,
        ch: char,
        point: Cursor,
    },
    Delete {
        char_idx: usize,
        ch: char,
        point: Cursor,
    },
    /// Group x edits together for undo/redo
    Group(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cursor {
    pub x: usize,
    pub y: usize,
}

impl Cursor {
    pub fn newline(&mut self) {
        self.y += 1;
        self.x = 0;
    }

    pub fn move_left(&mut self) {
        if self.x > 0 {
            self.x -= 1;
        }
    }

    pub fn move_right(&mut self) {
        self.x += 1;
    }

    pub fn move_up(&mut self) {
        self.y -= 1;
        self.x = 0;
    }

    pub fn move_down(&mut self) {
        println!("moving cursor down");
        self.y += 1;
        self.x = 0;
    }
}

impl Default for Cursor {
    fn default() -> Self {
        Self { x: 0, y: 0 }
    }
}

pub struct Highlighter {
    language: Language,
    highlight_query: tree_sitter::Query,
    cursor: Option<QueryCursor>,
}

impl Highlighter {
    pub fn new(language: Language, highlight_query: &'static str) -> Self {
        Self {
            language,
            highlight_query: Query::new(language, highlight_query).unwrap(),
            cursor: None,
        }
    }

    pub fn highlight<'slf, 'a, T>(&'slf mut self, tree: &'a Tree, text: T)
    where
        'slf: 'a,
        T: TextProvider<'a> + 'a,
    {
        self.cursor = Some(tree_sitter::QueryCursor::new());

        for matches in
            self.cursor
                .as_mut()
                .unwrap()
                .matches(&self.highlight_query, tree.root_node(), text)
        {
            println!("{:?}", matches)
        }
    }
}

/// The backing document type for the editor.
///
/// Uses a rope to efficiently store the document text. There's also an optional
/// tree-sitter parser that can be used to incrementally parse the document.
///
/// ## Usage
///
/// ```rust
/// use crate::document::Document;
///
/// let doc = Document::from_str("Hello, world!");
///
/// doc.insert_text("\nhello again!");
pub struct Document {
    pub rope: ropey::Rope,
    pub cursor: Cursor,
    parser: Option<Parser>,
    tree: Option<Tree>,
    highlighter: Option<Highlighter>,
    edits: AllocRingBuffer<Edit>,
}

impl Document {
    pub fn from_reader<T>(r: T) -> std::io::Result<Self>
    where
        T: std::io::Read,
    {
        Ok(Self {
            rope: ropey::Rope::from_reader(r)?,
            cursor: Cursor::default(),
            parser: None,
            tree: None,
            highlighter: None,
            // Must be power of 2
            edits: AllocRingBuffer::with_capacity(16 * 16),
        })
    }

    pub fn from_str(s: &str) -> Self {
        Self {
            rope: ropey::Rope::from_str(s),
            cursor: Cursor::default(),
            parser: None,
            tree: None,
            highlighter: None,
            edits: AllocRingBuffer::with_capacity(32 * 32),
        }
    }

    pub fn configure_parser(&mut self, language: Language) {
        let mut parser = Parser::new();
        parser.set_language(language).unwrap();
        self.parser = Some(parser);

        self.highlighter = Some(Highlighter::new(
            language,
            // TODO: Get highlight query from caller
            tree_sitter_rust::HIGHLIGHT_QUERY,
        ));

        self.reparse();
        self.highlighter.as_mut().unwrap().highlight(
            self.tree.as_ref().unwrap(),
            RopeTextProvider { rope: &self.rope },
        );
    }

    fn reparse(&mut self) {
        println!("REPARSING");
        if let Some(parser) = &mut self.parser {
            self.tree = parser.parse_with(
                &mut |u, _p| {
                    if u > self.rope.len_bytes() {
                        return "";
                    }
                    self.rope.chunk_at_byte(u).0
                },
                self.tree.as_ref(),
            );

            if let Some(tree) = &self.tree {
                let mut cursor = tree.walk();

                // Walk the tree and print all nodes
                // loop {
                //     let node = cursor.node();
                //     let start = node.start_byte();
                //     let end = node.end_byte();
                //     let text = self.rope.get_slice(start..end);
                //     if let Some(text) = text {
                //         println!("{}: {}", node.kind(), text);
                //     }
                //     if cursor.goto_first_child() {
                //         continue;
                //     }
                //     while !cursor.goto_next_sibling() {
                //         if !cursor.goto_parent() {
                //             return;
                //         }
                //     }
                // }
            }
        }
    }

    pub fn newline(&mut self) {
        self.insert_char('\n');
    }

    /// Append a character to the cursor position.
    pub fn insert_char(&mut self, ch: char) {
        let char_idx = self.rope.line_to_char(self.cursor.y) + self.cursor.x;

        if ch == '\n' {
            self.cursor.newline();
        } else {
            self.cursor.x += 1;
        }

        self.rope.insert_char(char_idx, ch);
        self.reparse();
    }

    pub fn insert_text(&mut self, text: &str) {
        let char_idx = self.rope.line_to_char(self.cursor.y) + self.cursor.x;
        if char_idx > self.rope.len_chars() {
            eprintln!(
                "CHAR INDEX OOB: char_idx {} > rope.len_chars {}",
                char_idx,
                self.rope.len_chars()
            );
            return;
        }

        for ch in text.chars() {
            self.edits.push(Edit::Insert {
                char_idx,
                ch,
                point: self.cursor,
            });
            if ch == '\n' {
                self.cursor.newline();
            } else {
                self.cursor.x += 1;
            }
        }

        self.rope.insert(char_idx, text);

        self.reparse();
    }

    /// Remove a character from the cursor position
    pub fn remove_char(&mut self) {
        let char_idx = self.rope.line_to_char(self.cursor.y) + self.cursor.x;
        if char_idx > 0 {
            self.rope.remove(char_idx - 1..char_idx);
            self.cursor.move_left();
        }
        self.reparse();
    }

    pub fn move_cursor_down(&mut self) {
        if self.rope.len_lines() > self.cursor.y {
            self.cursor.move_down();
        }
    }

    pub fn move_cursor_up(&mut self) {
        if self.cursor.y > 0 {
            self.cursor.move_up();
        }
    }

    pub fn move_cursor_left(&mut self) {
        if self.cursor.x > 0 {
            self.cursor.move_left();
        } else if self.cursor.y > 0 {
            self.cursor.y -= 1;
            self.cursor.x = 0;
        }
    }

    pub fn move_cursor_right(&mut self) {
        if self.cursor.y >= self.rope.len_lines() {
            return;
        }
        let line = self.rope.line(self.cursor.y);
        if self.cursor.x < line.len_chars() {
            self.cursor.move_right();
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self {
            rope: ropey::Rope::from_str(""),
            cursor: Cursor::default(),
            parser: None,
            tree: None,
            highlighter: None,
            edits: AllocRingBuffer::with_capacity(16 * 16),
        }
    }
}

struct RopeTextProvider<'a> {
    rope: &'a ropey::Rope,
}

impl<'a> TextProvider<'a> for RopeTextProvider<'a> {
    type I = ChunksWrapper<'a>;
    fn text(&mut self, node: tree_sitter::Node) -> Self::I {
        println!("{:?}", node);
        match self.rope.get_slice(node.start_byte()..node.end_byte()) {
            Some(s) => s.chunks().into(),
            None => ChunksWrapper(None),
        }
        // self.rope
        //     .get_slice(node.start_byte()..node.end_byte())
        //     .unwrap()
        //     .chunks()
        //     .into()
    }
}
struct ChunksWrapper<'a>(Option<ropey::iter::Chunks<'a>>);

impl<'a> From<ropey::iter::Chunks<'a>> for ChunksWrapper<'a> {
    fn from(c: ropey::iter::Chunks<'a>) -> Self {
        Self(Some(c))
    }
}

impl<'a> Iterator for ChunksWrapper<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.0 {
            Some(c) => c.next().map(|v| v.as_bytes()),
            None => None,
        }
    }
}
