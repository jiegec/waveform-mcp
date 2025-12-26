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
fn test_format_signal_value() {
    use waveform_mcp::format_signal_value;

    // Test Event
    let event = wellen::SignalValue::Event;
    assert_eq!(format_signal_value(event), "Event");

    // Test Binary (2-bit)
    let binary_data: [u8; 1] = [2];
    let binary = wellen::SignalValue::Binary(&binary_data, 2);
    assert_eq!(format_signal_value(binary), "2'b10");

    // Test Binary (1-bit)
    let binary_data1: [u8; 1] = [1];
    let binary1 = wellen::SignalValue::Binary(&binary_data1, 1);
    assert_eq!(format_signal_value(binary1), "1'b1");

    // Test Binary (16-bit - should use hex)
    let binary_data16: [u8; 2] = [0x55, 0x55];
    let binary16 = wellen::SignalValue::Binary(&binary_data16, 16);
    assert_eq!(format_signal_value(binary16), "16'h5555");

    // Test Binary (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xd];
    let binary8 = wellen::SignalValue::Binary(&binary_data8, 8);
    assert_eq!(format_signal_value(binary8), "8'h0d");

    // Test Binary (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xcd];
    let binary8 = wellen::SignalValue::Binary(&binary_data8, 8);
    assert_eq!(format_signal_value(binary8), "8'hcd");

    // Test Binary (9-bit - should use hex)
    let binary_data9: [u8; 2] = [0x1, 0xcd];
    let binary9 = wellen::SignalValue::Binary(&binary_data9, 9);
    assert_eq!(format_signal_value(binary9), "9'h1cd");

    // Test FourValue
    let four_data: [u8; 1] = [0];
    let four = wellen::SignalValue::FourValue(&four_data, 1);
    assert_eq!(format_signal_value(four), "[0]");

    // Test NineValue
    let nine_data: [u8; 1] = [0];
    let nine = wellen::SignalValue::NineValue(&nine_data, 1);
    assert_eq!(format_signal_value(nine), "[0]");

    // Test String
    let string = wellen::SignalValue::String("test");
    assert_eq!(format_signal_value(string), "test");

    // Test Real
    let real = wellen::SignalValue::Real(3.15);
    assert_eq!(format_signal_value(real), "3.15");
}

