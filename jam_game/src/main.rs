extern crate quicksilver;
#[macro_use]
extern crate recs;
use recs::{Ecs, EntityId};
use noise::{NoiseFn, HybridMulti};
use std::collections::HashMap;

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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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

// Must be power of 2
const PARTITION_SIZE: u8 = (1 << 4);

struct AreaChanges {
    // TODO: Implement array mode for this structure for areas of dense change
    changes: HashMap<GridCoord, TileValue>
}

impl AreaChanges {
    fn new() -> AreaChanges {
        AreaChanges { changes: HashMap::new() }
    }

    fn sample(&self, pos: &GridCoord) -> Option<&TileValue> {
        // For now this just forwards the query to the internal hashmap
        // In future once a second storage type is available this will have to use the correct one
        self.changes.get(&pos)
    }

    fn add_change(&mut self, pos: &GridCoord, tile_value: &TileValue) {
        // Insert will overwrite old values with that key, so this is just always the correct option
        // TODO: add storage type switch here before insert
        self.changes.insert(pos.clone(), tile_value.clone());
    }
}

struct TileMap {
    generator_func: HybridMulti,
    rock_density: f64,
    tile_drawables: HashMap<TileValue, Color>,
    selected_tile: GridCoord,
    // TODO add map changes data structure here
    // Concept: Since changes will likely concentrated in a few areas, but there may be small changes all over the map
    // Spatial partition by zeroing out the last ~4 bits of a position (16x16 groups) and then 
    // for sparse changes (a few mined rocks) - do a hash table to find any changes within those 256 tiles (sparse storage, slower but less memory used)
    // for dense areas (a base) - Keep a 2D array of all 256 tiles in this chunk (and save that whole thing)
    // The exact switchover point should be tuned over time, but things to consider while doing that are:
    //      - switch should be before the point where hash collisions start being likely
    //      - should delay as much as sensible, 2D array will be much bigger memory hog and not scale well
    //      - the main tiles that a players base is on should definitely be in the array once it expands, so looking at common bases seems like a good way to tune this
    // Game saving thoughts: 
    //      - Could also use this partitioning to not load whole save files on start up, load more lazily
    //      - Alternatively, could ignore the partitioning for the save files to make it easier to tweak things like sizes and internal behavior later (don't save 2d arrays just a bunch o changes)
    map_changes: HashMap<GridCoord, AreaChanges>

    // TODO cache world sample queries
    // Re-generating untouched space and/or re-querying the changes data is expensive, so lets not do that every frame for every visible tile
    // Cache sizing still needs to be figured out - could be dynamic with camera size or just always big enough for max zoom
}

struct GameplayState {
    system: Ecs,
    world: TileMap,
    camera_id: EntityId
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
struct GridCoord {
    x: i64,
    y: i64
}

impl TileMap {
    fn new() -> TileMap {
        let generator_func = HybridMulti::new();

        let tile_drawables:  HashMap<TileValue, Color> = [
            (TileValue::Empty, Color::from_rgba(127, 127, 127, 1.0)),
            (TileValue::Rock, Color::from_rgba(227, 227, 227, 1.0)),
            (TileValue::Error, Color::MAGENTA)
        ].iter().cloned().collect();

        TileMap { generator_func, rock_density: 0.5, tile_drawables, selected_tile: GridCoord{ x: 0, y: 0 }, map_changes: HashMap::new() }
    }

    fn sample(&self, pos: &GridCoord) -> &TileValue {
        // Unwrap values from struct
        let x = pos.x;
        let y = pos.y;

        // Mask away the bits 
        let partition_x = x & !(PARTITION_SIZE as i64 - 1);
        let partition_y = y & !(PARTITION_SIZE as i64 - 1);
        let partition_coord = GridCoord { x: partition_x, y: partition_y };

        // Check the history for a matching change
        // First see if there is any changes within this tiles partition
        if self.map_changes.contains_key(&partition_coord)  {
            // Ask the partition if there is a value for this tile
            let tile_value: Option<&TileValue> = self.map_changes.get(&partition_coord).unwrap().sample(pos);
            if tile_value.is_some() {
                // There is a changed value in this tile, use that
                return tile_value.unwrap();
            }
        }

        // If no edits have been applied to this tile, sample the noise function to decide what goes here
        // Noise is from -1..1 but I only want 0..1 so shift it first
        let value = ((self.generator_func.get([x as f64, y as f64]) + 1.0) / (2.0 + self.rock_density)).round();
        let value = if value > 1.0 { 1.0 } else if value < 0.0 { 0.0 } else { value };
        match value as i32 {
            0 => &TileValue::Empty,
            1 => &TileValue::Rock,
            _ => &TileValue::Error
        }
    }

    fn draw(&self, window: &mut Window, view_box: &Rectangle) {
        // TODO: Optimize the shit out of this
        /* Ideas for this: 
            - Don't need to resample noise very frame since most of the tiles are the same, only need to sample on the edges or when there is a change
            - Don't need to make a new transform every frame either, same reason
            - Use faster noise function
            - Is the color copy slow?
        */
        // TODO: Consider double size tiles at low zoom for LOD

        // Bounds to draw between
        let x_min = view_box.pos.x.floor() as i64;
        let x_max =(view_box.pos.x + view_box.size.x).ceil() as i64;
        let y_min = view_box.pos.y.floor() as i64;
        let y_max =(view_box.pos.y + view_box.size.y).ceil() as i64;

        // Rectangle to reuse to maybe avoid constant re-allocation?
        let rect = Rectangle::new_sized((1, 1));
        
        // Draw one sprite rectangle for each tile within the bounds
        for x in x_min..x_max {
            for y in y_min..y_max {
                let coord = GridCoord {x, y};
                let col: Color = match self.tile_drawables.get(self.sample(&coord)) { Some(c) => c.clone(), _ => Color::MAGENTA };
                window.draw_ex(&rect, Col(col), Transform::translate((x as f32, y as f32)), 0);
            }
        }

        // Draw a circle on the currently highlighted tile
        window.draw(&Circle::new((self.selected_tile.x as f32 + 0.5, self.selected_tile.y as f32 + 0.5), 0.5), Col(Color::RED));
    }

    fn  pos_to_grid(&self, world_x: f32 , world_y: f32) -> GridCoord {
        GridCoord { x: world_x as i64, y: world_y as i64}
    }

    fn make_change(&mut self, pos: &GridCoord, new_value: &TileValue) {
        // Unwrap values from struct
        let x = pos.x;
        let y = pos.y;

        // Mask away the bits 
        let partition_x = x & !(PARTITION_SIZE as i64 - 1);
        let partition_y = y & !(PARTITION_SIZE as i64 - 1);
        let partition_coord = GridCoord { x: partition_x, y: partition_y };

        // First ensure there is a change table for this partition
        if !self.map_changes.contains_key(&partition_coord)  {
            self.map_changes.insert(partition_coord, AreaChanges::new());
        }

        // Safe to unwrap immediately because we know at this point the key is in the table
        let partition_changes = self.map_changes.get_mut(&partition_coord).unwrap();
        partition_changes.add_change(pos, new_value);
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

        // Feed the camera to the view controller on the window
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