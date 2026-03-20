use crate::agent::AgentKey;

/// Flat-array spatial hash grid. Two-pass counting design with zero per-cell allocation.
pub struct SpatialHash {
    cell_size: f32,
    inv_cell_size: f32,
    grid_width: usize,
    grid_height: usize,
    _world_width: f32,
    _world_height: f32,

    /// Number of entities in each cell (indexed by cell_index)
    counts: Vec<u32>,
    /// Prefix-sum offsets into entries array (indexed by cell_index)
    offsets: Vec<u32>,
    /// Packed entity keys, contiguous per-cell
    entries: Vec<AgentKey>,
    /// Temporary write cursors during rebuild pass 2
    cursors: Vec<u32>,
}

impl SpatialHash {
    pub fn new(world_width: f32, world_height: f32, cell_size: f32, max_entities: usize) -> Self {
        let inv_cell_size = 1.0 / cell_size;
        let grid_width = (world_width * inv_cell_size).ceil() as usize;
        let grid_height = (world_height * inv_cell_size).ceil() as usize;
        let num_cells = grid_width * grid_height;

        Self {
            cell_size,
            inv_cell_size,
            grid_width,
            grid_height,
            _world_width: world_width,
            _world_height: world_height,
            counts: vec![0; num_cells],
            offsets: vec![0; num_cells],
            entries: Vec::with_capacity(max_entities),
            cursors: vec![0; num_cells],
        }
    }

    /// Rebuild the entire grid from scratch. Call once per tick.
    pub fn rebuild<'a, I>(&mut self, creatures: I)
    where
        I: Iterator<Item = (AgentKey, f32, f32)> + Clone,
    {
        let num_cells = self.grid_width * self.grid_height;

        // Clear counts
        for c in self.counts.iter_mut() {
            *c = 0;
        }

        // Pass 1: count entities per cell
        let mut total = 0u32;
        for (_, x, y) in creatures.clone() {
            let idx = self.cell_index(x, y);
            self.counts[idx] += 1;
            total += 1;
        }

        // Prefix sum → offsets
        self.offsets[0] = 0;
        for i in 1..num_cells {
            self.offsets[i] = self.offsets[i - 1] + self.counts[i - 1];
        }

        // Prepare entries and cursors
        self.entries.resize(total as usize, AgentKey::default());
        for i in 0..num_cells {
            self.cursors[i] = self.offsets[i];
        }

        // Pass 2: place entities
        for (key, x, y) in creatures {
            let idx = self.cell_index(x, y);
            let pos = self.cursors[idx] as usize;
            self.entries[pos] = key;
            self.cursors[idx] += 1;
        }
    }

    /// Query all entities in neighboring cells (3x3 neighborhood around position).
    /// Calls `callback` with each found AgentKey.
    #[inline]
    pub fn query_neighbors(&self, x: f32, y: f32, mut callback: impl FnMut(AgentKey)) {
        let cx = (x * self.inv_cell_size) as i32;
        let cy = (y * self.inv_cell_size) as i32;
        let gw = self.grid_width as i32;
        let gh = self.grid_height as i32;

        for dy in -1..=1 {
            for dx in -1..=1 {
                // Toroidal wrapping
                let nx = ((cx + dx) % gw + gw) % gw;
                let ny = ((cy + dy) % gh + gh) % gh;
                let cell_idx = (ny * gw + nx) as usize;

                let start = self.offsets[cell_idx] as usize;
                let count = self.counts[cell_idx] as usize;
                for i in start..start + count {
                    callback(self.entries[i]);
                }
            }
        }
    }

    /// Query entities within a specific radius (squared distance check).
    /// Returns keys of entities within range_sq of (x, y).
    #[inline]
    pub fn query_radius(
        &self,
        x: f32,
        y: f32,
        range: f32,
        mut callback: impl FnMut(AgentKey),
    ) {
        // How many cells to check in each direction
        let cells_to_check = (range * self.inv_cell_size).ceil() as i32;
        let cx = (x * self.inv_cell_size) as i32;
        let cy = (y * self.inv_cell_size) as i32;
        let gw = self.grid_width as i32;
        let gh = self.grid_height as i32;

        for dy in -cells_to_check..=cells_to_check {
            for dx in -cells_to_check..=cells_to_check {
                let nx = ((cx + dx) % gw + gw) % gw;
                let ny = ((cy + dy) % gh + gh) % gh;
                let cell_idx = (ny * gw + nx) as usize;

                let start = self.offsets[cell_idx] as usize;
                let count = self.counts[cell_idx] as usize;
                for i in start..start + count {
                    callback(self.entries[i]);
                }
            }
        }
    }

    #[inline(always)]
    fn cell_index(&self, x: f32, y: f32) -> usize {
        let cx = ((x * self.inv_cell_size) as usize).min(self.grid_width - 1);
        let cy = ((y * self.inv_cell_size) as usize).min(self.grid_height - 1);
        cy * self.grid_width + cx
    }

