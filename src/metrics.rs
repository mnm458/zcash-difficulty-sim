use crate::Config;
use crate::simulation::SimResult;
use base64::Engine;

pub struct Metrics {
    pub avg_st: f64,
    pub avg_d: f64,
    pub std_dev_d: f64,
    pub std_dev_st: f64,
    pub pct_delays: f64,
    pub pct_stolen: f64,
    pub attack_blocks: u64,
}

pub fn compute(config: &Config, result: &SimResult) -> Metrics {
    let n = config.n as usize;
    let t = config.t as f64;
    let start = 2 * n + 1;
    let count = result.ds.len();

    if start >= count {
        return Metrics {
            avg_st: 0.0, avg_d: 0.0, std_dev_d: 0.0,
            std_dev_st: 0.0, pct_delays: 0.0, pct_stolen: 0.0,
            attack_blocks: 0,
        };
    }
    let len = (count - start) as f64;

    // Averages
    let avg_st: f64 = result.sts[start..].iter().map(|&x| x as f64).sum::<f64>() / len;
    let avg_d: f64 = result.ds[start..].iter().map(|&x| x as f64).sum::<f64>() / len;

    // Std deviations (normalized)
    let var_d: f64 = result.ds[start..].iter()
        .map(|&x| { let diff = x as f64 - avg_d; diff * diff })
        .sum::<f64>() / len / (avg_d * avg_d);

    let var_st: f64 = result.sts[start..].iter()
        .map(|&x| { let diff = x as f64 - avg_st; diff * diff })
        .sum::<f64>() / len / (avg_st * avg_st);

    // Delays: blocks with ST > 4*T
    let delays: f64 = result.sts[start..].iter()
        .map(|&st| if st as f64 > 4.0 * t { (st as f64 / t) - 4.0 } else { 0.0 })
        .sum::<f64>();
    let pct_delays = delays * 100.0 / len;

    // Stolen: reward advantage for on-off miners vs dedicated miners
    let baseline_hr = config.baseline_d * config.dx / config.t;
    let mut dedicated_time = 0.0_f64;
    let mut dedicated_reward = 0.0_f64;
    let mut attackers_time = 0.0_f64;
    let mut attackers_reward = 0.0_f64;
    let mut attack_blocks = 0_u64;

    for i in start..count {
        let n_st = result.sts[i] as f64 / t;
        let n_d = result.ds[i] as f64 / avg_d;

        dedicated_time += n_st;
        dedicated_reward += n_st / n_d;

        if result.hrs[i] > baseline_hr + 1 {
            attackers_time += n_st;
            attackers_reward += n_st / n_d;
            attack_blocks += 1;
        }
    }

    let pct_stolen = if attackers_time > 0.0 && dedicated_time > 0.0 {
        ((attackers_reward / attackers_time) / (dedicated_reward / dedicated_time) - 1.0) * 100.0
    } else {
        0.0
    };

    Metrics {
        avg_st,
        avg_d,
        std_dev_d: var_d.sqrt(),
        std_dev_st: var_st.sqrt(),
        pct_delays,
        pct_stolen,
        attack_blocks,
    }
}

/// Print metrics summary to stdout.
pub fn print_summary(label: &str, config: &Config, metrics: &Metrics) {
    println!("=== {} ===", label);
    println!("  Target ST: {}s | Avg ST: {:.1}s | Avg D: {:.0}", config.t, metrics.avg_st, metrics.avg_d);
    println!("  StdDev D: {:.3} | StdDev ST: {:.2}", metrics.std_dev_d, metrics.std_dev_st);
    println!("  Delays: {:.2}% | Stolen: {:.2}% | Attack blocks: {}", metrics.pct_delays, metrics.pct_stolen, metrics.attack_blocks);
    println!();
}

/// Generate a single self-contained HTML report with all charts and metrics.
pub fn write_html_report(
    path: &str,
    charts: &[(String, Vec<u8>)],
    metrics_data: &[(&str, &Config, &Metrics, &Config, &Metrics)],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;

    writeln!(f, r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Zcash Digishield DAA Simulation Report</title>
<style>
  body {{ font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; margin: 20px; background: #f5f5f5; }}
  h1 {{ color: #1a1a2e; border-bottom: 3px solid #e94560; padding-bottom: 10px; }}
  h2 {{ color: #16213e; margin-top: 40px; }}
  h3 {{ color: #0f3460; }}
  .chart {{ background: white; padding: 10px; margin: 15px 0; box-shadow: 0 2px 8px rgba(0,0,0,0.1); border-radius: 4px; text-align: center; }}
  .chart img {{ max-width: 100%; height: auto; }}
  table {{ border-collapse: collapse; width: 100%; margin: 15px 0; background: white; box-shadow: 0 2px 8px rgba(0,0,0,0.1); }}
  th, td {{ border: 1px solid #ddd; padding: 10px 14px; text-align: right; }}
  th {{ background: #16213e; color: white; }}
  tr:nth-child(even) {{ background: #f9f9f9; }}
  td:first-child {{ text-align: left; font-weight: bold; }}
  .params {{ background: #e8e8e8; padding: 10px; border-radius: 4px; margin: 10px 0; font-size: 0.9em; }}
</style>
</head>
<body>
<h1>Zcash Digishield DAA Simulation Report</h1>
<div class="params">
  <strong>Parameters:</strong> PoWAveragingWindow=17, PoWMedianBlockSpan=11, DX=2^13 (8192), Baseline D=104,070,000 (~mainnet), Blocks=20000, Fork Height=200<br>
  <strong>Scenarios:</strong> No Attack | On-Off Mining (8x HR, start 130%/stop 135%) | Hashrate Crash (75% drop at block 5000)
</div>
"#)?;

    // Metrics table
    writeln!(f, "<h2>Summary Metrics</h2>")?;
    writeln!(f, "<table>")?;
    writeln!(f, "<tr><th>Scenario</th><th>Target</th><th>Avg ST</th><th>Avg D</th><th>StdDev D</th><th>StdDev ST</th><th>Delays %</th><th>Stolen %</th><th>Attack Blocks</th></tr>")?;

    for &(label, c75, m75, c25, m25) in metrics_data {
        writeln!(f, "<tr><td>{}</td><td>{}s</td><td>{:.1}s</td><td>{:.0}</td><td>{:.3}</td><td>{:.2}</td><td>{:.2}%</td><td>{:.2}%</td><td>{}</td></tr>",
                 label, c75.t, m75.avg_st, m75.avg_d, m75.std_dev_d, m75.std_dev_st, m75.pct_delays, m75.pct_stolen, m75.attack_blocks)?;
        writeln!(f, "<tr><td>{}</td><td>{}s</td><td>{:.1}s</td><td>{:.0}</td><td>{:.3}</td><td>{:.2}</td><td>{:.2}%</td><td>{:.2}%</td><td>{}</td></tr>",
                 label, c25.t, m25.avg_st, m25.avg_d, m25.std_dev_d, m25.std_dev_st, m25.pct_delays, m25.pct_stolen, m25.attack_blocks)?;
    }
    writeln!(f, "</table>")?;

    // Charts
    writeln!(f, "<h2>Charts</h2>")?;
    let engine = base64::engine::general_purpose::STANDARD;
    for (title, png_data) in charts {
        let b64 = engine.encode(png_data);
        writeln!(f, "<div class=\"chart\">")?;
        writeln!(f, "<h3>{}</h3>", title)?;
        writeln!(f, "<img src=\"data:image/bmp;base64,{}\">", b64)?;
        writeln!(f, "</div>")?;
    }

    writeln!(f, "</body></html>")?;
    Ok(())
}
