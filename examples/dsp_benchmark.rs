//! Repeatable CPU benchmark and bit-exact audio regression fixture.
//! cargo run --release --example dsp_benchmark -- benchmark-results
#[path = "../src/dsp.rs"]
#[allow(dead_code)]
mod dsp;
#[path = "../src/presets.rs"]
mod presets;

use std::{hint::black_box, io::Write, path::PathBuf, time::Instant};

fn main() {
    let dir = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&dir).unwrap();
    let input: Vec<f32> = (0..48000)
        .map(|i| {
            let t = i as f32 / 48000.;
            ((std::f32::consts::TAU * 170. * t).sin() * 0.18
                + (std::f32::consts::TAU * 340. * t).sin() * 0.08)
                * (1. - t * 0.5)
        })
        .collect();
    let all = presets::factory();
    let mut report = Vec::new();
    for preset in &all {
        let mut timings = Vec::new();
        for _ in 0..5 {
            let mut processor = dsp::Dsp::new(48000.);
            processor.configure(preset.params);
            let start = Instant::now();
            // 10 seconds per preset, callback-sized blocks, unchanged controls.
            for _ in 0..10 {
                for block in input.chunks(480) {
                    processor.configure(black_box(preset.params));
                    for &sample in block {
                        black_box(processor.tick(black_box(sample), true));
                    }
                }
            }
            timings.push(start.elapsed().as_secs_f64() * 1000.);
        }
        timings.sort_by(f64::total_cmp);
        report.push(
            serde_json::json!({"preset": preset.name, "median_ms": timings[2], "runs_ms": timings}),
        );
        let mut processor = dsp::Dsp::new(48000.);
        processor.configure(preset.params);
        let mut fixture = std::io::BufWriter::new(
            std::fs::File::create(dir.join(format!("{}.f32", preset.name))).unwrap(),
        );
        // Include bypass and a control change so cached coefficients are exercised.
        for pass in 0..3 {
            if pass == 2 {
                let mut changed = preset.params;
                changed.pitch = -changed.pitch;
                changed.delay_ms = 350.;
                changed.room = 0.9;
                processor.configure(changed);
            }
            for &sample in &input {
                fixture
                    .write_all(&processor.tick(sample, pass != 1).to_le_bytes())
                    .unwrap();
            }
        }
    }
    let total: f64 = report
        .iter()
        .map(|v| v["median_ms"].as_f64().unwrap())
        .sum();
    let result = serde_json::json!({"sample_rate":48000,"seconds_per_preset":10,"repeats":5,"sum_median_ms":total,"presets":report});
    std::fs::write(
        dir.join("dsp.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
    println!("240 seconds audio: {total:.2} ms (sum of preset medians)");
}
