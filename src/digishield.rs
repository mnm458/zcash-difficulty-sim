
const POW_AVERAGING_WINDOW: usize = 17;
const POW_MEDIAN_BLOCK_SPAN: usize =  11;

const POW_ADJUSTMENT_BLOCK_SPAN: usize = POW_AVERAGING_WINDOW + POW_MEDIAN_BLOCK_SPAN;

const  POW_DAMPING_FACTOR: i64 = 4;

const POW_MAX_ADJUST_UP_PERCENT: i64 = 16;

const POW_MAX_ADJUST_DOWN_PERCENT: i64 = 32;

fn median_time(times: &[u64]) -> u64 {
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

// This function will compute the arithmetic mean of per-block difficulties (equivalent to MeanTarget)
// In Zebra, MeanTarget is the arithmetic mean of targets (which is the inverse of difficulty).
// In this simulator I instead compute the harmonic mean of difficulties, which is mathematically equivalent.
fn mean_difficulty(cumulative_difficulties: &[u64]) -> u64 {
    let start = POW_MEDIAN_BLOCK_SPAN + 1;
    let scale:u64 =  10_000_000_000_000;

    let mut inv_sum:u64 = 0;
    for i in start..cumulative_difficulties.len() {
        let d_i = cumulative_difficulties[i] - cumulative_difficulties[i-1];
        inv_sum += scale / d_i;
    }
    POW_AVERAGING_WINDOW as u64 * scale / inv_sum
}


pub fn digishield(
    timestamps: &[u64],
    cumulative_difficulties: &[u64],
    t: u64,
    height: u64,
    fork_height: u64,
    difficulty_guess: u64,
) -> u64 {
    let required_size = POW_ADJUSTMENT_BLOCK_SPAN + 1;
    assert_eq!(timestamps.len(), cumulative_difficulties.len());
    // Hard-code difficulty for the first N+1 blocks after fork (or genesis).
    if height >= fork_height && height <= fork_height + POW_ADJUSTMENT_BLOCK_SPAN as u64 + 1 {
        return difficulty_guess;
    }
    assert!(
        timestamps.len() == required_size,
        "need {} timestamps, got {}",
        required_size,
        timestamps.len()
    );
    let mean_d = mean_difficulty(cumulative_difficulties);
    let newer_median = median_time(&timestamps[POW_AVERAGING_WINDOW + 1..]);
    let older_median = median_time(&timestamps[1..POW_MEDIAN_BLOCK_SPAN + 1]);
    let actual_timespan = newer_median as i64 - older_median as i64;
    let averaging_window_timespan = POW_AVERAGING_WINDOW as i64 * t as i64;
    let damped_variance = (actual_timespan - averaging_window_timespan) / POW_DAMPING_FACTOR;
    let actual_timespan_damped = averaging_window_timespan + damped_variance;
    let min_actual_timespan = averaging_window_timespan * (100 - POW_MAX_ADJUST_UP_PERCENT) / 100;
    let max_actual_timespan = averaging_window_timespan * (100 + POW_MAX_ADJUST_DOWN_PERCENT) / 100;
    let actual_timespan_bounded = std::cmp::max(
        min_actual_timespan,
        std::cmp::min(max_actual_timespan, actual_timespan_damped),
    );
    let new_d = mean_d as i64 * averaging_window_timespan / actual_timespan_bounded;

    std::cmp::max(1, new_d as u64)
}