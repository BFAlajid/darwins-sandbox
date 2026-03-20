/// Environmental contamination grid for the STH simulation.
///
/// Models soil and water contamination across a spatial grid, with terrain-dependent
/// decay rates and facility placement. Each cell represents a ~10m x 10m area.

use serde::{Deserialize, Serialize};

// ---------- Terrain ----------

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainType {
    Residential = 0,
    School = 1,
    HealthCenter = 2,
    Creek = 3,       // high contamination risk
    Market = 4,
    Road = 5,
    OpenField = 6,   // open defecation risk
}

// ---------- Facility ----------

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FacilityType {
    Latrine = 0,
    WaterPump = 1,
    HandwashStation = 2,
    School = 3,
    HealthCenter = 4,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Facility {
    pub x: f32,
    pub y: f32,
    pub facility_type: FacilityType,
}

// ---------- Contamination Cell ----------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContaminationCell {
    pub soil_contamination: f32,  // 0.0-1.0, from open defecation
    pub water_contamination: f32, // 0.0-1.0, from runoff
    pub has_latrine: bool,
    pub has_water_source: bool,
    pub terrain: TerrainType,
}

impl ContaminationCell {
    pub fn new(terrain: TerrainType) -> Self {
        Self {
            soil_contamination: 0.0,
            water_contamination: 0.0,
            has_latrine: false,
            has_water_source: false,
            terrain,
        }
    }
}

// ---------- Environment Grid ----------

pub struct EnvironmentGrid {
    cells: Vec<ContaminationCell>,
    grid_width: usize,
    grid_height: usize,
    cell_size: f32,
    inv_cell_size: f32,
    pub facilities: Vec<Facility>,
}

impl EnvironmentGrid {
    /// Create a new grid covering `width x height` world units with the given cell size.
    /// All cells start as `Residential` with zero contamination.
    pub fn new(width: f32, height: f32, cell_size: f32) -> Self {
        assert!(cell_size > 0.0, "cell_size must be positive");
        assert!(width > 0.0 && height > 0.0, "dimensions must be positive");

        let grid_width = (width / cell_size).ceil() as usize;
        let grid_height = (height / cell_size).ceil() as usize;
        let total = grid_width * grid_height;

        Self {
            cells: vec![ContaminationCell::new(TerrainType::Residential); total],
            grid_width,
            grid_height,
            cell_size,
            inv_cell_size: 1.0 / cell_size,
            facilities: Vec::new(),
        }
    }

    // ---- coordinate helpers ----

    /// Convert world coordinates to cell indices. Returns signed values so callers
    /// can detect out-of-bounds without underflow.
    pub fn world_to_cell(&self, x: f32, y: f32) -> (i32, i32) {
        let cx = (x * self.inv_cell_size).floor() as i32;
        let cy = (y * self.inv_cell_size).floor() as i32;
        (cx, cy)
    }

    fn cell_in_bounds(&self, cx: i32, cy: i32) -> bool {
        cx >= 0 && cy >= 0 && (cx as usize) < self.grid_width && (cy as usize) < self.grid_height
    }

    fn cell_index(&self, cx: i32, cy: i32) -> Option<usize> {
        if self.cell_in_bounds(cx, cy) {
            Some(cy as usize * self.grid_width + cx as usize)
        } else {
            None
        }
    }

    /// Get an immutable reference to a cell (bounds-checked).
    pub fn get_cell(&self, cx: i32, cy: i32) -> Option<&ContaminationCell> {
        self.cell_index(cx, cy).map(|i| &self.cells[i])
    }

    /// Get a mutable reference to a cell (bounds-checked).
    pub fn get_cell_mut(&mut self, cx: i32, cy: i32) -> Option<&mut ContaminationCell> {
        self.cell_index(cx, cy).map(|i| &mut self.cells[i])
    }

    // ---- contamination dynamics ----

    /// Natural contamination decay per tick (1 hour).
    /// Decay rates depend on terrain type:
    /// - Creek: 0.002 (wet = slower decay)
    /// - OpenField: 0.008 (UV exposure speeds decay)
    /// - Other: 0.005
    pub fn update_contamination(&mut self) {
        for cell in &mut self.cells {
            let decay_rate = match cell.terrain {
                TerrainType::Creek => 0.002,      // wet = slower decay
                TerrainType::OpenField => 0.008,   // UV exposure speeds decay
                _ => 0.005,                        // residential/other
            };
            cell.soil_contamination *= 1.0 - decay_rate;

            // Water contamination decays faster with improved water source
            if cell.has_water_source {
                cell.water_contamination *= 0.999;
            } else {
                cell.water_contamination *= 0.9999;
            }
        }
    }