#[test]
fn test_find_signal_by_path() {
    use waveform_mcp::find_signal_by_path;

    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 ! clk $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
0!";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Debug: print all available signals
    for var in hierarchy.iter_vars() {
        println!("Signal path: {}", var.full_name(hierarchy));
    }

    // Test finding existing signal - use the correct path format
    let result = find_signal_by_path(hierarchy, "top.clk");
    assert!(result.is_some(), "Should find 'top.clk' signal");

    // Test finding non-existing signal
    let result = find_signal_by_path(hierarchy, "nonexistent");
    assert!(result.is_none(), "Should not find 'nonexistent' signal");
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

#[test]
fn test_list_signals_recursive() {
    // Create a VCD file with nested hierarchy
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 ! clk $end\n\
$scope module submodule1 $end\n\
$var wire 1 @ data1 $end\n\
$scope module inner $end\n\
$var wire 1 # data2 $end\n\
$upscope $end\n\
$upscope $end\n\
$scope module submodule2 $end\n\
$var wire 1 $ data3 $end\n\
$upscope $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
0!\n\
0@\n\
0#\n\
0$";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Test recursive mode (default): should find all signals at all levels
    let mut all_signals: Vec<String> = Vec::new();
    for var in hierarchy.iter_vars() {
        all_signals.push(var.full_name(hierarchy));
    }
    assert!(
        !all_signals.is_empty(),
        "Should find signals in recursive mode"
    );

    // Test non-recursive mode at top level (top scope)
    let mut top_level_signals: Vec<String> = Vec::new();
    for scope_ref in hierarchy.scopes() {
        let scope = &hierarchy[scope_ref];
        let scope_path = scope.full_name(hierarchy);
        if scope_path == "top" {
            // Top-level scope
            for var_ref in scope.vars(hierarchy) {
                let var = &hierarchy[var_ref];
                top_level_signals.push(var.full_name(hierarchy));
            }
            break;
        }
    }
    assert_eq!(
        top_level_signals.len(),
        1,
        "Should find 1 signal at top level"
    );
    assert!(
        top_level_signals.contains(&"top.clk".to_string()),
        "Should find 'top.clk' at top level"
    );

    // Test non-recursive mode at submodule level
    let mut submodule1_signals: Vec<String> = Vec::new();

    if let Some(submodule1_ref) = waveform_mcp::find_scope_by_path(hierarchy, "top.submodule1") {
        let submodule1 = &hierarchy[submodule1_ref];
        for var_ref in submodule1.vars(hierarchy) {
            let var = &hierarchy[var_ref];
            submodule1_signals.push(var.full_name(hierarchy));
        }
    }

    assert_eq!(
        submodule1_signals.len(),
        1,
        "Should find 1 signal at submodule1 level"
    );
    assert!(
        submodule1_signals.contains(&"top.submodule1.data1".to_string()),
        "Should find 'data1' at submodule1 level"
    );

    // Verify that inner module signal is not included in submodule1 non-recursive list
    assert!(
        !submodule1_signals.iter().any(|s| s.contains("inner")),
        "Should not include inner module signals in submodule1 non-recursive list"
    );
}

#[test]
fn test_list_signals_lib() {
    use waveform_mcp::list_signals;

    // Create a VCD file with multiple signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 1 1 data $end\n\
$var wire 1 2 enable $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
01\n\
02";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Test listing all signals (recursive)
    let signals = list_signals(hierarchy, None, None, true, None);
    assert_eq!(signals.len(), 3, "Should find 3 signals");

    // Test filtering by name pattern
    let clk_signals = list_signals(hierarchy, Some("clk"), None, true, None);
    assert_eq!(clk_signals.len(), 1, "Should find 1 signal matching 'clk'");
    assert!(
        clk_signals[0].contains("clk"),
        "Signal should contain 'clk'"
    );

    // Test filtering by hierarchy prefix
    let top_signals = list_signals(hierarchy, None, Some("top"), true, None);
    assert_eq!(top_signals.len(), 3, "Should find 3 signals under 'top'");

    // Test limit
    let limited_signals = list_signals(hierarchy, None, None, true, Some(2));
    assert_eq!(limited_signals.len(), 2, "Should limit to 2 signals");

    // Test unlimited limit (-1)
    let unlimited_signals = list_signals(hierarchy, None, None, true, Some(-1));
    assert_eq!(
        unlimited_signals.len(),
        3,
        "Should return all signals with -1 limit"
    );
}

#[test]
fn test_read_signal_values_lib() {
    use waveform_mcp::read_signal_values;

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
b100101011 1\n\
#10\n\
10\n\
#20\n\
00\n\
#30\n\
10";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Find the signal
    let signal_ref = waveform_mcp::find_signal_by_path(hierarchy, "top.clk")
        .expect("Should find 'top.clk' signal");

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
    let signal_ref = waveform_mcp::find_signal_by_path(hierarchy, "top.test")
        .expect("Should find 'top.test' signal");

    // Load the signal
    waveform.load_signals(&[signal_ref]);

    // Read values at different time indices
    let values = read_signal_values(&waveform, signal_ref, &[0, 1, 2, 3])
        .expect("Should read signal values");

    assert_eq!(values.len(), 4, "Should read 4 values");
    assert!(values[0].contains("9'h12b"), "First value should be 9'h12b");
}

#[test]
fn test_get_signal_metadata_lib() {
    use waveform_mcp::get_signal_metadata;

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
    use waveform_mcp::find_signal_events;

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
    let signal_ref = waveform_mcp::find_signal_by_path(hierarchy, "top.clk")
        .expect("Should find 'top.clk' signal");

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

#[test]
fn test_find_conditional_events_lib() {
    use waveform_mcp::find_conditional_events;

    // Create a VCD file with multiple signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 clk $end\n\
$var wire 1 1 valid $end\n\
$var wire 1 2 ready $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
01\n\
02\n\
#10\n\
10\n\
01\n\
02\n\
#20\n\
10\n\
11\n\
02\n\
#30\n\
00\n\
11\n\
02\n\
#40\n\
00\n\
11\n\
12\n\
#50\n\
00\n\
01\n\
12\n\
#60\n\
00\n\
01\n\
02\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test simple AND condition - use valid time range (0-6 for 7 time points)
    let events = find_conditional_events(&mut waveform, "top.valid && top.ready", 0, 6, -1)
        .expect("Should find events for AND condition");
    assert!(!events.is_empty(), "Should find at least one event");
    // Check that the event shows both signals as 1
    assert!(
        events[0].contains("top.valid = 1'b1"),
        "Should show valid as 1"
    );
    assert!(
        events[0].contains("top.ready = 1'b1"),
        "Should show ready as 1"
    );
    // Check timestamp - valid && ready is true only at time index 4 (40ns)
    assert!(
        events[0].contains("Time index 4 (40ns)"),
        "Event should be at time index 4 (40ns)"
    );

    // Test OR condition
    let events = find_conditional_events(&mut waveform, "top.valid || top.ready", 0, 6, -1)
        .expect("Should find events for OR condition");
    assert!(!events.is_empty(), "Should find at least one event");
    // Check timestamps - valid || ready is true at time indices 2, 3, 4, 5 (20ns, 30ns, 40ns, 50ns)
    assert_eq!(
        events.len(),
        4,
        "Should find 4 events where valid || ready is true"
    );
    assert!(
        events[0].contains("Time index 2 (20ns)"),
        "First event at time 2"
    );
    assert!(
        events[1].contains("Time index 3 (30ns)"),
        "Second event at time 3"
    );
    assert!(
        events[2].contains("Time index 4 (40ns)"),
        "Third event at time 4"
    );
    assert!(
        events[3].contains("Time index 5 (50ns)"),
        "Fourth event at time 5"
    );

    // Test NOT condition
    let events = find_conditional_events(&mut waveform, "!top.clk", 0, 6, -1)
        .expect("Should find events for NOT condition");
    assert!(!events.is_empty(), "Should find at least one event");
    assert!(events[0].contains("top.clk = 1'b0"), "Should show clk as 0");
    // Check timestamps - !clk is true at time indices 0, 3, 4, 5, 6 (0ns, 30ns, 40ns, 50ns, 60ns)
    assert_eq!(events.len(), 5, "Should find 5 events where !clk is true");
    assert!(
        events[0].contains("Time index 0 (0ns)"),
        "First event at time 0"
    );
    assert!(
        events[1].contains("Time index 3 (30ns)"),
        "Second event at time 3"
    );
    assert!(
        events[2].contains("Time index 4 (40ns)"),
        "Third event at time 4"
    );
    assert!(
        events[3].contains("Time index 5 (50ns)"),
        "Fourth event at time 5"
    );
    assert!(
        events[4].contains("Time index 6 (60ns)"),
        "Fifth event at time 6"
    );

    // Test complex condition with parentheses
    let events = find_conditional_events(
        &mut waveform,
        "top.clk && (top.valid || top.ready)",
        0,
        6,
        -1,
    )
    .expect("Should find events for complex condition");
    assert!(!events.is_empty(), "Should find at least one event");
    // Check timestamps - clk && (valid || ready) is true only at time indices 2 (20ns)
    assert_eq!(
        events.len(),
        1,
        "Should find 2 events for complex condition"
    );
    assert!(
        events[0].contains("Time index 2 (20ns)"),
        "First event at time 2"
    );

    // Test limit
    let events = find_conditional_events(&mut waveform, "top.valid", 0, 6, 2)
        .expect("Should find events with limit");
    assert_eq!(events.len(), 2, "Should limit to 2 events");

    // Test time range
    let events = find_conditional_events(&mut waveform, "top.valid && top.ready", 3, 5, -1)
        .expect("Should find events in time range");
    assert!(!events.is_empty(), "Should find events in specified range");
    // Check timestamp - valid && ready at time 4 is within range 3-5
    assert!(
        events[0].contains("Time index 4 (40ns)"),
        "Event should be at time index 4 (40ns)"
    );
}

#[test]
fn test_conditional_events_timestamps() {
    use waveform_mcp::find_conditional_events;

    // Create a VCD file with a counter that increments each time step
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! counter $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0000 !\n\
#10\n\
b0001 !\n\
#20\n\
b0010 !\n\
#30\n\
b0011 !\n\
#40\n\
b0100 !\n\
#50\n\
b0101 !\n\
#60\n\
b0110 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test that events are found at correct timestamps
    // counter == 2 (4'h2) should be found at time index 2 (20ns)
    let events = find_conditional_events(&mut waveform, "top.counter == 4'b0010", 0, 6, -1)
        .expect("Should find events");
    assert_eq!(events.len(), 1, "Should find exactly 1 event");
    assert!(
        events[0].contains("Time index 2 (20ns)"),
        "Event should be at time 2 (20ns)"
    );
    assert!(
        events[0].contains("top.counter = 4'b0010"),
        "Should show counter = 2"
    );

    // counter == 5 (4'h5) should be found at time index 5 (50ns)
    let events = find_conditional_events(&mut waveform, "top.counter == 4'b0101", 0, 6, -1)
        .expect("Should find events");
    assert_eq!(events.len(), 1, "Should find exactly 1 event");
    assert!(
        events[0].contains("Time index 5 (50ns)"),
        "Event should be at time 5 (50ns)"
    );

    // counter != 0 should be found at time indices 1-6 (10ns-60ns)
    let events = find_conditional_events(&mut waveform, "top.counter != 4'b0000", 0, 6, -1)
        .expect("Should find events");
    assert_eq!(events.len(), 6, "Should find 6 events where counter != 0");
    assert!(
        events[0].contains("Time index 1 (10ns)"),
        "First event at time 1"
    );
    assert!(
        events[5].contains("Time index 6 (60ns)"),
        "Last event at time 6"
    );
}

#[test]
fn test_parse_condition() {
    use waveform_mcp::find_conditional_events;

    // Test basic parsing by calling find_conditional_events with invalid condition
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 sig1 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test invalid signal name
    let result = find_conditional_events(&mut waveform, "nonexistent.signal", 0, 0, -1);
    assert!(result.is_err(), "Should fail for nonexistent signal");

    // Test invalid syntax (unclosed parenthesis)
    let result = find_conditional_events(&mut waveform, "(top.sig1 && top.sig1", 0, 0, -1);
    assert!(result.is_err(), "Should fail for invalid syntax");
}

#[test]
fn test_comparison_operators() {
    use waveform_mcp::find_conditional_events;

    // Create a VCD file with a counter signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! counter $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0000 !\n\
#10\n\
b0001 !\n\
#20\n\
b0010 !\n\
#30\n\
b0011 !\n\
#40\n\
b0100 !\n\
#50\n\
b0101 !\n\
#60\n\
b0110 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test equality with binary literal
    let events = find_conditional_events(&mut waveform, "top.counter == 4'b0101", 0, 6, -1)
        .expect("Should find events for binary literal comparison");
    assert!(!events.is_empty(), "Should find at least one event");
    assert!(
        events[0].contains("top.counter = 4'b0101"),
        "Should show counter value 5 (4'b0101) {}",
        events[0]
    );

    // Test equality with decimal literal
    let events = find_conditional_events(&mut waveform, "top.counter == 3'd3", 0, 6, -1)
        .expect("Should find events for decimal literal comparison");
    assert!(!events.is_empty(), "Should find at least one event");
    assert!(
        events[0].contains("top.counter = 4'b0011"),
        "Should show counter value 3"
    );

    // Test equality with hex literal
    let events = find_conditional_events(&mut waveform, "top.counter == 4'h6", 0, 6, -1)
        .expect("Should find events for hex literal comparison");
    assert!(!events.is_empty(), "Should find at least one event");
    assert!(
        events[0].contains("top.counter = 4'b0110"),
        "Should show counter value 6"
    );

    // Test inequality
    let events = find_conditional_events(&mut waveform, "top.counter != 4'b0000", 0, 6, -1)
        .expect("Should find events for inequality comparison");
    assert!(!events.is_empty(), "Should find at least one event");

    // Test complex condition with comparison
    let events = find_conditional_events(
        &mut waveform,
        "top.counter == 4'b0101 || top.counter == 4'b0011",
        0,
        6,
        -1,
    )
    .expect("Should find events for complex comparison");
    assert_eq!(
        events.len(),
        2,
        "Should find 2 events matching either condition"
    );
}

#[test]
fn test_verilog_literal_parsing() {
    use waveform_mcp::find_conditional_events;

    // Test invalid literal format
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 sig1 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test invalid literal format
    let result = find_conditional_events(&mut waveform, "top.sig1 == invalid", 0, 0, -1);
    assert!(result.is_err(), "Should fail for invalid literal format");

    // Test single = (should be ==)
    let result = find_conditional_events(&mut waveform, "top.sig1 = 1", 0, 0, -1);
    assert!(result.is_err(), "Should fail for single =");
}
