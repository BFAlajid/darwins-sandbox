use crate::config::SimConfig;
use crate::creature::Creature;

/// Apply random movement (M1: no brain, creatures move randomly).
/// In later milestones, thrust/turn come from neural network output.
#[inline(always)]
pub fn apply_random_movement(creature: &mut Creature, thrust: f32, turn: f32, config: &SimConfig) {
    // Turn
    let clamped_turn = turn.clamp(-config.max_turn_rate, config.max_turn_rate);
    creature.rotation += clamped_turn;
    // Keep rotation in [0, TAU)
    creature.rotation = creature.rotation.rem_euclid(std::f32::consts::TAU);

    // Acceleration from thrust
    let max_thrust = creature.max_thrust();
    let clamped_thrust = thrust.clamp(0.0, 1.0) * max_thrust;
    let ax = clamped_thrust * creature.rotation.cos() / creature.mass();
    let ay = clamped_thrust * creature.rotation.sin() / creature.mass();

    // Integrate velocity
    creature.vx += ax;
    creature.vy += ay;

    // Apply drag
    let drag = 1.0 - config.drag_coefficient;
    creature.vx *= drag;
    creature.vy *= drag;

    // Integrate position
    creature.x += creature.vx;
    creature.y += creature.vy;

    // Toroidal wrapping
    creature.x = creature.x.rem_euclid(config.world_width);
    creature.y = creature.y.rem_euclid(config.world_height);
}

/// Calculate squared speed of a creature (avoids sqrt)
#[inline(always)]
pub fn speed_squared(creature: &Creature) -> f32 {
    creature.vx * creature.vx + creature.vy * creature.vy
}

/// Squared distance between two points with toroidal wrapping
#[inline(always)]
pub fn distance_squared_toroidal(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    world_width: f32,
    world_height: f32,
) -> f32 {
    let mut dx = (x1 - x2).abs();
    let mut dy = (y1 - y2).abs();
    if dx > world_width * 0.5 {
        dx = world_width - dx;
    }
    if dy > world_height * 0.5 {
        dy = world_height - dy;
    }
    dx * dx + dy * dy
}
