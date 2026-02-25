use crate::Config;
use crate::digishield::digishield;

/// POW_ADJUSTMENT_BLOCK_SPAN = 28 (17 averaging window + 11 MTP span).
/// The sliding window stores 29 entries (28 intervals).
const WINDOW_SIZE: usize = 29;

pub struct SimResult {
    pub sts: Vec<u64>, //solvetime (per-block timestamp differences)
    pub ds: Vec<u64>, //difficulties
}

pub fn run(config: &Config, neg_log_rand: &[f64]) -> SimResult {
    let t = config.t;
    let dx = config.dx;
    let baseline_d = config.baseline_d;
    let fork_height = config.fork_height;
    let blocks = config.blocks as usize;
    let transition_block = config.transition_block;
    let old_t = config.old_t;

    // Initialize at old_t steady state
    let baseline_hr = baseline_d * dx / old_t;

    let mut sts: Vec<u64> = Vec::with_capacity(blocks);
    let mut ds: Vec<u64> = Vec::with_capacity(blocks);

    // Fabricate 28 prior blocks at steady state so the DAA has a full window (29 entries).
    let mut ts: Vec<u64> = Vec::with_capacity(WINDOW_SIZE);
    let mut cd: Vec<u64> = Vec::with_capacity(WINDOW_SIZE);

    let start_timestamp: u64 = 1772055701;
    let start_cd: u64 = 1_000_000_000;

    ts.push(start_timestamp);
    cd.push(start_cd);
    for i in 1..WINDOW_SIZE {
        ts.push(start_timestamp + (i as u64) * old_t);
        cd.push(start_cd + (i as u64) * baseline_d);
    }

    let mut next_d: u64;

    for i in 0..blocks {
        let height = fork_height + i as u64;

        let effective_t = if (i as u64) < transition_block { old_t } else { t };

        next_d = digishield(&ts, &cd, effective_t, height, fork_height, baseline_d);

        cd.push(cd.last().unwrap() + next_d);
        if cd.len() > WINDOW_SIZE {
            cd.remove(0); //pop to maintain window size
        }

        // Simulate solvetime: ST = -ln(rand) * D * DX / HR
        let simulated_st = (neg_log_rand[i] * (next_d * dx) as f64 / baseline_hr as f64) as u64;
        let current_st = std::cmp::max(1, simulated_st);

        ts.push(ts.last().unwrap() + current_st);
        if ts.len() > WINDOW_SIZE {
            ts.remove(0);
        }

        ds.push(next_d);
        sts.push(current_st);
    }

    SimResult { sts, ds }
}
