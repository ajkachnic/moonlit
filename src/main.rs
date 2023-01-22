mod document;
mod editor;
mod view;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Point;
use sdl2::render::{Canvas, TextureCreator};
use sdl2::ttf::{self, Font};
use sdl2::video::{Window, WindowContext};
use std::io;
use std::time::Duration;
use view::View;

use crate::document::Document;
use crate::editor::Editor;

struct Application<'a> {
    editor: Editor<'a>,
}

impl<'a> Application<'a> {
    pub fn new(font: Font<'a, 'a>) -> Self {
        Self {
            editor: Editor::new(Document::default(), font),
        }
    }
    pub fn render(&mut self, canvas: &mut Canvas<Window>) {
        self.editor.render(Point::new(0, 0), canvas);
    }

    pub fn handle_event(&mut self, event: Event) -> bool {
        self.editor.handle_event(event)
    }
}

pub fn main() -> io::Result<()> {
    let mut args = std::env::args();
    args.next();
    let path = args.next();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let ttf_context = ttf::init().unwrap();

    let jetbrains_mono = ttf_context
        .load_font("resources/fonts/JetBrainsMono-Regular.ttf", 14)
        .unwrap();

    let window = video_subsystem
        .window("moonlit", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let mut app = Application::new(jetbrains_mono);

    if let Some(path) = path {
        let reader = std::fs::File::open(path)?;
        app.editor.document = Document::from_reader(reader)?;
    }

    app.editor
        .document
        .configure_parser(tree_sitter_rust::language());

    let mut event_pump = sdl_context.event_pump().unwrap();
    app.render(&mut canvas);
    canvas.present();

    // Basic Event Loop
    'running: loop {
        for event in event_pump.poll_iter() {
            if app.handle_event(event) {
                break 'running;
            }
        }

        app.render(&mut canvas);
        canvas.present();
        std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
    Ok(())
}
