extern crate quicksilver;
extern crate lru;

pub mod tile_world {
    use noise::{NoiseFn, HybridMulti};
    use std::collections::HashMap;
    use quicksilver::geom::Rectangle;

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct GridCoord {
        pub x: i64,
        pub y: i64
    }

    #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
    pub enum TileValue {
        Empty,
        Rock,
        Error,
        HabModule,

        Subtile(GridCoord), // Subtiles have a GridCoord that points at the true position of the metatile 
        InternalUnknown // Special value for when using dense storage for values that have not yet been computed
    }

    // Must be power of 2
    pub const PARTITION_SIZE: u8 = (1 << 4);

    // Length of table at which the storage mode should switch to dense storage
    pub const DENSE_SWITCH_POINT: u32 = ((PARTITION_SIZE as u32) * (PARTITION_SIZE as u32)) / 3;

    pub struct AreaChanges {
        // TODO: Implement array mode for this structure for areas of dense change
        changes_map: HashMap<u16, TileValue>,
        changes_vec: Vec<TileValue>,
        using_dense_storage: bool
    }

    pub struct TileMap {
        pub rock_density: f64,
        generator_func: HybridMulti,
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
        map_changes: HashMap<GridCoord, AreaChanges>,
        // TODO: figure out a way of re-enabling caching behavior without making everything be mutable
        // Re-generating untouched space and/or re-querying the changes data is expensive, so lets not do that every frame for every visible tile
        // Cache sizing still needs to be figured out - could be dynamic with camera size or just always big enough for max zoom
        // tile_cache: LruCache<GridCoord, TileValue>,
        // caching_enabled: bool,
        // The x/y size of tiles in grid coordinates
        // If a tile type is not in this list, it is assumed to be 1x1
        // When a tile of a given size is placed it will automatically set all tiles within its area to subtiles
        // When it is removed all tiles within that area become "Empty"
        tile_type_sizes: HashMap<TileValue, GridCoord> 
    }

    impl AreaChanges {
        pub fn new() -> AreaChanges {
            AreaChanges { 
                changes_map: HashMap::new(), 
                changes_vec: Vec::new(), 
                using_dense_storage: false 
            }
        }

        pub fn sample(&self, pos: &GridCoord) -> Option<TileValue> {
            let internal_pos_x = (pos.x & (PARTITION_SIZE as i64 - 1)) as u8;
            let internal_pos_y = (pos.y & (PARTITION_SIZE as i64 - 1)) as u8;

            if self.using_dense_storage {
                let index = internal_pos_x as usize + ((PARTITION_SIZE as usize) * (internal_pos_y as usize));
                let lookup_result = self.changes_vec[index];
                if lookup_result == TileValue::InternalUnknown { None }
                else { Some(lookup_result) }
            }
            else {
                let internal_key = ((internal_pos_x as u16) << 8) | (internal_pos_y as u16);
                // For now this just forwards the query to the internal hashmap
                // In future once a second storage type is available this will have to use the correct one
                match self.changes_map.get(&internal_key) {
                    Some(key) => Some(key.clone()),
                    None => None
                }
            }
        }

        pub fn add_change(&mut self, pos: &GridCoord, tile_value: &TileValue) {
            let internal_pos_x = (pos.x & (PARTITION_SIZE as i64 - 1)) as u8;
            let internal_pos_y = (pos.y & (PARTITION_SIZE as i64 - 1)) as u8;

            if self.using_dense_storage {
                let index = internal_pos_x as usize + ((PARTITION_SIZE as usize) * (internal_pos_y as usize));
                self.changes_vec[index] = *tile_value;
            }
            else {
                if self.changes_map.len() > DENSE_SWITCH_POINT as usize {
                    self.switch_to_dense();
                    // Mode switched, go back around
                    self.add_change(pos, tile_value);
                }
                else {
                    let internal_key = ((internal_pos_x as u16) << 8) | (internal_pos_y as u16);

                    // Insert will overwrite old values with that key, so this is just always the correct option
                    self.changes_map.insert(internal_key, tile_value.clone());
                }
            }
        }

        fn switch_to_dense(&mut self) {
            if self.using_dense_storage { return; }

            self.changes_vec.resize((PARTITION_SIZE as usize) * (PARTITION_SIZE as usize), TileValue::InternalUnknown);

            for (key, val) in self.changes_map.iter() {
                let internal_pos_x = key >> 8;
                let internal_pos_y = key & ((1 << 8) - 1);
                let index = internal_pos_x as usize + ((PARTITION_SIZE as usize) * (internal_pos_y as usize));
                self.changes_vec[index] = *val;
            }

            self.changes_map.clear();
            self.changes_map.shrink_to_fit();
            self.using_dense_storage = true;
        }

