extern crate quicksilver;

use quicksilver::{
    Future, Result,
    combinators::result,
    geom::{Circle, Rectangle, Shape, Vector},
    graphics::{Background::Col, Background::Img, Color, Font, FontStyle},
    input::{Key},
    lifecycle::{Asset, Settings, State, Window, run},
};

struct DrawObjects {
    player: GameObject,
    show_framerate: bool,
    fps_font: Font,
    fps_font_style: FontStyle
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
        let fps_font: Font = match Font::load("SourceCodePro.ttf").wait() {
            Ok(f) => f,
            Err(e) => {
                println!("Failed to load SourceCodePro.ttf! : {:?}", e);
                return Err(e);
            }
        };
        let fps_font_style: FontStyle = FontStyle::new(24.0, Color::YELLOW);

        let player = GameObject { position: Vector::new(100, 100), rotation: 0.0, scale: Vector::new(100, 100), shape: SpriteShape::Circle, color: Color::BLUE};
        Ok(DrawObjects {player, show_framerate: false, fps_font, fps_font_style} )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;
        self.player.draw(window);

        if self.show_framerate {
            // Show 2 decimal places after the .
            let fps_string = format!("FPS: {:.*}", 2, window.current_fps());
            let mut fps_text = Asset::new(result(self.fps_font.render(&fps_string, &self.fps_font_style)));
            fps_text.execute(|image| {
                window.draw(&image.area().with_center((650, 50)), Img(&image));
                Ok(())
            })?;
        }

        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        if window.keyboard()[Key::A].is_down() {
            self.player.position.x -= 1.0;
        }
        if window.keyboard()[Key::D].is_down() {
            self.player.position.x += 1.0;
        }
        if window.keyboard()[Key::S].is_down() {
            self.player.position.y += 1.0;
        }
        if window.keyboard()[Key::W].is_down() {
            self.player.position.y -= 1.0;
        }

        if window.keyboard()[Key::Q].is_down() {
            self.show_framerate = !self.show_framerate;
        }

        Ok(())
    }
}

fn main() {
    run::<DrawObjects>("Game Test", Vector::new(800, 600), Settings::default());
}