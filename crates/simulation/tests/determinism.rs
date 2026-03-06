use simulation::config::SimConfig;
use simulation::world::World;

#[test]
fn same_seed_produces_identical_state() {
    let config = SimConfig::default();

    let mut world_a = World::new(config.clone()).unwrap();
    let mut world_b = World::new(config).unwrap();

    // Run both for 1000 ticks
    for _ in 0..1000 {
        world_a.step();
        world_b.step();
    }

    // Compare state
    assert_eq!(world_a.tick, world_b.tick);
    assert_eq!(world_a.creature_count(), world_b.creature_count());
    assert_eq!(world_a.food_count(), world_b.food_count());
    assert_eq!(world_a.total_births, world_b.total_births);
    assert_eq!(world_a.total_deaths, world_b.total_deaths);
    assert_eq!(world_a.generation_max, world_b.generation_max);

    // Compare creature positions (via render buffer)
    let data_a: Vec<f32> = {
        let ptr = world_a.render_data_ptr();
        let len = world_a.render_data_len();
        unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
    };
    let data_b: Vec<f32> = {
        let ptr = world_b.render_data_ptr();
        let len = world_b.render_data_len();
        unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
    };
    assert_eq!(data_a.len(), data_b.len());
    for (i, (a, b)) in data_a.iter().zip(data_b.iter()).enumerate() {
        assert_eq!(
            a.to_bits(),
            b.to_bits(),
            "Render data differs at index {}: {} vs {}",
            i,
            a,
            b
        );
    }
}

#[test]
fn different_seeds_produce_different_state() {
    let mut config_a = SimConfig::default();
    config_a.seed = 42;
    let mut config_b = SimConfig::default();
    config_b.seed = 99;

    let mut world_a = World::new(config_a).unwrap();
    let mut world_b = World::new(config_b).unwrap();

    for _ in 0..100 {
        world_a.step();
        world_b.step();
    }

    // Check multiple signals for difference — at least one must differ
    let births_differ = world_a.total_births != world_b.total_births;
    let deaths_differ = world_a.total_deaths != world_b.total_deaths;
    let count_differ = world_a.creature_count() != world_b.creature_count();
    let food_differ = world_a.food_count() != world_b.food_count();

    // Also compare render data
    let data_a: Vec<f32> = {
        let ptr = world_a.render_data_ptr();
        let len = world_a.render_data_len();
        unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
    };
    let data_b: Vec<f32> = {
        let ptr = world_b.render_data_ptr();
        let len = world_b.render_data_len();
        unsafe { std::slice::from_raw_parts(ptr, len).to_vec() }
    };
    let render_differ = data_a.len() != data_b.len()
        || data_a
            .iter()
            .zip(data_b.iter())
            .any(|(a, b)| a.to_bits() != b.to_bits());

    assert!(
        births_differ || deaths_differ || count_differ || food_differ || render_differ,
        "Different seeds should produce different states"
    );
}
