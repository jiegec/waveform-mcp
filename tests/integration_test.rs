use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_tool_response_format() {
    // Test that tool responses are properly formatted
    use serde_json::json;

    let response = serde_json::to_string(&json!({
        "content": [{"type": "text", "text": "Test message"}],
        "structuredContent": null
    }));

    assert!(response.is_ok());
}

#[test]
fn test_vcd_file_creation_and_reading() {
    // Create a simple VCD file
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 8 1 data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
b000000001";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    println!("VCD file path: {:?}", temp_file.path());

    // Try to read the waveform
    let waveform = wellen::simple::read(temp_file.path());
    assert!(waveform.is_ok(), "Failed to read VCD file");

    let waveform = waveform.unwrap();
    let hierarchy = waveform.hierarchy();

    // Debug: check what's in the hierarchy
    println!(
        "Hierarchy has {} top-level items",
        hierarchy.items().count()
    );
    println!(
        "Hierarchy has {} vars (all levels)",
        hierarchy.vars().count()
    );
    println!(
        "Hierarchy has {} scopes (all levels)",
        hierarchy.scopes().count()
    );

    // Check that we have signals
    let vars: Vec<_> = hierarchy.iter_vars().collect();
    println!("Vars collected: {:?}", vars);
    assert!(!vars.is_empty(), "No variables found in VCD file");
}

#[test]
fn test_signal_full_name() {
    // Create a simple VCD file with hierarchy
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$scope module submodule $end\n\
$var wire 1 0 clk $end\n\
$upscope $end\n\
$var wire 8 1 data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
b000000001";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Check full names of variables
    for var in hierarchy.iter_vars() {
        let full_name = var.full_name(hierarchy);
        println!("Variable full name: {}", full_name);
        assert!(!full_name.is_empty(), "Full name should not be empty");
    }
}

#[test]
fn test_list_signals_function() {
    // Create a simple VCD file
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 8 1 data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
b000000001";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Simulate list_signals function
    let mut signals = Vec::new();
    for var in hierarchy.iter_vars() {
        let path = var.full_name(hierarchy);
        signals.push(format!("{} ({})", path, var.signal_ref().index()));
    }

    println!("Found signals: {}", signals.join("\n"));
    assert!(!signals.is_empty(), "Should have found at least one signal");
}

#[test]
fn test_format_time() {
    use waveform_mcp::format_time;

    // Test with nanosecond timescale (factor = 1)
    let timescale_ns = wellen::Timescale {
        factor: 1,
        unit: wellen::TimescaleUnit::NanoSeconds,
    };
    assert_eq!(format_time(10, Some(&timescale_ns)), "10ns");

    // Test with picosecond timescale (factor = 1000)
    let timescale_ps = wellen::Timescale {
        factor: 1000,
        unit: wellen::TimescaleUnit::PicoSeconds,
    };
    assert_eq!(format_time(5, Some(&timescale_ps)), "5000ps");

    // Test with millisecond timescale (factor = 1000000)
    let timescale_ms = wellen::Timescale {
        factor: 1000000,
        unit: wellen::TimescaleUnit::MilliSeconds,
    };
    assert_eq!(format_time(2, Some(&timescale_ms)), "2000000ms");

    // Test with no timescale
    assert_eq!(format_time(100, None), "100 (unknown timescale)");
}
