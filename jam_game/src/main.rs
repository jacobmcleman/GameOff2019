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
    graphics::{Background::Col, Background::Img, Color, View, Image},
    input::{Key, MouseButton},
    lifecycle::{Settings, State, Window, Asset, run},
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
    tile_textures: HashMap<TileValue, Image>,
    _tile_cursor: Asset<Image>,
    empty_asset: Asset<Image>,
    hab_asset: Asset<Image>,
    rock_asset: Asset<Image>,
    selected_tile: GridCoord,
    can_place: bool
}

fn draw(window: &mut Window, sprite: &Sprite, transform: &TransformComponent) {
    match sprite.shape {
        SpriteShape::_Circle => window.draw(&Circle::new(transform.position, transform.scale.x), Col(sprite.color)),
        SpriteShape::_Rectangle => window.draw(&Rectangle::new(transform.position, transform.scale), Col(sprite.color))
    }
}

fn draw_tile(window: &mut Window, tile_textures: &HashMap<TileValue, Image>, pos: &GridCoord, value: &TileValue, size: &GridCoord) {
        let rect = Rectangle::new_sized((1, 1)); 
        match value {
            TileValue::Subtile(_) => {}, // Don't render subtiles
            _ => {
                let transform = Transform::translate((pos.x as f32, pos.y as f32)) * Transform::scale((size.x as f32, size.y as f32));
                match tile_textures.get(value) {
                    Some(image) => window.draw_ex(&rect, Img(&image), transform, 0),
                    None => window.draw_ex(&rect, Col(Color::MAGENTA), transform, 0)
                };
            }
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
        
        let tile_textures:  HashMap<TileValue, Image> = HashMap::new();

        let empty_asset = Asset::new(Image::load("tile_textures/empty.png"));
        let hab_asset = Asset::new(Image::load("tile_textures/hab.png"));
        let rock_asset = Asset::new(Image::load("tile_textures/rock.png"));

        Ok( GameplayState{ 
            system, world: 
            TileMap::new(), 
            camera_id: camera_ent, 
            tile_textures, 
            _tile_cursor: Asset::new(Image::load("selection.png")),
            empty_asset,
            hab_asset,
            rock_asset,
            selected_tile: GridCoord{x: 0, y: 0},
            can_place: false
        } )
    }

      

    fn draw(&mut self, window: &mut Window) -> Result<()> {
        // Load images we don't have yet if they're ready
        let mut newly_loaded_assets: HashMap<TileValue, Image> = HashMap::new();
        if !self.tile_textures.contains_key(&TileValue::Empty) {
            self.empty_asset.execute(|image| { newly_loaded_assets.insert(TileValue::Empty, image.clone()); Ok(()) })?;
        }
        if !self.tile_textures.contains_key(&TileValue::Rock) {
            self.rock_asset.execute(|image| { newly_loaded_assets.insert(TileValue::Rock, image.clone()); Ok(()) })?;
        }
        if !self.tile_textures.contains_key(&TileValue::HabModule) {
            self.hab_asset.execute(|image| { newly_loaded_assets.insert(TileValue::HabModule, image.clone()); Ok(()) })?;
        }
        if !newly_loaded_assets.is_empty() {
            for (key, val) in newly_loaded_assets.iter() {
                self.tile_textures.insert(*key, val.clone());
            }
        }

        window.clear(Color::BLACK)?;

        //Prepare the camera
        // Calculate the aspect ratio of the displaysa
        let screen_size = window.screen_size();
        let aspect_ratio = screen_size.x / screen_size.y;

        // Feed the camera to the view controller on the window
        let camera: &Camera = self.system.borrow(self.camera_id).unwrap();
        let transform: &TransformComponent = self.system.borrow(self.camera_id).unwrap();
        let cam_rect = Rectangle::new(transform.position, (camera.height * aspect_ratio, camera.height));
        window.set_view(View::new(cam_rect));

        // Draw the tilemap first as a background
        self.world.for_each_tile_rect(&cam_rect, |pos: &GridCoord, value: &TileValue, size: &GridCoord| {
            draw_tile(window, &self.tile_textures, pos, value, size);
        });
        
        // Draw a circle on the currently highlighted tile
        if self.can_place {
            window.draw_ex(
                &Circle::new((0, 0), 1.5), 
                Col(Color::GREEN),
                Transform::translate((self.selected_tile.x as f32 + 0.5, self.selected_tile.y as f32 + 0.5)),
                1
                );
        }
        else {
            window.draw_ex(
                &Circle::new((0, 0), 0.5), 
                Col(Color::RED),
                Transform::translate((self.selected_tile.x as f32 + 0.5, self.selected_tile.y as f32 + 0.5)),
                1
                );
        }

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

        self.selected_tile = self.world.pos_to_grid(window.mouse().pos().x, window.mouse().pos().y);
        let selection_area_left = self.selected_tile.x - 1;
        let selection_area_top = self.selected_tile.y - 1;

        self.can_place = self.world.area_clear(&GridCoord{x: selection_area_left, y: selection_area_top}, &GridCoord{x: 3, y: 3});

        if window.mouse()[MouseButton::Left].is_down() && self.can_place {
            self.world.make_change(&self.selected_tile, &TileValue::HabModule);
        }

        Ok(())
    }
}

fn main() {
    run::<GameplayState>("Game Test", Vector::new(800, 600), Settings::default());
}