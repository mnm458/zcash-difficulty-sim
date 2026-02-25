mod digishield;
mod metrics;
mod plotting;
mod simulation;
use rand::Rng;
use simulation::run;
use crate::simulation::SimResult;

pub struct Config {
    pub label: String,
    pub blocks: u64,           // number of blocks to simulate
    pub t: u64,                // target solvetime (75 or 25)
    pub n: u64,                // DAA adjustment block span (28 = 17 averaging + 11 MTP)
    pub dx: u64,               // difficulty scaling (2^13 for Zcash)
    pub baseline_d: u64,       // starting difficulty
    pub attack_start: u64,     // % of baseline_d where attacker turns on (on-off only)
    pub attack_stop: u64,      // % of baseline_d where attacker turns off (on-off only)
    pub attack_size: u64,      // attacker HR as % of baseline HR (on-off only)
    pub fork_height: u64,      // 0 for genesis
}

struct ScenarioResult {
    label: String,
    config_75: Config,
    config_25: Config,
    res_75: SimResult,
    res_25: SimResult,
    metrics_75: metrics::Metrics,
    metrics_25: metrics::Metrics,
}

fn make_config(label: &str, t: u64) -> Config {
    Config {
        label: label.to_string(),
        blocks: 20_000,
        t,
        n: 28,
        dx: 8192,
        baseline_d: 104_070_000, // ~104M, close to Zcash mainnet difficulty
        attack_start: 130,
        attack_stop: 135,
        attack_size: 800,
        fork_height: 200,   // start after enough blocks for the DAA window
    }
}

fn main() {
    // Pre-generate random values (shared across all runs for fair comparison)
    let mut rng = rand::thread_rng();
    let neg_log_rand: Vec<f64> = (0..20_000).map(|_| -rng.r#gen::<f64>().ln()).collect();

    let mut all_charts: Vec<(String, Vec<u8>)> = Vec::new();

    // === Fork Transition Scenario (single run, not a 75s-vs-25s comparison) ===
    let transition_block = 5000u64;
    let fork_config = Config {
        label: "Fork Transition (75s → 25s)".to_string(),
        blocks: 20_000,
        t: 25,  // new target
        n: 28,
        dx: 8192,
        baseline_d: 104_070_000,
        attack_start: 0, attack_stop: 0, attack_size: 0,
        fork_height: 200,
        attack_mode: AttackMode::ForkTransition { transition_block, old_t: 75 },
    };
    println!("Running: Fork Transition (75s → 25s) ...");
    let fork_res = run(&fork_config, &neg_log_rand);

    // Fork transition chart
    let png = plotting::plot_fork_transition(&fork_config, &fork_res, transition_block as usize);
    all_charts.push(("Fork Transition (75s -> 25s) : DAA Convergence".to_string(), png));

    // Generate HTML report
    let metrics_data: Vec<(&str, &Config, &metrics::Metrics, &Config, &metrics::Metrics)> = results.iter()
        .map(|r| (r.label.as_str(), &r.config_75, &r.metrics_75, &r.config_25, &r.metrics_25))
        .collect();
    metrics::write_html_report("report.html", &all_charts, &metrics_data)
        .expect("failed to write HTML report");
}
