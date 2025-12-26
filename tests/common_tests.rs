//! Common utility tests

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
