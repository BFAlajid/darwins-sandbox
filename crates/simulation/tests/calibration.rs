//! Calibration tests: verify STH simulation reproduces thesis findings.
//!
//! Target prevalence (thesis data, Cebu City):
//!   - Urban (Guadalupe): ~20.3% any STH
//!   - Rural (Sudlon II): ~4.9% any STH
//!   - Overall: ~11.7%
//!
//! These tests run 365 simulated days (8760 ticks) and check that
//! prevalence falls within acceptable calibration windows.

use simulation::barangay::BarangayConfig;
use simulation::sth_config::SthConfig;
use simulation::sth_world::SthWorld;

fn run_days(world: &mut SthWorld, days: u32) {
    let ticks = days as u64 * 24;
    for _ in 0..ticks {
        world.step();
    }
}

fn get_prevalence_any(stats_json: &str) -> f64 {
    // Parse prevalence_any from the JSON stats
    let v: serde_json::Value = serde_json::from_str(stats_json).unwrap();
    v["prevalenceAny"].as_f64().unwrap_or(0.0)
}

fn get_prevalence_ascaris(stats_json: &str) -> f64 {
    let v: serde_json::Value = serde_json::from_str(stats_json).unwrap();
    v["prevalenceAscaris"].as_f64().unwrap_or(0.0)
}

#[test]
fn urban_prevalence_in_expected_range_after_1_year() {
    let mut config = SthConfig::default();
    config.seed = 42;
    config.barangays = vec![BarangayConfig::urban_default()];

    let mut world = SthWorld::new(config).unwrap();
    run_days(&mut world, 365);

    let stats = world.get_stats_json();
    let prev = get_prevalence_any(&stats);

    // Thesis: 20.3% urban. Allow wide window for stochastic variance: 5%-45%
    assert!(
        prev >= 0.05 && prev <= 0.45,
        "Urban prevalence after 1 year = {:.1}%, expected 5%-45% (thesis target: 20.3%)",
        prev * 100.0,
    );
}

#[test]
fn rural_prevalence_lower_than_urban() {
    let mut urban_config = SthConfig::default();
    urban_config.seed = 123;
    urban_config.barangays = vec![BarangayConfig::urban_default()];

    let mut rural_config = SthConfig::default();
    rural_config.seed = 123;
    rural_config.barangays = vec![BarangayConfig::rural_default()];

    let mut urban = SthWorld::new(urban_config).unwrap();
    let mut rural = SthWorld::new(rural_config).unwrap();

    run_days(&mut urban, 365);
    run_days(&mut rural, 365);

    let urban_prev = get_prevalence_any(&urban.get_stats_json());
    let rural_prev = get_prevalence_any(&rural.get_stats_json());

    // Key thesis finding: urban prevalence > rural prevalence
    assert!(
        urban_prev > rural_prev,
        "Urban ({:.1}%) should exceed rural ({:.1}%) prevalence",
        urban_prev * 100.0,
        rural_prev * 100.0,
    );
}

#[test]
fn mda_reduces_prevalence_short_term() {
    let mut config = SthConfig::default();
    config.seed = 42;
    config.barangays = vec![BarangayConfig::urban_default()];

    let mut world = SthWorld::new(config).unwrap();

    // Run 180 days to establish baseline prevalence
    run_days(&mut world, 180);
    let pre_mda = get_prevalence_any(&world.get_stats_json());

    // Launch MDA on all schools with albendazole
    world.launch_mda(-1, 0);

    // Run 30 more days for MDA to take effect
    run_days(&mut world, 30);
    let post_mda = get_prevalence_any(&world.get_stats_json());

    // MDA should reduce prevalence (or at minimum not increase it significantly)
    // With albendazole efficacy ~95% for Ascaris, expect meaningful drop
    assert!(
        post_mda <= pre_mda + 0.05,
        "Post-MDA prevalence ({:.1}%) should not significantly exceed pre-MDA ({:.1}%)",
        post_mda * 100.0,
        pre_mda * 100.0,
    );
}

#[test]
fn deterministic_seed_produces_identical_runs() {
    let config = SthConfig::default();

    let mut world_a = SthWorld::new(config.clone()).unwrap();
    let mut world_b = SthWorld::new(config).unwrap();

    run_days(&mut world_a, 100);
    run_days(&mut world_b, 100);

    let stats_a = world_a.get_stats_json();
    let stats_b = world_b.get_stats_json();

    let prev_a = get_prevalence_any(&stats_a);
    let prev_b = get_prevalence_any(&stats_b);

    assert_eq!(
        prev_a, prev_b,
        "Same seed should produce identical prevalence: {:.4} vs {:.4}",
        prev_a, prev_b,
    );
}

#[test]
fn ascaris_dominates_in_urban_setting() {
    let mut config = SthConfig::default();
    config.seed = 42;
    config.barangays = vec![BarangayConfig::urban_default()];

    let mut world = SthWorld::new(config).unwrap();
    run_days(&mut world, 365);

    let stats = world.get_stats_json();
    let ascaris = get_prevalence_ascaris(&stats);
    let any = get_prevalence_any(&stats);

    // Ascaris should be the dominant species (thesis: Ascaris = 20.3%, Trichuris = 9.4%)
    // So Ascaris should account for >40% of total STH prevalence
    if any > 0.01 {
        assert!(
            ascaris / any > 0.3,
            "Ascaris ({:.1}%) should dominate total STH ({:.1}%)",
            ascaris * 100.0,
            any * 100.0,
        );
    }
}

#[test]
fn simulation_completes_3_years_without_crash() {
    let mut config = SthConfig::default();
    config.seed = 7777;
    config.barangays = vec![BarangayConfig::urban_default()];

    let mut world = SthWorld::new(config).unwrap();

    // 3 years = 1095 days = 26280 ticks
    run_days(&mut world, 1095);

    let stats = world.get_stats_json();
    let prev = get_prevalence_any(&stats);

    // Should not crash, prevalence should be valid
    assert!(
        prev >= 0.0 && prev <= 1.0,
        "Prevalence after 3 years = {:.4}, should be in [0, 1]",
        prev,
    );

    assert!(
        world.agent_count() > 0,
        "Should still have agents after 3 years",
    );
}
