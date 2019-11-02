extern crate quicksilver;
#[macro_use]
extern crate recs;
use recs::{Ecs, EntityId};

use quicksilver::{
    Future, Result,
    combinators::result,
    geom::{Circle, Rectangle, Shape, Vector},
    graphics::{Background::Col, Background::Img, Color, Font, FontStyle},
    input::{Key},
    lifecycle::{Asset, Settings, State, Window, run},
};

#[derive(Copy, Clone, Debug, PartialEq)]
enum SpriteShape {
    Circle,
    Rectangle
}

#[derive(Clone, Debug, PartialEq)]
struct Sprite {
    shape: SpriteShape,
    color: Color
}

#[derive(Clone, Debug, PartialEq)]
struct Transform {
    position: Vector,
    rotation: f32,
    scale: Vector
}

#[derive(Clone, Debug, PartialEq)]
struct KeyboardMove {
    entity: EntityId 
}

struct GameplayState {
    system: Ecs,
    show_framerate: bool,
    fps_font: Font,
    fps_font_style: FontStyle
}

fn Draw(window: &mut Window, sprite: &Sprite, transform: &Transform) {
    match sprite.shape {
        SpriteShape::Circle => window.draw(&Circle::new(transform.position, transform.scale.x), Col(sprite.color)),
        SpriteShape::Rectangle => window.draw(&Rectangle::new(transform.position, transform.scale), Col(sprite.color))
    }
}

impl State for GameplayState {
    fn new() -> Result<GameplayState> {
        let fps_font: Font = match Font::load("SourceCodePro.ttf").wait() {
            Ok(f) => f,
            Err(e) => {
                println!("Failed to load SourceCodePro.ttf! : {:?}", e);
                return Err(e);
            }
        };
        let mut system = Ecs::new();
        let playerEnt: EntityId = system.create_entity();

        // Ignore result since this ID should be valid, we literally just made it
        let _ = system.set(playerEnt, Transform { position: Vector::new(100, 100), rotation: 0.0, scale: Vector::new(100, 100) });
        let _ = system.set(playerEnt, Sprite { shape: SpriteShape::Circle, color: Color::BLUE });

        let fps_font_style: FontStyle = FontStyle::new(24.0, Color::YELLOW);
        Ok(GameplayState {system, show_framerate: false, fps_font, fps_font_style} )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        // Get the ids of components that have both a transform and a sprite (everything needed to draw)
        let mut drawable_ids: Vec<EntityId> = Vec::new();
        let filter = component_filter!(Sprite, Transform);
        self.system.collect_with(&filter, &mut drawable_ids);
        // Draw everything that we can draw
        for drawable in drawable_ids {
            let sprite = match self.system.borrow::<Sprite>(drawable) {
                Ok(s) => s,
                _ => {
                    println!("Failed to find sprite component!");
                    return Ok(())
                }
            };

            let transform = match self.system.borrow::<Transform>(drawable) {
                Ok(s) => s,
                _ => {
                    println!("Failed to find transform component!");
                    return Ok(())
                }
            };
            Draw(window, sprite, transform);
        }

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
        

        if window.keyboard()[Key::Q].is_down() {
            self.show_framerate = !self.show_framerate;
        }

        Ok(())
    }
}

fn main() {
    



    run::<GameplayState>("Game Test", Vector::new(800, 600), Settings::default());
}