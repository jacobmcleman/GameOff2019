extern crate quicksilver;
#[macro_use]
extern crate recs;
use recs::{Ecs, EntityId};

use quicksilver::{
    Result,
    geom::{Circle, Rectangle, Vector},
    graphics::{Background::Col, Color},
    input::{Key},
    lifecycle::{Settings, State, Window, run},
};

#[derive(Copy, Clone, Debug, PartialEq)]
enum SpriteShape {
    _Circle,
    _Rectangle
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
    speed: f32
}

struct GameplayState {
    system: Ecs
}

fn draw(window: &mut Window, sprite: &Sprite, transform: &Transform) {
    match sprite.shape {
        SpriteShape::_Circle => window.draw(&Circle::new(transform.position, transform.scale.x), Col(sprite.color)),
        SpriteShape::_Rectangle => window.draw(&Rectangle::new(transform.position, transform.scale), Col(sprite.color))
    }
}

impl State for GameplayState {
    fn new() -> Result<GameplayState> {
        let mut system = Ecs::new();
        let player_ent: EntityId = system.create_entity();

        // Ignore result since this ID should be valid, we literally just made it
        let _ = system.set(player_ent, Transform { position: Vector::new(100, 100), rotation: 0.0, scale: Vector::new(100, 100) });
        let _ = system.set(player_ent, Sprite { shape: SpriteShape::_Circle, color: Color::BLUE });
        let _ = system.set(player_ent, KeyboardMove { speed: 2.5 });

        Ok(GameplayState {system } )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        // Get the ids of components that have both a transform and a sprite (everything needed to draw)
        let mut drawable_ids: Vec<EntityId> = Vec::new();
        let drawable_filter = component_filter!(Sprite, Transform);
        self.system.collect_with(&drawable_filter, &mut drawable_ids);
        // Draw everything that we can draw
        for drawable in drawable_ids {
            let sprite: &Sprite = self.system.borrow(drawable).unwrap();
            let transform: &Transform = self.system.borrow(drawable).unwrap();
            draw(window, sprite, transform);
        }

        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
         // Get the ids of components that have both a transform and a sprite (everything needed to draw)
         let mut updatable_ids: Vec<EntityId> = Vec::new();
         let updatable_filter = component_filter!(KeyboardMove, Transform);
         self.system.collect_with(&updatable_filter, &mut updatable_ids);
         // Draw everything that we can draw
         for updateable in updatable_ids {
            let mover: &KeyboardMove = self.system.borrow(updateable).unwrap();
            let mut x_move = 0.0;
            let mut y_move = 0.0;

            if window.keyboard()[Key::W].is_down() { y_move -= mover.speed; }
            if window.keyboard()[Key::S].is_down() { y_move += mover.speed; }
            if window.keyboard()[Key::A].is_down() { x_move -= mover.speed; }
            if window.keyboard()[Key::D].is_down() { x_move += mover.speed; }
            
            if x_move != 0.0 {
                self.system.borrow_mut::<Transform>(updateable).map(|transform| transform.position.x += x_move).unwrap();
            }
            if y_move != 0.0 {
                self.system.borrow_mut::<Transform>(updateable).map(|transform| transform.position.y += y_move).unwrap();
            }
         }

        Ok(())
    }
}

fn main() {
    



    run::<GameplayState>("Game Test", Vector::new(800, 600), Settings::default());
}