    /// Deposit contamination from open defecation at a world position.
    /// Spreads 30% of the amount to the 4 cardinal neighbors.
    pub fn deposit_contamination(&mut self, x: f32, y: f32, amount: f32) {
        let (cx, cy) = self.world_to_cell(x, y);
        if let Some(cell) = self.get_cell_mut(cx, cy) {
            cell.soil_contamination = (cell.soil_contamination + amount).min(1.0);
        }
        // Spread to cardinal neighbors
        for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
            if let Some(cell) = self.get_cell_mut(cx + dx, cy + dy) {
                cell.soil_contamination =
                    (cell.soil_contamination + amount * 0.3).min(1.0);
            }
        }
    }

    /// Read soil and water contamination at a world position.
    pub fn sample_contamination(&self, x: f32, y: f32) -> (f32, f32) {
        let (cx, cy) = self.world_to_cell(x, y);
        self.get_cell(cx, cy)
            .map(|c| (c.soil_contamination, c.water_contamination))
            .unwrap_or((0.0, 0.0))
    }

    /// Rain event spreads contamination toward creek and low-lying areas.
    /// `factor` controls the intensity of the rain (0.0-1.0).
    pub fn spread_contamination_rain(&mut self, factor: f32) {
        let factor = factor.clamp(0.0, 1.0);
        if factor <= 0.0 {
            return;
        }

        // Snapshot current soil contamination to avoid feedback loops
        let soil_snapshot: Vec<f32> = self.cells.iter().map(|c| c.soil_contamination).collect();

        let gw = self.grid_width;
        let gh = self.grid_height;

        for gy in 0..gh {
            for gx in 0..gw {
                let idx = gy * gw + gx;

                // Creek cells receive runoff from all neighbors
                if self.cells[idx].terrain == TerrainType::Creek {
                    let mut runoff = 0.0_f32;
                    for (dx, dy) in [(-1i32, 0), (1, 0), (0, -1), (0, 1)] {
                        let nx = gx as i32 + dx;
                        let ny = gy as i32 + dy;
                        if nx >= 0 && ny >= 0 && (nx as usize) < gw && (ny as usize) < gh {
                            let ni = ny as usize * gw + nx as usize;
                            runoff += soil_snapshot[ni] * factor * 0.1;
                        }
                    }
                    self.cells[idx].water_contamination =
                        (self.cells[idx].water_contamination + runoff).min(1.0);
                }

                // Rain washes some soil contamination into water
                let wash = soil_snapshot[idx] * factor * 0.05;
                self.cells[idx].water_contamination =
                    (self.cells[idx].water_contamination + wash).min(1.0);
            }
        }
    }

    // ---- facilities ----

    /// Place a facility at a world position.
    /// Also marks the underlying cell appropriately (latrine / water source).
    pub fn add_facility(&mut self, x: f32, y: f32, facility_type: FacilityType) {
        self.facilities.push(Facility {
            x,
            y,
            facility_type,
        });

        let (cx, cy) = self.world_to_cell(x, y);
        if let Some(cell) = self.get_cell_mut(cx, cy) {
            match facility_type {
                FacilityType::Latrine => cell.has_latrine = true,
                FacilityType::WaterPump | FacilityType::HandwashStation => {
                    cell.has_water_source = true;
                }
                FacilityType::School => cell.terrain = TerrainType::School,
                FacilityType::HealthCenter => cell.terrain = TerrainType::HealthCenter,
            }
        }
    }

    // ---- rendering data ----

    /// Flat soil contamination array for rendering overlay.
    /// One f32 per cell, row-major order.
    pub fn contamination_grid_flat(&self) -> Vec<f32> {
        self.cells.iter().map(|c| c.soil_contamination).collect()
    }

    /// Flat facility array for rendering: [x, y, type_u8, 1.0] per facility.
    pub fn facility_positions_flat(&self) -> Vec<f32> {
        let mut out = Vec::with_capacity(self.facilities.len() * 4);
        for f in &self.facilities {
            out.push(f.x);
            out.push(f.y);
            out.push(f.facility_type as u8 as f32);
            out.push(1.0);
        }
        out
    }

    // ---- accessors ----

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

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_grid_dimensions() {
        let grid = EnvironmentGrid::new(100.0, 50.0, 10.0);
        assert_eq!(grid.grid_width(), 10);
        assert_eq!(grid.grid_height(), 5);
        assert_eq!(grid.cells.len(), 50);
    }

    #[test]
    fn world_to_cell_basic() {
        let grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        assert_eq!(grid.world_to_cell(0.0, 0.0), (0, 0));
        assert_eq!(grid.world_to_cell(15.0, 25.0), (1, 2));
        assert_eq!(grid.world_to_cell(99.0, 99.0), (9, 9));
    }

    #[test]
    fn out_of_bounds_returns_none() {
        let grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        assert!(grid.get_cell(-1, 0).is_none());
        assert!(grid.get_cell(0, -1).is_none());
        assert!(grid.get_cell(10, 0).is_none());
        assert!(grid.get_cell(0, 10).is_none());
    }

    #[test]
    fn deposit_and_sample_contamination() {
        let mut grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid.deposit_contamination(15.0, 15.0, 0.5);

        // Center cell should have 0.5
        let (soil, _water) = grid.sample_contamination(15.0, 15.0);
        assert!((soil - 0.5).abs() < 1e-6);

        // Neighbor should have 0.5 * 0.3 = 0.15
        let (neighbor_soil, _) = grid.sample_contamination(25.0, 15.0);
        assert!((neighbor_soil - 0.15).abs() < 1e-6);
    }

    #[test]
    fn contamination_clamped_to_one() {
        let mut grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid.deposit_contamination(15.0, 15.0, 0.8);
        grid.deposit_contamination(15.0, 15.0, 0.8);
        let (soil, _) = grid.sample_contamination(15.0, 15.0);
        assert!((soil - 1.0).abs() < 1e-6);
    }

    #[test]
    fn update_contamination_decays() {
        let mut grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid.deposit_contamination(15.0, 15.0, 0.5);
        let before = grid.sample_contamination(15.0, 15.0).0;

        grid.update_contamination();
        let after = grid.sample_contamination(15.0, 15.0).0;
        assert!(after < before, "contamination should decay");
    }

    #[test]
    fn terrain_affects_decay_rate() {
        // Creek decays slower than OpenField
        let mut grid_creek = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid_creek.get_cell_mut(0, 0).unwrap().terrain = TerrainType::Creek;
        grid_creek.get_cell_mut(0, 0).unwrap().soil_contamination = 0.5;

        let mut grid_field = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid_field.get_cell_mut(0, 0).unwrap().terrain = TerrainType::OpenField;
        grid_field.get_cell_mut(0, 0).unwrap().soil_contamination = 0.5;

        grid_creek.update_contamination();
        grid_field.update_contamination();

        let creek_val = grid_creek.get_cell(0, 0).unwrap().soil_contamination;
        let field_val = grid_field.get_cell(0, 0).unwrap().soil_contamination;
        assert!(
            creek_val > field_val,
            "Creek should retain more contamination than open field"
        );
    }

    #[test]
    fn add_facility_marks_cell() {
        let mut grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid.add_facility(15.0, 15.0, FacilityType::Latrine);
        assert!(grid.get_cell(1, 1).unwrap().has_latrine);

        grid.add_facility(25.0, 25.0, FacilityType::WaterPump);
        assert!(grid.get_cell(2, 2).unwrap().has_water_source);

        grid.add_facility(35.0, 35.0, FacilityType::School);
        assert_eq!(grid.get_cell(3, 3).unwrap().terrain, TerrainType::School);
    }

    #[test]
    fn facility_positions_flat_format() {
        let mut grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        grid.add_facility(10.0, 20.0, FacilityType::Latrine);
        let flat = grid.facility_positions_flat();
        assert_eq!(flat.len(), 4);
        assert!((flat[0] - 10.0).abs() < 1e-6);
        assert!((flat[1] - 20.0).abs() < 1e-6);
        assert!((flat[2] - 0.0).abs() < 1e-6); // Latrine = 0
        assert!((flat[3] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn contamination_grid_flat_length() {
        let grid = EnvironmentGrid::new(100.0, 100.0, 10.0);
        let flat = grid.contamination_grid_flat();
        assert_eq!(flat.len(), 100); // 10x10
    }

    #[test]
    fn rain_spreads_to_creek() {
        let mut grid = EnvironmentGrid::new(30.0, 10.0, 10.0);
        // Cell 0 = contaminated residential, Cell 1 = creek
        grid.get_cell_mut(0, 0).unwrap().soil_contamination = 0.8;
        grid.get_cell_mut(1, 0).unwrap().terrain = TerrainType::Creek;

        let creek_water_before = grid.get_cell(1, 0).unwrap().water_contamination;
        grid.spread_contamination_rain(1.0);
        let creek_water_after = grid.get_cell(1, 0).unwrap().water_contamination;

        assert!(
            creek_water_after > creek_water_before,
            "Rain should spread contamination to creek water"
        );
    }

    #[test]
    #[should_panic(expected = "cell_size must be positive")]
    fn zero_cell_size_panics() {
        EnvironmentGrid::new(100.0, 100.0, 0.0);
    }
}
