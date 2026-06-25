//! Criterion benchmarks for waveform-mcp hotspots.
//!
//! Fixtures live under `<temp_dir>/waveform-mcp-fixtures`. Generate them
//! with `cargo run --release --example gen_fixtures` before benching.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use waveform_mcp::{
    find_conditional_events, find_signal_by_path, find_signal_events, list_signals,
    read_signal_values,
};

const FIXTURES: &[&str] = &["small", "medium"];

#[derive(Copy, Clone)]
enum Format {
    Vcd,
    Fst,
}

impl Format {
    fn ext(self) -> &'static str {
        match self {
            Format::Vcd => "vcd",
            Format::Fst => "fst",
        }
    }
    fn label(self) -> &'static str {
        self.ext()
    }
}

fn fixture_dir() -> &'static Path {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| std::env::temp_dir().join("waveform-mcp-fixtures"))
}

fn fixture_path(name: &str, fmt: Format) -> PathBuf {
    fixture_dir().join(format!("{}.{}", name, fmt.ext()))
}

fn require_fixture(path: &Path) {
    if !path.exists() {
        panic!(
            "missing fixture: {}\n\nGenerate fixtures first:\n  cargo run --release --example gen_fixtures",
            path.display()
        );
    }
}

fn load(path: &Path) -> wellen::simple::Waveform {
    wellen::simple::read(path).expect("failed to load fixture")
}

// A path that is guaranteed to exist in every preset (top.mod_0.sig_0).
const PROBE_SIGNAL: &str = "top.mod_0.sig_0";

// Last var encountered in all_vars() order — exercises the deep end of the
// hierarchy lookup so a future regression to a linear scan would show up here.
fn last_signal_path(wf: &wellen::simple::Waveform) -> String {
    let h = wf.hierarchy();
    let last = h.all_vars().last().expect("fixture has at least one var");
    h[last].full_name(h)
}

fn bench_find_signal_by_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_signal_by_path");
    for name in FIXTURES {
        for fmt in [Format::Vcd, Format::Fst] {
            let path = fixture_path(name, fmt);
            require_fixture(&path);
            let wf = load(&path);
            let h = wf.hierarchy();
            let target = last_signal_path(&wf);
            let id = BenchmarkId::new(fmt.label(), *name);
            group.bench_with_input(id, &target, |b, target| {
                b.iter(|| black_box(find_signal_by_path(h, target)));
            });
        }
    }
    group.finish();
}

fn bench_list_signals(c: &mut Criterion) {
    let mut group = c.benchmark_group("list_signals");
    for name in FIXTURES {
        for fmt in [Format::Vcd, Format::Fst] {
            let path = fixture_path(name, fmt);
            require_fixture(&path);
            let wf = load(&path);
            let h = wf.hierarchy();
            let id = BenchmarkId::new(fmt.label(), *name);
            group.bench_function(id, |b| {
                b.iter(|| black_box(list_signals(h, None, None, true, Some(-1))));
            });
        }
    }
    group.finish();
}

fn bench_find_signal_events(c: &mut Criterion) {
    let mut group = c.benchmark_group("find_signal_events");
    for name in FIXTURES {
        for fmt in [Format::Vcd, Format::Fst] {
            let path = fixture_path(name, fmt);
            require_fixture(&path);
            let mut wf = load(&path);
            let signal_ref =
                find_signal_by_path(wf.hierarchy(), PROBE_SIGNAL).expect("probe signal not found");
            wf.load_signals(&[signal_ref]);
            let end = wf.time_table().len().saturating_sub(1);
            let id = BenchmarkId::new(fmt.label(), *name);
            group.bench_function(id, |b| {
                b.iter(|| {
                    black_box(find_signal_events(&wf, signal_ref, 0, end, -1)).unwrap();
                });
            });
        }
    }
    group.finish();
}

fn bench_find_conditional_events(
    c: &mut Criterion,
    label: &str,
    condition_for: fn(&str) -> String,
) {
    let mut group = c.benchmark_group(format!("find_conditional_events/{}", label));
    for name in FIXTURES {
        for fmt in [Format::Vcd, Format::Fst] {
            let path = fixture_path(name, fmt);
            require_fixture(&path);
            let mut wf = load(&path);
            let end = wf.time_table().len().saturating_sub(1);
            let cond = condition_for(name);
            // Prime the signal cache so the bench measures the steady-state
            // scan, not the initial load_signals I/O.
            let _ = find_conditional_events(&mut wf, &cond, 0, end, 1);
            let id = BenchmarkId::new(fmt.label(), *name);
            group.bench_function(id, move |b| {
                b.iter(|| {
                    black_box(find_conditional_events(&mut wf, &cond, 0, end, -1)).unwrap();
                });
            });
        }
    }
    group.finish();
}

fn bench_conditional_simple(c: &mut Criterion) {
    bench_find_conditional_events(c, "simple", |_| PROBE_SIGNAL.to_string());
}

fn bench_conditional_compare(c: &mut Criterion) {
    bench_find_conditional_events(c, "compare", |_| format!("{} == 8'h55", PROBE_SIGNAL));
}

fn bench_conditional_past_edge(c: &mut Criterion) {
    bench_find_conditional_events(c, "past_edge", |_| {
        format!("!$past({}) && {}", PROBE_SIGNAL, PROBE_SIGNAL)
    });
}

fn bench_read_signal_values(c: &mut Criterion) {
    let mut group = c.benchmark_group("read_signal_values");
    for name in FIXTURES {
        for fmt in [Format::Vcd, Format::Fst] {
            let path = fixture_path(name, fmt);
            require_fixture(&path);
            let mut wf = load(&path);
            let signal_ref =
                find_signal_by_path(wf.hierarchy(), PROBE_SIGNAL).expect("probe signal not found");
            wf.load_signals(&[signal_ref]);
            let n = wf.time_table().len();
            let indices: Vec<usize> = (0..10).map(|k| (k * n) / 10).collect();
            let id = BenchmarkId::new(fmt.label(), *name);
            group.bench_function(id, |b| {
                b.iter(|| {
                    black_box(read_signal_values(&wf, signal_ref, &indices)).unwrap();
                });
            });
        }
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_find_signal_by_path,
    bench_list_signals,
    bench_find_signal_events,
    bench_read_signal_values,
    bench_conditional_simple,
    bench_conditional_compare,
    bench_conditional_past_edge,
);
criterion_main!(benches);
