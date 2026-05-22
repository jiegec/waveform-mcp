//! Generate the bench fixtures the waveform_ops benches expect.
//!
//! Shells out to the `gen_synthetic` example for each preset so the
//! generation logic lives in one place. Writes everything under
//! `<temp_dir>/waveform-mcp-fixtures`.

use std::path::PathBuf;
use std::process::Command;

struct Preset {
    name: &'static str,
    modules: usize,
    signals_per_module: usize,
    time_steps: usize,
}

const PRESETS: &[Preset] = &[
    Preset {
        name: "small",
        modules: 50,
        signals_per_module: 10,
        time_steps: 100,
    },
    Preset {
        name: "medium",
        modules: 100,
        signals_per_module: 100,
        time_steps: 1000,
    },
];

fn fixture_dir() -> PathBuf {
    std::env::temp_dir().join("waveform-mcp-fixtures")
}

fn main() {
    let dest = fixture_dir();
    std::fs::create_dir_all(&dest).expect("create fixture dir");

    for p in PRESETS {
        let vcd = dest.join(format!("{}.vcd", p.name));
        let fst = dest.join(format!("{}.fst", p.name));
        eprintln!(
            "Generating preset '{}': {} signals x {} time steps",
            p.name,
            p.modules * p.signals_per_module,
            p.time_steps
        );

        let status = Command::new(env!("CARGO"))
            .args([
                "run",
                "--release",
                "--quiet",
                "--example",
                "gen_synthetic",
                "--",
                "--modules",
                &p.modules.to_string(),
                "--signals-per-module",
                &p.signals_per_module.to_string(),
                "--time-steps",
                &p.time_steps.to_string(),
                "--density",
                "0.1",
                "--out-vcd",
                vcd.to_str().expect("vcd path is utf-8"),
                "--out-fst",
                fst.to_str().expect("fst path is utf-8"),
            ])
            .status()
            .expect("spawn cargo");
        assert!(
            status.success(),
            "gen_synthetic failed for preset '{}'",
            p.name
        );
    }

    eprintln!("Fixtures written to {}", dest.display());
}
