//! Synthetic waveform fixture generator.
//!
//! Emits matching .vcd and .fst files from a single in-memory model so
//! benchmarks compare format-independent costs cleanly.

use clap::Parser;
use fst_writer::{
    FstFileType, FstInfo, FstScopeType, FstSignalType, FstVarDirection, FstVarType, open_fst,
};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

const FST_FLUSH_AT: usize = 128 * 1024 * 1024;

#[derive(Parser, Debug)]
#[command(about = "Generate matching synthetic VCD and FST fixtures for benchmarking")]
struct Args {
    /// Number of modules directly under `top`
    #[arg(long, default_value_t = 10)]
    modules: usize,

    /// Signals per module
    #[arg(long, default_value_t = 100)]
    signals_per_module: usize,

    /// Bit width of each signal
    #[arg(long, default_value_t = 8)]
    width: u32,

    /// Number of time steps
    #[arg(long, default_value_t = 1000)]
    time_steps: usize,

    /// Probability a signal changes at a given time step (0.0 to 1.0)
    #[arg(long, default_value_t = 0.1)]
    density: f64,

    /// PRNG seed
    #[arg(long, default_value_t = 42)]
    seed: u64,

    /// Output VCD path
    #[arg(long)]
    out_vcd: PathBuf,

    /// Output FST path
    #[arg(long)]
    out_fst: PathBuf,
}

fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn bits_of(value: u64, width: u32) -> String {
    let mut s = String::with_capacity(width as usize);
    for bit in (0..width).rev() {
        s.push(if (value >> bit) & 1 == 1 { '1' } else { '0' });
    }
    s
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    assert!(
        args.width >= 1 && args.width <= 64,
        "width must be 1..=64 (generator uses u64 internally)"
    );
    assert!(
        (0.0..=1.0).contains(&args.density),
        "density must be in [0.0, 1.0]"
    );

    let n_signals = args.modules * args.signals_per_module;
    eprintln!(
        "Generating {} signals ({} modules x {}) x {} time steps (density {:.3}, width {})",
        n_signals, args.modules, args.signals_per_module, args.time_steps, args.density, args.width
    );

    let mut vcd = BufWriter::new(std::fs::File::create(&args.out_vcd)?);
    writeln!(vcd, "$date synthetic $end")?;
    writeln!(vcd, "$version waveform-mcp gen_synthetic $end")?;
    writeln!(vcd, "$timescale 1ns $end")?;
    writeln!(vcd, "$scope module top $end")?;

    let info = FstInfo {
        start_time: 0,
        timescale_exponent: -9,
        version: "waveform-mcp gen_synthetic".to_string(),
        date: "synthetic".to_string(),
        file_type: FstFileType::Verilog,
    };
    let mut fst = open_fst(&args.out_fst, &info)?;
    fst.scope("top", "", FstScopeType::Module)?;

    let mut vcd_codes = Vec::with_capacity(n_signals);
    let mut fst_ids = Vec::with_capacity(n_signals);
    let mut idx = 0usize;
    for m in 0..args.modules {
        let mod_name = format!("mod_{}", m);
        writeln!(vcd, "$scope module {} $end", mod_name)?;
        fst.scope(&mod_name, "", FstScopeType::Module)?;
        for s in 0..args.signals_per_module {
            let sig_name = format!("sig_{}", s);
            let code = format!("s{}", idx);
            writeln!(vcd, "$var wire {} {} {} $end", args.width, code, sig_name)?;
            let fst_id = fst.var(
                &sig_name,
                FstSignalType::bit_vec(args.width),
                FstVarType::Wire,
                FstVarDirection::Implicit,
                None,
            )?;
            vcd_codes.push(code);
            fst_ids.push(fst_id);
            idx += 1;
        }
        writeln!(vcd, "$upscope $end")?;
        fst.up_scope()?;
    }
    writeln!(vcd, "$upscope $end")?;
    writeln!(vcd, "$enddefinitions $end")?;
    fst.up_scope()?;
    let mut fst = fst.finish()?;

    let max_val: u64 = if args.width == 64 {
        u64::MAX
    } else {
        (1u64 << args.width) - 1
    };
    let threshold = (args.density * (u64::MAX as f64)) as u64;
    let mut current = vec![0u64; n_signals];
    let mut rng = args.seed.max(1);

    for t in 0..args.time_steps {
        writeln!(vcd, "#{}", t)?;
        fst.time_change(t as u64)?;

        for i in 0..n_signals {
            let force = t == 0;
            let roll = xorshift(&mut rng);
            if !force && roll >= threshold {
                continue;
            }
            let new_val = xorshift(&mut rng) & max_val;
            if !force && new_val == current[i] {
                continue;
            }
            current[i] = new_val;
            let bits = bits_of(new_val, args.width);
            writeln!(vcd, "b{} {}", bits, vcd_codes[i])?;
            fst.signal_change(fst_ids[i], bits.as_bytes())?;
        }

        if fst.size() >= FST_FLUSH_AT {
            fst.flush()?;
        }
    }

    vcd.flush()?;
    fst.finish()?;

    eprintln!(
        "Wrote VCD ({} bytes) and FST ({} bytes)",
        std::fs::metadata(&args.out_vcd)?.len(),
        std::fs::metadata(&args.out_fst)?.len(),
    );

    Ok(())
}