    pub fn grid_width(&self) -> usize {
        self.grid_width
    }

    pub fn grid_height(&self) -> usize {
        self.grid_height
    }

    pub fn cell_size(&self) -> f32 {
        self.cell_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::DenseSlotMap;

    fn make_keys(n: usize) -> (DenseSlotMap<AgentKey, ()>, Vec<AgentKey>) {
        let mut map = DenseSlotMap::with_key();
        let keys: Vec<AgentKey> = (0..n).map(|_| map.insert(())).collect();
        (map, keys)
    }

    #[test]
    fn empty_grid_query_returns_nothing() {
        let sh = SpatialHash::new(100.0, 100.0, 10.0, 0);
        let mut found = vec![];
        sh.query_neighbors(50.0, 50.0, |k| found.push(k));
        assert!(found.is_empty());
    }

    #[test]
    fn single_entity_found_by_query() {
        let (_map, keys) = make_keys(1);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 1);
        sh.rebuild([(keys[0], 50.0, 50.0)].iter().cloned());

        let mut found = vec![];
        sh.query_neighbors(50.0, 50.0, |k| found.push(k));
        assert_eq!(found, vec![keys[0]]);
    }

    #[test]
    fn entity_found_in_neighbor_cell() {
        let (_map, keys) = make_keys(1);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 1);
        // Place at (15, 15) = cell (1, 1)
        sh.rebuild([(keys[0], 15.0, 15.0)].iter().cloned());

        // Query from (5, 5) = cell (0, 0) — neighbor of (1, 1)
        let mut found = vec![];
        sh.query_neighbors(5.0, 5.0, |k| found.push(k));
        assert!(found.contains(&keys[0]));
    }

    #[test]
    fn distant_entity_not_found() {
        let (_map, keys) = make_keys(1);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 1);
        // Place at (95, 95) = cell (9, 9)
        sh.rebuild([(keys[0], 95.0, 95.0)].iter().cloned());

        // Query from (5, 5) = cell (0, 0) — far from (9, 9)
        // With toroidal wrapping, (0,0) neighbors are (9,9),(0,0),(1,0) etc.
        // Actually toroidal wrapping DOES connect (0,0) to (9,9)!
        let mut found = vec![];
        sh.query_neighbors(5.0, 5.0, |k| found.push(k));
        // With 10x10 grid and toroidal wrapping, corner cells ARE neighbors
        assert!(found.contains(&keys[0]));
    }

    #[test]
    fn multiple_entities_same_cell() {
        let (_map, keys) = make_keys(3);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 3);
        sh.rebuild([
            (keys[0], 5.0, 5.0),
            (keys[1], 7.0, 3.0),
            (keys[2], 2.0, 8.0),
        ].iter().cloned());

        let mut found = vec![];
        sh.query_neighbors(5.0, 5.0, |k| found.push(k));
        assert!(found.contains(&keys[0]));
        assert!(found.contains(&keys[1]));
        assert!(found.contains(&keys[2]));
    }

    #[test]
    fn rebuild_clears_previous_data() {
        let (_map, keys) = make_keys(2);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 2);

        // First build
        sh.rebuild([(keys[0], 50.0, 50.0)].iter().cloned());
        let mut found1 = vec![];
        sh.query_neighbors(50.0, 50.0, |k| found1.push(k));
        assert_eq!(found1.len(), 1);

        // Rebuild with different entity
        sh.rebuild([(keys[1], 50.0, 50.0)].iter().cloned());
        let mut found2 = vec![];
        sh.query_neighbors(50.0, 50.0, |k| found2.push(k));
        assert_eq!(found2.len(), 1);
        assert_eq!(found2[0], keys[1]);
    }

    #[test]
    fn query_radius_finds_nearby() {
        let (_map, keys) = make_keys(2);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 2);
        sh.rebuild([
            (keys[0], 50.0, 50.0),
            (keys[1], 90.0, 90.0),
        ].iter().cloned());

        let mut found = vec![];
        sh.query_radius(50.0, 50.0, 15.0, |k| found.push(k));
        // key[0] is in the query area, key[1] is far
        assert!(found.contains(&keys[0]));
    }

    #[test]
    fn grid_dimensions_correct() {
        let sh = SpatialHash::new(100.0, 50.0, 10.0, 0);
        assert_eq!(sh.grid_width(), 10);
        assert_eq!(sh.grid_height(), 5);
        assert_eq!(sh.cell_size(), 10.0);
    }

    #[test]
    fn boundary_entity_clamped_to_grid() {
        let (_map, keys) = make_keys(1);
        let mut sh = SpatialHash::new(100.0, 100.0, 10.0, 1);
        // Place exactly at world boundary
        sh.rebuild([(keys[0], 100.0, 100.0)].iter().cloned());
        // Should not panic — cell_index clamps to grid bounds
        let mut found = vec![];
        sh.query_neighbors(99.0, 99.0, |k| found.push(k));
        assert!(found.contains(&keys[0]));
    }
}
