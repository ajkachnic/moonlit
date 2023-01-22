use std::time::Duration;

use sdl2::event::Event;
use sdl2::rect::Point;
use sdl2::render::Canvas;
use sdl2::video::Window;

pub trait View {
    fn render(&mut self, position: Point, canvas: &mut Canvas<Window>) {}
    fn handle_event(&mut self, event: Event) -> bool {
        false
    }
    fn update(&mut self, _dt: Duration) {}
}
