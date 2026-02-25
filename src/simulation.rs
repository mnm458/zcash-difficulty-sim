use crate::Config;
use crate::AttackMode;
use crate::digishield::digishield;

/// POW_ADJUSTMENT_BLOCK_SPAN = 28 (17 averaging window + 11 MTP span).
/// The sliding window stores 29 entries (28 intervals).
const WINDOW_SIZE: usize = 29;

pub struct SimResult {
    pub sts: Vec<u64>,
    pub ds: Vec<u64>,
    pub hrs: Vec<u64>,
}


pub fn run(config: &Config, neg_log_rand: &[f64]) -> SimResult {
    let t = config.t;
    let dx = config.dx;
    let baseline_d = config.baseline_d;
    let fork_height = config.fork_height;
    let blocks = config.blocks as usize;

    // For ForkTransition, initialize at old_t steady state; otherwise use config.t
    let init_t = match config.attack_mode {
        AttackMode::ForkTransition { old_t, .. } => old_t,
        _ => t,
    };
    let baseline_hr = baseline_d * dx / init_t;

    let mut sts: Vec<u64> = Vec::with_capacity(blocks);
    let mut ds: Vec<u64> = Vec::with_capacity(blocks);
    let mut hrs: Vec<u64> = Vec::with_capacity(blocks);

    // Initialize timestamps and cumulative difficulties.
    // Fabricate 28 prior blocks at steady state so the DAA has a full window (29 entries).
    let mut ts: Vec<u64> = Vec::with_capacity(WINDOW_SIZE);
    let mut cd: Vec<u64> = Vec::with_capacity(WINDOW_SIZE);

    let start_timestamp: u64 = 1_540_000_000;
    let start_cd: u64 = 1_000_000_000;

    ts.push(start_timestamp);
    cd.push(start_cd);
    for i in 1..WINDOW_SIZE {
        ts.push(start_timestamp + (i as u64) * init_t);
        cd.push(start_cd + (i as u64) * baseline_d);
    }

    let mut hr = baseline_hr;
    let mut next_d: u64 = baseline_d;

    for i in 0..blocks {
        let height = fork_height + i as u64;

        // Determine the effective target solvetime for this block.
        // For ForkTransition, T changes at transition_block.
        let effective_t = match config.attack_mode {
            AttackMode::ForkTransition { transition_block, old_t } => {
                if (i as u64) < transition_block { old_t } else { t }
            }
            _ => t,
        };

        // Determine hashrate based on attack mode
        match config.attack_mode {
            AttackMode::None | AttackMode::ForkTransition { .. } => {
                hr = baseline_hr;
            }
            AttackMode::OnOff => {
                if next_d > (config.attack_stop * baseline_d) / 100 {
                    hr = baseline_hr;
                } else if next_d < (config.attack_start * baseline_d) / 100 {
                    hr = (baseline_hr * config.attack_size) / 100;
                }
            }
            AttackMode::Crash { crash_block, crash_pct } => {
                if (i as u64) >= crash_block {
                    hr = (baseline_hr * crash_pct) / 100;
                } else {
                    hr = baseline_hr;
                }
            }
        }

        // Run Digishield DAA with the effective T for this block.
        // Signature: digishield(timestamps, cumulative_difficulties, t, height, fork_height, difficulty_guess)
        next_d = digishield(&ts, &cd, effective_t, height, fork_height, baseline_d);

        // Append new cumulative difficulty (CD gets 1 block ahead of TS)
        cd.push(cd.last().unwrap() + next_d);
        if cd.len() > WINDOW_SIZE {
            cd.remove(0);
        }

        // Simulate solvetime: ST = -ln(rand) * D * DX / HR
        // HR and physics don't change — only the DAA's target T changes
        let simulated_st = if hr > 0 {
            (neg_log_rand[i] * (next_d * dx) as f64 / hr as f64) as u64
        } else {
            effective_t // fallback to target if HR is zero
        };
        let current_st = std::cmp::max(1, simulated_st);

        // TS catches up with CD
        ts.push(ts.last().unwrap() + current_st);
        if ts.len() > WINDOW_SIZE {
            ts.remove(0);
        }

        ds.push(next_d);
        sts.push(current_st);
        hrs.push(hr);
    }

    SimResult { sts, ds, hrs }
}