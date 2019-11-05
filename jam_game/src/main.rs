extern crate quicksilver;
#[macro_use]
extern crate recs;
use recs::{Ecs, EntityId};

use noise::{NoiseFn, HybridMulti};

use quicksilver::{
    Result,
    geom::{Circle, Rectangle, Vector},
    graphics::{Background::Col, Color, View},
    input::{Key},
    lifecycle::{Settings, State, Window, run},
};

#[derive(Copy, Clone, Debug, PartialEq)]
enum SpriteShape {
    _Circle,
    _Rectangle
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum TileValue {
    Empty,
    Rock,
    Error
}

#[derive(Clone, Debug, PartialEq)]
struct Sprite {
    shape: SpriteShape,
    color: Color
}

#[derive(Clone, Debug, PartialEq)]
struct TransformComponent {
    position: Vector,
    rotation: f32,
    scale: Vector
}

#[derive(Clone, Debug, PartialEq)]
struct KeyboardMove {
    speed: f32
}

#[derive(Clone, Debug, PartialEq)]
struct Camera {
    height: f32
}

struct TileMap {
    generator_func: HybridMulti,
    rock_density: f64
}

struct GameplayState {
    system: Ecs,
    world: TileMap,
    camera_id: EntityId
}

impl TileMap {
    fn new() -> TileMap {
        let generator_func = HybridMulti::new();
        TileMap { generator_func, rock_density: 0.5 }
    }

    fn sample(&self, x: i32, y: i32) -> TileValue {
        // TODO: Sample world edits list here

        // If no edits have been applied to this tile, sample the noise function to decide what goes here
        // Noise is from -1..1 but I only want 0..1 so shift it first
        let value = (self.generator_func.get([x as f64, y as f64]) + 1.0) / (2.0 + self.rock_density);
        match value.round() as i32 {
            0 => TileValue::Empty,
            1 => TileValue::Rock,
            _ => TileValue::Error
        }
    }

    fn draw(&self, window: &mut Window, view_box: &Rectangle) {
        // TODO: Optimize the shit out of this
        // TODO: Consider double size tiles at low zoom for LOD

        // Bounds to draw between
        let x_min = view_box.pos.x.floor() as i32;
        let x_max =(view_box.pos.x + view_box.size.x).ceil() as i32;
        let y_min = view_box.pos.y.floor() as i32;
        let y_max =(view_box.pos.y + view_box.size.y).ceil() as i32;

        // Draw one sprite rectangle for each tile within the bounds
        for x in x_min..x_max {
            for y in y_min..y_max {
                match self.sample(x, y) {
                    TileValue::Empty => window.draw(&Rectangle::new((x, y), (1, 1)), Col(Color::from_rgba(127, 127, 127, 1.0))),
                    TileValue::Rock => window.draw(&Rectangle::new((x, y), (1, 1)), Col(Color::from_rgba(227, 227, 227, 1.0))),
                    TileValue::Error => window.draw(&Rectangle::new((x, y), (1, 1)), Col(Color::MAGENTA))
                }
            }
        }
    }
}

fn draw(window: &mut Window, sprite: &Sprite, transform: &TransformComponent) {
    match sprite.shape {
        SpriteShape::_Circle => window.draw(&Circle::new(transform.position, transform.scale.x), Col(sprite.color)),
        SpriteShape::_Rectangle => window.draw(&Rectangle::new(transform.position, transform.scale), Col(sprite.color))
    }
}

impl State for GameplayState {
    fn new() -> Result<GameplayState> {
        let mut system = Ecs::new();
        let camera_ent: EntityId = system.create_entity();

        // Ignore result since this ID should be valid, we literally just made it
        let _ = system.set(camera_ent, TransformComponent { position: Vector::new(100, 100), rotation: 0.0, scale: Vector::new(100, 100) });
        let _ = system.set(camera_ent, KeyboardMove { speed: 2.5 });
        let _ = system.set(camera_ent, Camera { height: 10.0 });

        Ok( GameplayState{ system, world: TileMap::new(), camera_id: camera_ent } )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        //Prepare the camera
        // Calculate the aspect ratio of the display
        let screen_size = window.screen_size();
        let aspect_ratio = screen_size.x / screen_size.y;

        // Get the ids of components that have both a transform and a keyboard mover
        let camera: &Camera = self.system.borrow(self.camera_id).unwrap();
        let transform: &TransformComponent = self.system.borrow(self.camera_id).unwrap();
        let cam_rect = Rectangle::new(transform.position, (camera.height * aspect_ratio, camera.height));
        window.set_view(View::new(cam_rect));

        // Draw the tilemap first as a background
        self.world.draw(window, &cam_rect);

        // Get the ids of components that have both a transform and a sprite (everything needed to draw)
        let mut drawable_ids: Vec<EntityId> = Vec::new();
        let drawable_filter = component_filter!(Sprite, TransformComponent);
        self.system.collect_with(&drawable_filter, &mut drawable_ids);
        // Draw everything that we can draw
        for drawable in drawable_ids {
            let sprite: &Sprite = self.system.borrow(drawable).unwrap();
            let transform: &TransformComponent = self.system.borrow(drawable).unwrap();
            draw(window, sprite, transform);
        }

        Ok(())
    }

    fn update(&mut self, window: &mut Window) -> Result<()> {
        // Get change in time since last frame
        let framerate = window.current_fps();
        // First frame has framerate of 0 and that makes for a sad division time so catch that fucker here before it fucks everything up
        let delta_time = if framerate < 1.0 { 0.0 } else { 1.0 / framerate };

         // Get the ids of components that have both a transform and a keyboard mover
         let mut updatable_ids: Vec<EntityId> = Vec::new();
         let updatable_filter = component_filter!(KeyboardMove, TransformComponent);
         self.system.collect_with(&updatable_filter, &mut updatable_ids);
         for updateable in updatable_ids {
            let mover: &KeyboardMove = self.system.borrow(updateable).unwrap();
            let mut x_move = 0.0;
            let mut y_move = 0.0;

            if window.keyboard()[Key::W].is_down() { y_move -= mover.speed; }
            if window.keyboard()[Key::S].is_down() { y_move += mover.speed; }
            if window.keyboard()[Key::A].is_down() { x_move -= mover.speed; }
            if window.keyboard()[Key::D].is_down() { x_move += mover.speed; }
            
            x_move *= delta_time as f32;
            y_move *= delta_time as f32;

            if x_move != 0.0 {
                self.system.borrow_mut::<TransformComponent>(updateable).map(|transform| transform.position.x += x_move).unwrap();
            }
            if y_move != 0.0 {
                self.system.borrow_mut::<TransformComponent>(updateable).map(|transform| transform.position.y += y_move).unwrap();
            }
         }

        if window.keyboard()[Key::Q].is_down() {
            self.system.borrow_mut::<Camera>(self.camera_id).map(|cam| cam.height += delta_time as f32).unwrap();
        }
        if window.keyboard()[Key::E].is_down() {
            self.system.borrow_mut::<Camera>(self.camera_id).map(|cam| cam.height -= delta_time as f32).unwrap();
        }

        if window.keyboard()[Key::N].is_down() {
            self.world.rock_density -= delta_time;
            println!("Rock Density: {}", self.world.rock_density);
        }

        if window.keyboard()[Key::M].is_down() {
            self.world.rock_density += delta_time;
            println!("Rock Density: {}", self.world.rock_density);
        }

        Ok(())
    }
}

fn main() {
    run::<GameplayState>("Game Test", Vector::new(800, 600), Settings::default());
}