use crate::creature::CreatureKey;

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
    entries: Vec<CreatureKey>,
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
        I: Iterator<Item = (CreatureKey, f32, f32)> + Clone,
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
        self.entries.resize(total as usize, CreatureKey::default());
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
    /// Calls `callback` with each found CreatureKey.
    #[inline]
    pub fn query_neighbors(&self, x: f32, y: f32, mut callback: impl FnMut(CreatureKey)) {
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
        mut callback: impl FnMut(CreatureKey),
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
