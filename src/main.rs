mod digishield;
mod metrics;
mod plotting;
mod simulation;
use rand::Rng;

pub struct Config {
    pub label: String,
    pub blocks: u64,           // number of blocks to simulate
    pub t: u64,                // new target solvetime (25s post-fork)
    pub old_t: u64,            // old target solvetime (75s pre-fork)
    pub n: u64,                // DAA adjustment block span (28 = 17 averaging + 11 MTP)
    pub dx: u64,               // difficulty scaling (2^13 for Zcash)
    pub baseline_d: u64,       // starting difficulty (pre-fork steady state)
    pub fork_height: u64,      // height at which DAA window becomes valid
    pub transition_block: u64, // block index (relative to fork_height) where T changes
}

fn main() {
    println!("Zcash Digishield DAA — Fork Transition Simulator (75s -> 25s)");
    println!("==============================================================\n");

    // Pre-generate random values for reproducible stochastic mining
    let mut rng = rand::thread_rng();
    let neg_log_rand: Vec<f64> = (0..20_000).map(|_| -rng.r#gen::<f64>().ln()).collect();

    let transition_block = 5000u64;
    let config = Config {
        label: "Fork Transition (75s -> 25s)".to_string(),
        blocks: 20_000,
        t: 25,
        old_t: 75,
        n: 28,
        dx: 8192,
        baseline_d: 104_070_000, // ~104M, close to Zcash mainnet difficulty
        fork_height: 200,
        transition_block,
    };

    println!("Running: {} ...", config.label);
    let res = simulation::run(&config, &neg_log_rand);

    // Compute and print metrics for post-fork steady state
    let m = metrics::compute(&config, &res);
    metrics::print_summary(&config, &m);

    // Fork transition analysis
    let tb = transition_block as usize;
    let new_t = config.t as f64;
    let old_t_val = config.old_t as f64;
    let old_d = config.baseline_d as f64;
    let new_equil_d = old_d * new_t / old_t_val;

    // Pre-fork avg solvetime (last 100 blocks before transition)
    let pre_start = if tb > 100 { tb - 100 } else { 0 };
    let pre_avg_st: f64 = res.sts[pre_start..tb].iter().map(|&s| s as f64).sum::<f64>()
        / (tb - pre_start) as f64;
    println!("  Pre-fork avg ST (last 100 blocks):  {:.1}s (target: {:.0}s)", pre_avg_st, old_t_val);

    // How many blocks until D is within 10% of new equilibrium?
    let mut d_converged_block: Option<usize> = None;
    for i in tb..res.ds.len() {
        if (res.ds[i] as f64) <= new_equil_d * 1.10 && d_converged_block.is_none() {
            d_converged_block = Some(i);
        }
    }

    if let Some(db) = d_converged_block {
        let blocks_to_converge = db - tb;
        let wallclock: u64 = res.sts[tb..db].iter().sum();
        println!("  Difficulty converged to <=1.1x D_new={:.0} at block {} ({} blocks after fork)",
            new_equil_d * 1.10, db, blocks_to_converge);
        println!("  Wall-clock time to D convergence:   {:.1}min", wallclock as f64 / 60.0);
    } else {
        println!("  Difficulty did NOT converge within simulation window!");
    }

    // Post-convergence averages
    let post_start = (tb + 200).min(res.sts.len());
    let post_end = (tb + 1000).min(res.sts.len());
    if post_end > post_start {
        let post_avg_st: f64 = res.sts[post_start..post_end].iter().map(|&s| s as f64).sum::<f64>()
            / (post_end - post_start) as f64;
        let post_avg_d: f64 = res.ds[post_start..post_end].iter().map(|&d| d as f64).sum::<f64>()
            / (post_end - post_start) as f64;
        println!("  Post-convergence avg ST (blk {}..{}): {:.1}s (target: {:.0}s)",
            post_start, post_end, post_avg_st, new_t);
        println!("  Post-convergence avg D (blk {}..{}):  {:.0} (expected: {:.0})",
            post_start, post_end, post_avg_d, new_equil_d);
    }

    // Max solvetime in first 100 blocks after fork
    let trans_end = (tb + 100).min(res.sts.len());
    let max_st_transition = res.sts[tb..trans_end].iter().max().unwrap_or(&0);
    let avg_st_transition: f64 = res.sts[tb..trans_end].iter().map(|&s| s as f64).sum::<f64>()
        / (trans_end - tb) as f64;
    println!("  First 100 blocks after fork: max ST={}s ({:.1}min), avg ST={:.1}s",
        max_st_transition, *max_st_transition as f64 / 60.0, avg_st_transition);

    // Generate chart and HTML report (from last single run)
    println!("\nGenerating report...");
    let mut charts: Vec<(String, Vec<u8>)> = Vec::new();
    let png = plotting::plot_fork_transition(&config, &res, tb);
    charts.push(("Fork Transition (75s -> 25s) : DAA Convergence".to_string(), png));

    metrics::write_html_report("report.html", &config, &m, &charts)
        .expect("failed to write HTML report");

    // === Ensemble: 100 trials for robust convergence statistics ===
    let num_trials = 100;
    println!("\nRunning {} trial ensemble...", num_trials);

    let mut convergence_blocks: Vec<u64> = Vec::new();
    let mut convergence_wallclocks: Vec<f64> = Vec::new();
    let mut max_st_first_100: Vec<u64> = Vec::new();
    let mut avg_st_first_100: Vec<f64> = Vec::new();
    let mut post_avg_st: Vec<f64> = Vec::new();
    let mut post_avg_d: Vec<f64> = Vec::new();

    for trial in 0..num_trials {
        let rand_vals: Vec<f64> = (0..20_000).map(|_| -rng.r#gen::<f64>().ln()).collect();
        let trial_res = simulation::run(&config, &rand_vals);

        // Convergence: blocks until D <= 1.1 * new_equil_d
        let mut conv_block: Option<usize> = None;
        for i in tb..trial_res.ds.len() {
            if (trial_res.ds[i] as f64) <= new_equil_d * 1.10 && conv_block.is_none() {
                conv_block = Some(i);
            }
        }
        if let Some(cb) = conv_block {
            convergence_blocks.push((cb - tb) as u64);
            let wc: u64 = trial_res.sts[tb..cb].iter().sum();
            convergence_wallclocks.push(wc as f64 / 60.0);
        }

        // First 100 blocks after fork
        let te = (tb + 100).min(trial_res.sts.len());
        max_st_first_100.push(*trial_res.sts[tb..te].iter().max().unwrap_or(&0));
        avg_st_first_100.push(
            trial_res.sts[tb..te].iter().map(|&s| s as f64).sum::<f64>() / (te - tb) as f64
        );

        // Post-convergence steady state
        let ps = (tb + 200).min(trial_res.sts.len());
        let pe = (tb + 1000).min(trial_res.sts.len());
        if pe > ps {
            post_avg_st.push(
                trial_res.sts[ps..pe].iter().map(|&s| s as f64).sum::<f64>() / (pe - ps) as f64
            );
            post_avg_d.push(
                trial_res.ds[ps..pe].iter().map(|&d| d as f64).sum::<f64>() / (pe - ps) as f64
            );
        }

        if (trial + 1) % 25 == 0 {
            println!("  ... completed {}/{} trials", trial + 1, num_trials);
        }
    }

    // Compute ensemble statistics
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let std_dev = |v: &[f64]| {
        let m = mean(v);
        (v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64).sqrt()
    };
    let percentile = |v: &mut Vec<f64>, p: f64| -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((v.len() as f64 * p / 100.0) as usize).min(v.len() - 1);
        v[idx]
    };

    let conv_f64: Vec<f64> = convergence_blocks.iter().map(|&x| x as f64).collect();
    let max_st_f64: Vec<f64> = max_st_first_100.iter().map(|&x| x as f64).collect();

    println!("\n=== Ensemble Results ({} trials) ===", num_trials);
    println!("  Convergence (blocks to D <= 1.1x target):");
    println!("    Mean: {:.1} blocks | StdDev: {:.1} blocks",
        mean(&conv_f64), std_dev(&conv_f64));
    println!("    Median: {:.0} | P5: {:.0} | P95: {:.0}",
        percentile(&mut conv_f64.clone(), 50.0),
        percentile(&mut conv_f64.clone(), 5.0),
        percentile(&mut conv_f64.clone(), 95.0));
    println!("    Wall-clock: mean {:.1}min | StdDev {:.1}min",
        mean(&convergence_wallclocks), std_dev(&convergence_wallclocks));

    println!("  First 100 blocks after fork:");
    println!("    Avg ST: mean {:.1}s | Max ST: mean {:.0}s, worst {:.0}s",
        mean(&avg_st_first_100), mean(&max_st_f64),
        percentile(&mut max_st_f64.clone(), 99.0));

    println!("  Post-convergence steady state (blk {}..{}):", tb + 200, tb + 1000);
    println!("    Avg ST: {:.1}s (target: {}s) | Avg D: {:.0} (expected: {:.0})",
        mean(&post_avg_st), config.t, mean(&post_avg_d), new_equil_d);

    println!("\nDone! Open report.html to view results.");
}
