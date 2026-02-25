use plotters::prelude::*;
use plotters_bitmap::BitMapBackend;
use crate::Config;
use crate::simulation::SimResult;

const W: u32 = 1600;
const H: u32 = 700;
pub fn plot_fork_transition(
    config: &Config, res: &SimResult,
    transition_block: usize,
) -> Vec<u8> {
    let n = config.n as usize;
    let start = n; // show from early on to see pre-fork steady state
    let old_t = match config.attack_mode {
        crate::AttackMode::ForkTransition { old_t, .. } => old_t as f64,
        _ => 75.0,
    };
    let new_t = config.t as f64;

    // Show a window around the transition: 500 blocks before to 2000 blocks after
    let view_start = if transition_block > 500 { transition_block - 500 } else { start };
    let view_end = (transition_block + 2000).min(res.ds.len());

    // Build difficulty series (absolute values)
    let d_series: Vec<(usize, f64)> = (view_start..view_end)
        .map(|i| (i, res.ds[i] as f64))
        .collect();

    // Build 11-block rolling average solvetime series (absolute seconds)
    let st_series: Vec<(usize, f64)> = (view_start..view_end)
        .filter(|&i| i >= 5 && i + 5 < res.sts.len())
        .map(|i| {
            let sum: f64 = (i.saturating_sub(5)..=(i + 5).min(res.sts.len() - 1))
                .map(|j| res.sts[j] as f64)
                .sum();
            let count = ((i + 5).min(res.sts.len() - 1) - i.saturating_sub(5) + 1) as f64;
            (i, sum / count)
        })
        .collect();

    let d_max = d_series.iter().map(|&(_, d)| d).fold(0.0f64, f64::max) * 1.1;
    let st_max = st_series.iter().map(|&(_, s)| s).fold(0.0f64, f64::max) * 1.15;

    // New equilibrium difficulty: D_new = D_old * new_t / old_t
    let old_d = config.baseline_d as f64;
    let new_equil_d = old_d * new_t / old_t;

    render_to_png(|root| {
        let (upper, lower) = root.split_vertically(H * 55 / 100);

        // Upper chart: Difficulty
        {
            let mut chart = ChartBuilder::on(&upper)
                .caption("Fork Transition 75s -> 25s : Difficulty Adjustment", ("sans-serif", 22))
                .margin(10)
                .x_label_area_size(35)
                .y_label_area_size(70)
                .build_cartesian_2d(view_start as f64..view_end as f64, 0.0f64..d_max)
                .unwrap();

            chart.configure_mesh()
                .x_desc("Block Height")
                .y_desc("Difficulty")
                .x_label_formatter(&|x| format!("{:.0}", x))
                .draw().unwrap();

            // Vertical line at transition
            chart.draw_series(LineSeries::new(
                [(transition_block as f64, 0.0), (transition_block as f64, d_max)],
                ShapeStyle::from(&GREEN.mix(0.7)).stroke_width(2),
            )).unwrap()
                .label("Fork activation")
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], GREEN.mix(0.7).stroke_width(2)));

            // Reference lines: old D and new equilibrium D
            chart.draw_series(LineSeries::new(
                [(view_start as f64, old_d), (view_end as f64, old_d)],
                ShapeStyle::from(&BLUE.mix(0.4)).stroke_width(1),
            )).unwrap()
                .label(format!("Old equil D={:.0}", old_d))
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE.mix(0.4).stroke_width(1)));

            chart.draw_series(LineSeries::new(
                [(view_start as f64, new_equil_d), (view_end as f64, new_equil_d)],
                ShapeStyle::from(&RED.mix(0.4)).stroke_width(1),
            )).unwrap()
                .label(format!("New equil D={:.0} (T=25s)", new_equil_d))
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], RED.mix(0.4).stroke_width(1)));

            // Actual difficulty
            chart.draw_series(LineSeries::new(
                d_series.iter().map(|&(i, d)| (i as f64, d)),
                ShapeStyle::from(&BLUE).stroke_width(2),
            )).unwrap()
                .label("Difficulty")
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE.stroke_width(2)));

            chart.configure_series_labels()
                .position(SeriesLabelPosition::UpperRight)
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw().unwrap();
        }

        // Lower chart: Solvetime
        {
            let mut chart = ChartBuilder::on(&lower)
                .caption("Block Solvetime (11-block rolling avg)", ("sans-serif", 18))
                .margin(10)
                .x_label_area_size(35)
                .y_label_area_size(70)
                .build_cartesian_2d(view_start as f64..view_end as f64, 0.0f64..st_max)
                .unwrap();

            chart.configure_mesh()
                .x_desc("Block Height")
                .y_desc("Solvetime (s)")
                .x_label_formatter(&|x| format!("{:.0}", x))
                .draw().unwrap();

            // Vertical line at transition
            chart.draw_series(LineSeries::new(
                [(transition_block as f64, 0.0), (transition_block as f64, st_max)],
                ShapeStyle::from(&GREEN.mix(0.7)).stroke_width(2),
            )).unwrap();

            // Reference lines: old and new target solvetimes
            chart.draw_series(LineSeries::new(
                [(view_start as f64, old_t), (view_end as f64, old_t)],
                ShapeStyle::from(&BLUE.mix(0.4)).stroke_width(1),
            )).unwrap()
                .label(format!("Old target T={}s", old_t))
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], BLUE.mix(0.4).stroke_width(1)));

            chart.draw_series(LineSeries::new(
                [(view_start as f64, new_t), (view_end as f64, new_t)],
                ShapeStyle::from(&RED.mix(0.4)).stroke_width(1),
            )).unwrap()
                .label(format!("New target T={}s", new_t))
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], RED.mix(0.4).stroke_width(1)));

            // Actual solvetime
            chart.draw_series(LineSeries::new(
                st_series.iter().map(|&(i, s)| (i as f64, s)),
                ShapeStyle::from(&RED).stroke_width(2),
            )).unwrap()
                .label("Avg Solvetime")
                .legend(|(x, y)| PathElement::new([(x, y), (x + 20, y)], RED.stroke_width(2)));

            chart.configure_series_labels()
                .position(SeriesLabelPosition::UpperRight)
                .background_style(WHITE.mix(0.8))
                .border_style(BLACK)
                .draw().unwrap();
        }
    })
}