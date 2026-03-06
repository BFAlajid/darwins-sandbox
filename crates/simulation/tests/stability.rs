use simulation::config::SimConfig;
use simulation::world::World;

#[test]
fn population_stabilizes_at_carrying_capacity() {
    let config = SimConfig::default();
    let mut world = World::new(config).unwrap();

    // Run for 5000 ticks
    for _ in 0..5000 {
        world.step();
    }

    let count = world.creature_count();
    assert!(
        count > 10,
        "Population collapsed to {} after 5000 ticks",
        count
    );
    assert!(
        count < 1000,
        "Population exploded to {} after 5000 ticks",
        count
    );
}

#[test]
fn no_nan_deaths_in_normal_operation() {
    let config = SimConfig::default();
    let mut world = World::new(config).unwrap();

    for _ in 0..5000 {
        world.step();
    }

    assert_eq!(
        world.nan_deaths, 0,
        "Got {} NaN deaths in normal operation",
        world.nan_deaths
    );
}

#[test]
fn energy_budget_stays_stable() {
    let config = SimConfig::default();
    let mut world = World::new(config).unwrap();

    for _ in 0..5000 {
        world.step();
    }

    let drift = world.energy_drift_pct();
    assert!(
        drift < 20.0,
        "Energy drift too high: {:.1}%",
        drift
    );
}

#[test]
fn population_never_hits_zero_with_default_config() {
    let config = SimConfig::default();
    let mut world = World::new(config).unwrap();

    for tick in 0..3000 {
        world.step();
        // Allow brief population dips in early ticks due to initial starvation
        if tick > 200 {
            assert!(
                world.creature_count() > 0,
                "Population hit zero at tick {}",
                tick
            );
        }
    }
}

#[test]
fn reproduction_produces_new_generations() {
    let config = SimConfig::default();
    let mut world = World::new(config).unwrap();

    for _ in 0..2000 {
        world.step();
    }

    assert!(
        world.generation_max > 0,
        "No reproduction occurred in 2000 ticks"
    );
    assert!(
        world.total_births > 0,
        "No births recorded in 2000 ticks"
    );
}
