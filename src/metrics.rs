use crate::Config;
use crate::simulation::SimResult;
use base64::Engine;

pub struct Metrics {
    pub avg_st: f64,
    pub avg_d: f64,
    pub std_dev_d: f64,
    pub std_dev_st: f64,
}

pub fn compute(config: &Config, result: &SimResult) -> Metrics {
    // Compute metrics over post-fork steady state (skip warmup + transition)
    let start = (config.transition_block as usize + 200).min(result.ds.len());
    let count = result.ds.len();

    if start >= count {
        return Metrics {
            avg_st: 0.0, avg_d: 0.0, std_dev_d: 0.0, std_dev_st: 0.0,
        };
    }
    let len = (count - start) as f64;

    let avg_st: f64 = result.sts[start..].iter().map(|&x| x as f64).sum::<f64>() / len;
    let avg_d: f64 = result.ds[start..].iter().map(|&x| x as f64).sum::<f64>() / len;

    let var_d: f64 = result.ds[start..].iter()
        .map(|&x| { let diff = x as f64 - avg_d; diff * diff })
        .sum::<f64>() / len / (avg_d * avg_d);

    let var_st: f64 = result.sts[start..].iter()
        .map(|&x| { let diff = x as f64 - avg_st; diff * diff })
        .sum::<f64>() / len / (avg_st * avg_st);

    Metrics {
        avg_st,
        avg_d,
        std_dev_d: var_d.sqrt(),
        std_dev_st: var_st.sqrt(),
    }
}

/// Print metrics summary to stdout.
pub fn print_summary(config: &Config, metrics: &Metrics) {
    println!("=== Post-Fork Steady State ===");
    println!("  Target ST: {}s | Avg ST: {:.1}s | Avg D: {:.0}", config.t, metrics.avg_st, metrics.avg_d);
    println!("  StdDev D (normalized): {:.3} | StdDev ST (normalized): {:.2}", metrics.std_dev_d, metrics.std_dev_st);
    println!();
}

/// Generate a single self-contained HTML report with the fork transition chart and metrics.
pub fn write_html_report(
    path: &str,
    config: &Config,
    metrics: &Metrics,
    charts: &[(String, Vec<u8>)],
) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(path)?;

    let new_equil_d = config.baseline_d as f64 * config.t as f64 / config.old_t as f64;

    writeln!(f, r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>Zcash DAA Fork Transition Simulation</title>
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
<h1>Zcash DAA Fork Transition Simulation (75s -> 25s)</h1>
<div class="params">
  <strong>DAA Parameters:</strong> PoWAveragingWindow=17, PoWMedianBlockSpan=11, DampingFactor=4, Clamping=[84%, 132%]<br>
  <strong>Simulation:</strong> DX=2^13 (8192), Baseline D={:.0} (~mainnet), Blocks={}, Fork Height={}, Transition Block={}<br>
  <strong>Transition:</strong> PoWTargetSpacing changes from {}s to {}s | Expected new D={:.0}
</div>
"#, config.baseline_d, config.blocks, config.fork_height, config.transition_block,
    config.old_t, config.t, new_equil_d)?;

    // Metrics table
    writeln!(f, "<h2>Post-Fork Steady State Metrics</h2>")?;
    writeln!(f, "<table>")?;
    writeln!(f, "<tr><th>Metric</th><th>Value</th></tr>")?;
    writeln!(f, "<tr><td>Target Solvetime</td><td>{}s</td></tr>", config.t)?;
    writeln!(f, "<tr><td>Avg Solvetime</td><td>{:.1}s</td></tr>", metrics.avg_st)?;
    writeln!(f, "<tr><td>Avg Difficulty</td><td>{:.0}</td></tr>", metrics.avg_d)?;
    writeln!(f, "<tr><td>Expected Difficulty</td><td>{:.0}</td></tr>", new_equil_d)?;
    writeln!(f, "<tr><td>StdDev D (normalized)</td><td>{:.3}</td></tr>", metrics.std_dev_d)?;
    writeln!(f, "<tr><td>StdDev ST (normalized)</td><td>{:.2}</td></tr>", metrics.std_dev_st)?;
    writeln!(f, "</table>")?;

    // Charts
    writeln!(f, "<h2>Charts</h2>")?;
    let engine = base64::engine::general_purpose::STANDARD;
    for (title, bmp_data) in charts {
        let b64 = engine.encode(bmp_data);
        writeln!(f, "<div class=\"chart\">")?;
        writeln!(f, "<h3>{}</h3>", title)?;
        writeln!(f, "<img src=\"data:image/bmp;base64,{}\">", b64)?;
        writeln!(f, "</div>")?;
    }

    writeln!(f, "</body></html>")?;
    Ok(())
}