        fn _switch_to_sparse(&mut self) {
            if !self.using_dense_storage { return; }

            for x in 0..PARTITION_SIZE {
                for y in 0..PARTITION_SIZE {
                    let index = x as usize + ((PARTITION_SIZE as usize) * (y as usize));
                    let internal_key = ((x as u16) << 8) | (y as u16);
                    self.changes_map.insert(internal_key, self.changes_vec[index]);
                }
            }

            self.changes_vec.clear();
            self.changes_vec.shrink_to_fit();
            self.using_dense_storage = false;
        }
    }

    impl TileMap {
        pub fn new() -> TileMap {
            let generator_func = HybridMulti::new();

            let mut tile_type_sizes: HashMap<TileValue, GridCoord> = HashMap::new();
            tile_type_sizes.insert(TileValue::HabModule, GridCoord{x: 3, y: 3});

            TileMap { 
                generator_func, 
                rock_density: 0.25, 
                map_changes: HashMap::new(), 
                // tile_cache: LruCache::new(256),
                // caching_enabled: true,
                tile_type_sizes
            }
        }

        pub fn sample(&self, pos: &GridCoord) -> TileValue {
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
                let tile_value: Option<TileValue> = self.map_changes.get(&partition_coord).unwrap().sample(pos);
                if tile_value.is_some() {
                    // There is a changed value in this tile, use that
                    return tile_value.unwrap();
                }
            }

            // If no edits have been applied to this tile, sample the noise function to decide what goes here
            // Noise is from -1..1 but I only want 0..1 so shift it first
            let value = ((self.generator_func.get([x as f64, y as f64]) + 1.0) / (2.0 + self.rock_density)).round();
            let value = if value > 1.0 { 1.0 } else if value < 0.0 { 0.0 } else { value };
            let tile_val = match value as i32 {
                0 => TileValue::Empty,
                1 => TileValue::Rock,
                _ => TileValue::Error
            };

            return tile_val;
        }

        pub fn area_clear(&mut self, top_left: &GridCoord, size: &GridCoord) -> bool {
            let x_min = top_left.x;
            let x_max = top_left.x + size.x;
            let y_min = top_left.y;
            let y_max = top_left.y + size.y;

            let mut clear = true;

            for y in y_min..y_max {
                for x in x_min..x_max {
                    clear = clear && (self.sample(&GridCoord{x, y}) == TileValue::Empty);

                    if !clear { return clear; }
                }
            }

            return clear;
        }


        pub fn for_each_tile_rect<F>(&self, bounds: &Rectangle, func: F)
            where F : FnMut(&GridCoord, &TileValue, &GridCoord) {
            // Bounds to draw between
            let x_min = bounds.pos.x.floor() as i64;
            let x_size = bounds.size.x.ceil() as i64 + 1;
            let y_min = bounds.pos.y.floor() as i64;
            let y_size = bounds.size.y.ceil() as i64 + 1;
            
            self.for_each_tile(&GridCoord{x: x_min, y: y_min}, &GridCoord{x: x_size, y: y_size}, func)
        }

        pub fn for_each_tile<F>(&self, top_left: &GridCoord, size: &GridCoord, mut func: F)
            where F : FnMut(&GridCoord, &TileValue, &GridCoord) {
            // Bounds to draw between
            let x_min = top_left.x;
            let x_max = top_left.x + size.x;
            let y_min = top_left.y;
            let y_max = top_left.y + size.y;

            // Call func once for each tile within the bounds
            for y in y_min..y_max {
                for x in x_min..x_max {
                    let coord = GridCoord {x, y};
                    let tile_value = self.sample(&coord);
                    let size = self.get_tile_size(&tile_value);
                    func(&coord, &tile_value, &size);
                }
            }
        }

        pub fn pos_to_grid(&mut self, world_x: f32 , world_y: f32) -> GridCoord {
            let pos = GridCoord { x: world_x as i64, y: world_y as i64};
            match self.sample(&pos) {
                TileValue::Subtile(ref_position) => ref_position,
                _ => pos
            }
        }

        pub fn make_change(&mut self, pos: &GridCoord, new_value: &TileValue) {
            let old_value = self.sample(pos);
            let old_tile_size = self.get_tile_size(&old_value);

            if old_tile_size.x > 1 && old_tile_size.y > 1 {
                let x_min = pos.x - (old_tile_size.x / 2);
                let y_min = pos.y - (old_tile_size.y / 2);

                self.set_area(&GridCoord{x: x_min, y: y_min}, &old_tile_size, TileValue::Empty );
            }

            let tile_size = self.get_tile_size(new_value);

            let x_min = pos.x - (tile_size.x / 2);
            let y_min = pos.y - (tile_size.y / 2);

            self.set_area(&GridCoord{x: x_min, y: y_min}, &tile_size, TileValue::Subtile(*pos) );
            self.make_single_tile_change(&pos, *new_value);
        }

