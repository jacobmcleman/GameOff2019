extern crate quicksilver;
#[macro_use]
extern crate recs;
use recs::{Ecs, EntityId};
use std::collections::HashMap;

extern crate tilemap;

use tilemap::tile_world::{
    TileMap, TileValue, GridCoord
};

use quicksilver::{
    Result,
    geom::{Circle, Rectangle, Vector, Transform},
    graphics::{Background::Col, Color, View},
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

struct GameplayState {
    system: Ecs,
    world: TileMap,
    camera_id: EntityId,
    tile_colors: HashMap<TileValue, Color>
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

        
        let tile_colors:  HashMap<TileValue, Color> = [
            (TileValue::Empty, Color::from_rgba(127, 127, 127, 1.0)),
            (TileValue::Rock, Color::from_rgba(227, 227, 227, 1.0)),
            (TileValue::Error, Color::MAGENTA)
        ].iter().cloned().collect();

        Ok( GameplayState{ system, world: TileMap::new(), camera_id: camera_ent, tile_colors } )
    }

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        window.clear(Color::BLACK)?;

        //Prepare the camera
        // Calculate the aspect ratio of the display
        let screen_size = window.screen_size();
        let aspect_ratio = screen_size.x / screen_size.y;

        // Feed the camera to the view controller on the window
        let camera: &Camera = self.system.borrow(self.camera_id).unwrap();
        let transform: &TransformComponent = self.system.borrow(self.camera_id).unwrap();
        let cam_rect = Rectangle::new(transform.position, (camera.height * aspect_ratio, camera.height));
        window.set_view(View::new(cam_rect));

        // Rectangle to reuse to maybe avoid constant re-allocation? Not actually sure if this is an optimization
        let rect = Rectangle::new_sized((1, 1));

        // Draw the tilemap first as a background
        self.world.for_each_tile(&cam_rect, |pos: &GridCoord, value: &TileValue| {
            let col: Color = match self.tile_colors.get(value) { Some(c) => c.clone(), _ => Color::MAGENTA };
            window.draw_ex(&rect, Col(col), Transform::translate((pos.x as f32, pos.y as f32)), 0);
        });
        
        // Draw a circle on the currently highlighted tile
        window.draw(&Circle::new((self.world.selected_tile.x as f32 + 0.5, self.world.selected_tile.y as f32 + 0.5), 0.5), Col(Color::RED));

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

        let selected_tile = self. world.pos_to_grid(window.mouse().pos().x, window.mouse().pos().y);

        if window.keyboard()[Key::Space].is_down() {
            self.world.make_change(&selected_tile, &TileValue::Error);
        }

        // Dont' store the selected tile position on the world until later because reading that to make a change requires reading from it 
        // but you can't do that at the same time as writing to the world with make change
        // (you totally could since the data is in different parts of the struct but this is rust and you can't make a mutable reference at the same time as an immutable one)
        self.world.selected_tile = selected_tile;

        Ok(())
    }
}

fn main() {
    run::<GameplayState>("Game Test", Vector::new(800, 600), Settings::default());
}