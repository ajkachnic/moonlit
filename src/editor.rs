use sdl2::event::Event;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::ttf::Font;
use sdl2::video::Window;

use crate::document::Document;
use crate::view::View;

use tree_sitter::{Language, Parser};
use tree_sitter_highlight::{HighlightConfiguration, Highlighter};
pub struct ColorScheme {
    background: sdl2::pixels::Color,
    foreground: sdl2::pixels::Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            background: sdl2::pixels::Color::RGB(30, 33, 39),
            foreground: sdl2::pixels::Color::RGB(255, 255, 255),
        }
    }
}

const HIGHLIGHT_NAMES: &[&str] = &[
    "attribute",
    "constant",
    "function.builtin",
    "function",
    "keyword",
    "string",
    "type",
    "variable",
];

pub struct Editor<'a> {
    pub document: Document,
    pub rerender: bool,

    font: Font<'a, 'a>,
    color_scheme: ColorScheme,
    // highlighter: Option<Highlighter>,
}

impl<'a> Editor<'a> {
    pub fn new(document: Document, font: Font<'a, 'a>) -> Self {
        Self {
            document,
            rerender: true,
            font,
            color_scheme: ColorScheme::default(),
            // parser: None,
            // highlighter: None,
        }
    }

    // pub fn configure_highlighter(&mut self, language: Language) {
    //     let config = HighlightConfiguration::new(language, "", "", "").unwrap();

    //     config.configure(&HIGHLIGHT_NAMES);
    //     self.highlighter = Some(highlighter);
    // }

    fn render_cursor(&self, position: sdl2::rect::Point, canvas: &mut Canvas<Window>) {
        // We only support monospace fonts so we can calculate the width/height of a character
        let (char_width, char_height) = self.font.size_of_char(' ').unwrap();

        canvas.set_draw_color(self.color_scheme.foreground);
        canvas
            .fill_rect(sdl2::rect::Rect::new(
                position.x,
                position.y,
                2 as u32,
                char_height as u32,
            ))
            .unwrap();
    }
}

impl<'a> View for Editor<'a> {
    fn render(
        &mut self,
        position: sdl2::rect::Point,
        canvas: &mut sdl2::render::Canvas<sdl2::video::Window>,
    ) {
        if self.rerender == false {
            return;
        }

        let start = std::time::Instant::now();
        println!("start rendering");

        let texture_creator = canvas.texture_creator();

        canvas.set_draw_color(self.color_scheme.background);
        canvas.clear();

        // We only support monospace fonts so we can calculate the width/height of a character
        let (char_width, char_height) = self.font.size_of_char(' ').unwrap();

        // Draw the cursor
        let mut cursor_drawn = false;

        // Draw the actual text
        let mut y_offset = 0;
        let mut x_offset = 0;
        let mut y: usize = 0;
        for line in self.document.rope.lines() {
            if y_offset as u32 > canvas.output_size().unwrap().1 {
                break;
            }
            let mut x: usize = 0;
            x_offset = 0;
            'ch: for ch in line.chars() {
                if x == self.document.cursor.x && y == self.document.cursor.y {
                    println!("({x}, {y}) ({x_offset}, {y_offset})");
                    self.render_cursor(
                        Point::new(x_offset + position.x, y_offset + position.y),
                        canvas,
                    );
                    cursor_drawn = true;
                }

                match ch {
                    '\t' => {
                        x_offset += char_width as i32 * 4;
                        continue;
                    }
                    '\n' => {
                        break 'ch;
                    }
                    _ => {}
                }

                let surface = self
                    .font
                    .render_char(ch)
                    .blended(self.color_scheme.foreground)
                    .unwrap();
                let texture = texture_creator
                    .create_texture_from_surface(&surface)
                    .unwrap();

                canvas
                    .copy(
                        &texture,
                        None,
                        sdl2::rect::Rect::new(
                            position.x + x_offset,
                            position.y + y_offset,
                            char_width as u32,
                            char_height as u32,
                        ),
                    )
                    .unwrap();

                x_offset += char_width as i32;
                x += 1;
            }
            y_offset += char_height as i32;
            y += 1;
        }

        // Cursor is at the end of the document
        if !cursor_drawn {
            self.render_cursor(
                Point::new(
                    x_offset + position.x,
                    y_offset - char_height as i32 + position.y,
                ),
                canvas,
            );
        }

        println!("done rendering: {:?}", std::time::Instant::now() - start);

        self.rerender = false;
    }

    fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Left),
                ..
            } => {
                self.document.move_cursor_left();
                self.rerender = true;
                false
            }
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Right),
                ..
            } => {
                self.document.move_cursor_right();
                self.rerender = true;
                false
            }
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Up),
                ..
            } => {
                self.document.move_cursor_up();
                self.rerender = true;
                false
            }
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Down),
                ..
            } => {
                self.document.move_cursor_down();
                self.rerender = true;
                false
            }
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Backspace),
                ..
            } => {
                self.document.remove_char();
                self.rerender = true;
                false
            }
            Event::KeyDown {
                keycode: Some(sdl2::keyboard::Keycode::Return),
                ..
            } => {
                self.document.newline();
                self.rerender = true;
                false
            }
            Event::TextInput { text, .. } => {
                println!("TextInput {:?}", text);
                self.document.insert_text(text.as_str());
                self.rerender = true;
                false
            }
            Event::Quit { .. } => true,
            _ => false,
        }
    }
}