        pub fn set_area(&mut self, top_left: &GridCoord, size: &GridCoord, new_value: TileValue) {
            let x_min = top_left.x;
            let y_min = top_left.y;

            let x_max = x_min + size.x;
            let y_max = y_min + size.y;

            for y in y_min..y_max {
                for x in x_min..x_max {
                    self.make_single_tile_change(&GridCoord{x, y}, new_value);
                }
            }
        }

        fn make_single_tile_change(&mut self, pos: &GridCoord, new_value: TileValue) {
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
            partition_changes.add_change(pos, &new_value);
        }

        pub fn get_tile_size(&self, tile_type: &TileValue) -> GridCoord {
            match self.tile_type_sizes.get(&tile_type) {
                Some(size) => *size,
                None => GridCoord{x: 1, y: 1}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::tile_world::{
        TileMap, TileValue, GridCoord, AreaChanges, PARTITION_SIZE
    };

    use quicksilver::{
        geom::{Rectangle},
    };

    fn is_valid_generated_tile(value: &TileValue) -> bool {
        value == &TileValue::Empty || value == &TileValue::Rock
    }

    #[test]
    fn empty_map_access_gives_valid() {
        let map = TileMap::new();
        assert!(is_valid_generated_tile(&map.sample(&GridCoord{x: 0, y: 0})));
    }

    #[test]
    fn untouched_map_no_errors() {
        let map = TileMap::new();
        
        // Check the 1 million tiles closest to origin
        let x_min: i64 = -500;
        let x_max: i64 = 500;
        let y_min: i64 = -500;
        let y_max: i64 = 500;

        for x in x_min..x_max {
            for y in y_min..y_max {
                assert!(is_valid_generated_tile(&map.sample(&GridCoord{x, y})), "Found invalid tile at ({}, {})", x, y);
            }
        }
    }

    #[test]
    fn map_write_get_back_1() {
        let mut map = TileMap::new();
        map.make_change(&GridCoord{x: 0, y: 0}, &TileValue::Error);
        assert_eq!(map.sample(&GridCoord{x: 0, y: 0}), TileValue::Error);
    }

    #[test]
    fn map_write_get_back_dense_100() {
        let mut map = TileMap::new();

        // Large bounds but still within a single cache
        let x_min: i64 = -5;
        let x_max: i64 = 5;
        let y_min: i64 = -5;
        let y_max: i64 = 5;
        
        for x in x_min..x_max {
            for y in y_min..y_max {
                map.make_change(&GridCoord{x, y}, &TileValue::Error);
            }
        }

        for x in x_min..x_max {
            for y in y_min..y_max {
                assert_eq!(map.sample(&GridCoord{x, y}), TileValue::Error);
            }
        }
    }

    #[test]
    fn map_write_get_back_sparse_100() {
        let mut map = TileMap::new();

        // Large bounds but still within a single cache
        let x_min: i64 = -5;
        let x_max: i64 = 5;
        let y_min: i64 = -5;
        let y_max: i64 = 5;
        
        for x in x_min..x_max {
            for y in y_min..y_max {
                map.make_change(&GridCoord{x: 100 * x, y: 100 * y}, &TileValue::Error);
            }
        }

        for x in x_min..x_max {
            for y in y_min..y_max {
                assert_eq!(map.sample(&GridCoord{x: 100 * x, y: 100 * y}), TileValue::Error);
            }
        }
    }

    #[test]
    fn map_write_get_back_10000() {
        let mut map = TileMap::new();

        // Bounds big enough to be waaay beyond the cache
        let x_min: i64 = -50;
        let x_max: i64 = 50;
        let y_min: i64 = -50;
        let y_max: i64 = 50;
        
        for x in x_min..x_max {
            for y in y_min..y_max {
                map.make_change(&GridCoord{x, y}, &TileValue::Error);
            }
        }

        for x in x_min..x_max {
            for y in y_min..y_max {
                assert_eq!(map.sample(&GridCoord{x, y}), TileValue::Error);
            }
        }
    }

    #[test]
    fn pos_to_grid() {
        let mut map = TileMap::new();
        assert_eq!(map.pos_to_grid(0.0, 0.0), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(0.1, 0.1), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(-0.1, -0.1), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(0.6, -0.6), GridCoord{x: 0, y: 0});
        assert_eq!(map.pos_to_grid(1.0, 1.0), GridCoord{x: 1, y: 1});
    }

    #[test]
    fn pos_to_grid_subtiles() {
        let mut map = TileMap::new();
        map.make_change(&GridCoord{x: 0, y: 0}, &TileValue::Subtile(GridCoord{x: 1, y: 0}));
        assert_eq!(map.pos_to_grid(0.0, 0.0), GridCoord{x: 1, y: 0});
    }

    #[test]
    fn for_each_tile_bounds_gets_all() {
        let map = TileMap::new();
        // Create a rectangle from (0, 0) to (10, 10)
        let bounds = Rectangle::new_sized((10, 10));
        let mut tiles_hit: u32 = 0;

        let min_val = 0;
        let max_val = 10;

        map.for_each_tile_rect(&bounds, |pos: &GridCoord, _value: &TileValue, _size: &GridCoord| {
            tiles_hit += 1;
            assert!(pos.x >= min_val, "Expected X greater than {}, got {}", min_val, pos.x);
            assert!(pos.x <= max_val, "Expected X less than {}, got {}", max_val, pos.x);
            assert!(pos.y >= min_val, "Expected Y greater than {}, got {}", min_val, pos.y);
            assert!(pos.y <= max_val, "Expected Y less than {}, got {}", max_val, pos.y);
        });

        // One row past the end should be hit, so actual bounds are 11x11
        assert_eq!(tiles_hit, 121);
    }

    #[test]
    fn sample_from_switched_dense() {
        let mut partition = AreaChanges::new();

        // Change 3/4 of the tiles
        for x in 0..PARTITION_SIZE as i64 {
            for y in 0..PARTITION_SIZE as i64 {
                if x % 4 == 0 {
                    partition.add_change(&GridCoord{x, y}, &TileValue::Empty);
                }
            }
        }

        // Test all the tiles
        for x in 0..PARTITION_SIZE as i64 {
            for y in 0..PARTITION_SIZE as i64 {
                if x % 4 == 0 {
                    assert_eq!(partition.sample(&GridCoord{x, y}), Some(TileValue::Empty), "Tile value at ({}, {}) lost!", x, y);
                }
                else {
                    assert_eq!(partition.sample(&GridCoord{x, y}), None, "Tile value at ({}, {}) invented from nothing!", x, y);
                }
            }
        }
    }

    #[test]
    fn setting_large_object_works() {
        let mut map = TileMap::new();

        map.make_change(&GridCoord{x: 1, y: 1}, &TileValue::HabModule);

        for x in -2..5 {
            for y in -2..5 {
                let value_here = map.sample(&GridCoord{x, y});
                if x == 1 && y == 1 {
                    assert_eq!(value_here, TileValue::HabModule);
                }
                else if x >= 0 && x < 3 && y >= 0 && y < 3 {
                    assert_eq!(value_here, TileValue::Subtile(GridCoord{x: 1, y: 1}));
                }
                else {
                    assert!(is_valid_generated_tile(&value_here), 
                        "Expected untouched tile at ({}, {}), found not that",
                        x, y
                    );
                }
            }
        }
    }

    #[test]
    fn revoving_large_object_works() {
        let mut map = TileMap::new();

        map.make_change(&GridCoord{x: 1, y: 1}, &TileValue::HabModule);
        map.make_change(&GridCoord{x: 1, y: 1}, &TileValue::Empty);

        for x in -2..5 {
            for y in -2..5 {
                let value_here = map.sample(&GridCoord{x, y});
                if x >= 0 && x < 3 && y >= 0 && y < 3 {
                    assert_eq!(value_here, TileValue::Empty);
                }
                else {
                    assert!(is_valid_generated_tile(&value_here), 
                        "Expected untouched tile at ({}, {}), found not that",
                        x, y
                    );
                }
            }
        }
    }

    #[test]
    fn clear_space_is_clear() {
        let mut map = TileMap::new();

        map.set_area(&GridCoord{x: -3, y: -6}, &GridCoord{x: 9, y: 9}, TileValue::Rock);
        map.set_area(&GridCoord{x: 0, y: 0}, &GridCoord{x: 3, y: 3}, TileValue::Empty);

        assert_eq!(map.area_clear(&GridCoord{x: 0, y: 0}, &GridCoord{x: 3, y: 3}), true, "Clear area wasn't");
        assert_eq!(map.area_clear(&GridCoord{x: -1, y: 1}, &GridCoord{x: 3, y: 3}), false, "Unclear area wasn't");
        assert_eq!(map.area_clear(&GridCoord{x: 1, y: 1}, &GridCoord{x: 3, y: 3}), false, "Unclear area wasn't");
        assert_eq!(map.area_clear(&GridCoord{x: 1, y: -1}, &GridCoord{x: 3, y: 3}), false, "Unclear area wasn't");
        assert_eq!(map.area_clear(&GridCoord{x: -1, y: 1}, &GridCoord{x: 3, y: 3}), false, "Unclear area wasn't");
    }
}
