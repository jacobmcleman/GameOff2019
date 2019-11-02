extern crate quicksilver;

use quicksilver::{
    Result,
    geom::{Circle, Line, Rectangle, Transform, Triangle, Vector},
    graphics::{Background::Col, Color},
    input::{Key},
    lifecycle::{Settings, State, Window, run}
};

struct DrawObjects {
    player: GameObject
}

enum SpriteShape {
    Circle,
    Rectangle
}

struct GameObject {
    position: Vector,
    rotation: f32,
    scale: Vector,
    shape: SpriteShape,
    color: Color
}

impl GameObject {
    fn draw(&mut self, window: &mut Window) {
        match self.shape {
            SpriteShape::Circle => window.draw(&Circle::new(self.position, self.scale.x), Col(self.color)),
            SpriteShape::Rectangle => window.draw(&Rectangle::new(self.position, self.scale), Col(self.color))
        }
    }
}

impl State for DrawObjects {
    fn new() -> Result<DrawObjects> {
        let player = GameObject { position: Vector::new(100, 100), rotation: 0.0, scale: Vector::new(100, 100), shape: SpriteShape::Circle, color: Color::BLUE};
        Ok(DrawObjects {player} )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::WHITE)?;
        self.player.draw(window);
        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        if window.keyboard()[Key::Left].is_down() {
            self.player.position.x -= 1.0;
        }
        if window.keyboard()[Key::Right].is_down() {
            self.player.position.x += 1.0;
        }
        if window.keyboard()[Key::Down].is_down() {
            self.player.position.y += 1.0;
        }
        if window.keyboard()[Key::Up].is_down() {
            self.player.position.y -= 1.0;
        }

        Ok(())
    }
}

fn main() {
    run::<DrawObjects>("Game Test", Vector::new(800, 600), Settings::default());
}