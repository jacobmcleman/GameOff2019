extern crate quicksilver;

pub mod tile_world {
    use noise::{NoiseFn, HybridMulti};
    use std::collections::HashMap;

    use quicksilver::{
        geom::{Circle, Rectangle, Transform},
        graphics::{Background::Col, Color},
        lifecycle::{Window},
    };

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct GridCoord {
        pub x: i64,
        pub y: i64
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum TileValue {
        Empty,
        Rock,
        Error
    }
    
    // Must be power of 2
    const PARTITION_SIZE: u8 = (1 << 4);

    pub struct AreaChanges {
        // TODO: Implement array mode for this structure for areas of dense change
        changes: HashMap<GridCoord, TileValue>
    }

    pub struct TileMap {
        pub rock_density: f64,
        pub selected_tile: GridCoord,
        generator_func: HybridMulti,
        tile_drawables: HashMap<TileValue, Color>,
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

    impl TileMap {
        pub fn new() -> TileMap {
            let generator_func = HybridMulti::new();

            let tile_drawables:  HashMap<TileValue, Color> = [
                (TileValue::Empty, Color::from_rgba(127, 127, 127, 1.0)),
                (TileValue::Rock, Color::from_rgba(227, 227, 227, 1.0)),
                (TileValue::Error, Color::MAGENTA)
            ].iter().cloned().collect();

            TileMap { generator_func, rock_density: 0.5, tile_drawables, selected_tile: GridCoord{ x: 0, y: 0 }, map_changes: HashMap::new() }
        }

        pub fn sample(&self, pos: &GridCoord) -> &TileValue {
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

        pub fn draw(&self, window: &mut Window, view_box: &Rectangle) {
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

        pub fn pos_to_grid(&self, world_x: f32 , world_y: f32) -> GridCoord {
            GridCoord { x: world_x as i64, y: world_y as i64}
        }

        pub fn make_change(&mut self, pos: &GridCoord, new_value: &TileValue) {
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
}

#[cfg(test)]
mod tests {
    use crate::tile_world::{
        TileMap, TileValue, GridCoord
    };

    #[test]
    fn empty_map_access_gives_empty() {
        let map = TileMap::new();
        assert_eq!(map.sample(&GridCoord{x: 0, y: 0}), &TileValue::Empty);
    }

    #[test]
    fn map_write_get_back() {
        let mut map = TileMap::new();
        map.make_change(&GridCoord{x: 0, y: 0}, &TileValue::Error);
        assert_eq!(map.sample(&GridCoord{x: 0, y: 0}), &TileValue::Error);
    }

    #[test]
    fn pos_to_grid() {
        let map = TileMap::new();
        assert_eq!(map.pos_to_grid(0.0, 0.0), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(0.1, 0.1), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(-0.1, -0.1), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(0.6, -0.6), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(1.0, 1.0), GridCoord{x: 1, y: 1});
    }
}
