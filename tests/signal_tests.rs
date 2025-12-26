//! Signal tests

use std::io::Write;
use tempfile::NamedTempFile;
use waveform_mcp::find_signal_by_path;
use waveform_mcp::find_signal_events;
use waveform_mcp::get_signal_metadata;
use waveform_mcp::read_signal_values;

#[test]
fn test_read_signal_values_lib() {
    // Create a VCD file with signal changes
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 9 1 test $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
b101010101 1\n\
#10\n\
10\n\
b010010111 1\n\
#20\n\
00\n\
b000011011 1";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Find the signal
    let signal_ref =
        find_signal_by_path(hierarchy, "top.clk").expect("Should find 'top.clk' signal");

    // Load the signal
    waveform.load_signals(&[signal_ref]);

    // Read values at different time indices
    let values = read_signal_values(&waveform, signal_ref, &[0, 1, 2, 3])
        .expect("Should read signal values");

    assert_eq!(values.len(), 4, "Should read 4 values");
    assert!(values[0].contains("0ns"), "First value should be at 0ns");
    assert!(values[1].contains("10ns"), "Second value should be at 10ns");

    // Find the signal
    let hierarchy = waveform.hierarchy();
    let signal_ref =
        find_signal_by_path(hierarchy, "top.test").expect("Should find 'top.test' signal");

    // Load the signal
    waveform.load_signals(&[signal_ref]);

    // Read values at different time indices
    let values = read_signal_values(&waveform, signal_ref, &[0, 1, 2, 3])
        .expect("Should read signal values");

    assert_eq!(values.len(), 4, "Should read 4 values");
    assert!(values[0].contains("9'h155"), "First value should be 9'h155");
}

#[test]
fn test_get_signal_metadata_lib() {
    // Create a VCD file with different signal types
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 4 1 data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
b0000 1";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Test getting metadata for a 1-bit signal
    let clk_metadata =
        get_signal_metadata(hierarchy, "top.clk").expect("Should get metadata for 'top.clk'");
    assert!(
        clk_metadata.contains("top.clk"),
        "Metadata should contain signal name"
    );
    assert!(
        clk_metadata.contains("1 bits"),
        "Metadata should show 1 bit width"
    );

    // Test getting metadata for a 4-bit signal
    let data_metadata =
        get_signal_metadata(hierarchy, "top.data").expect("Should get metadata for 'top.data'");
    assert!(
        data_metadata.contains("top.data"),
        "Metadata should contain signal name"
    );
    assert!(
        data_metadata.contains("4 bits"),
        "Metadata should show 4 bit width"
    );

    // Test getting metadata for non-existent signal
    let error = get_signal_metadata(hierarchy, "nonexistent");
    assert!(
        error.is_err(),
        "Should return error for non-existent signal"
    );
}

#[test]
fn test_find_signal_events_lib() {
    // Create a VCD file with multiple signal changes
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
#10\n\
10\n\
#20\n\
00\n\
#30\n\
10\n\
#40\n\
00";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Find the signal
    let signal_ref =
        find_signal_by_path(hierarchy, "top.clk").expect("Should find 'top.clk' signal");

    // Load the signal
    waveform.load_signals(&[signal_ref]);

    // Find all events
    let events =
        find_signal_events(&waveform, signal_ref, 0, 10, -1).expect("Should find signal events");
    assert!(!events.is_empty(), "Should find at least one event");

    // Find events with limit
    let limited_events =
        find_signal_events(&waveform, signal_ref, 0, 10, 2).expect("Should find limited events");
    assert_eq!(limited_events.len(), 2, "Should limit to 2 events");

    // Find events in a specific time range
    let range_events =
        find_signal_events(&waveform, signal_ref, 2, 3, -1).expect("Should find events in range");
    assert!(
        !range_events.is_empty(),
        "Should find events in specified range"
    );
}
