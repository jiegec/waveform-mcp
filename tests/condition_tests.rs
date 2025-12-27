//! Condition tests

use waveform_mcp::find_conditional_events;

#[test]
fn test_find_conditional_events_lib() {
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
    // Check that event shows both signals as 1
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

#[test]
fn test_past_function() {
    // Create a VCD file with a signal that toggles
    // Signal pattern: 0 -> 1 -> 0 -> 1 -> 0
    // Rising edges at time indices: 1 and 3
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 signal $end\n\
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
00\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test rising edge detection: !$past(TOP.signal) && TOP.signal
    let events =
        find_conditional_events(&mut waveform, "!$past(top.signal) && top.signal", 0, 4, -1)
            .expect("Should find events for rising edge");

    // Should find 2 rising edges at time indices 1 and 3
    assert_eq!(events.len(), 2, "Should find 2 rising edges");
    assert!(
        events[0].contains("Time index 1 (10ns)"),
        "First rising edge at time 1"
    );
    assert!(
        events[1].contains("Time index 3 (30ns)"),
        "Second rising edge at time 3"
    );

    // Test falling edge detection: $past(TOP.signal) && !TOP.signal
    let events =
        find_conditional_events(&mut waveform, "$past(top.signal) && !top.signal", 0, 4, -1)
            .expect("Should find events for falling edge");

    // Should find 2 falling edges at time indices 2 and 4
    assert_eq!(events.len(), 2, "Should find 2 falling edges");
    assert!(
        events[0].contains("Time index 2 (20ns)"),
        "First falling edge at time 2"
    );
    assert!(
        events[1].contains("Time index 4 (40ns)"),
        "Second falling edge at time 4"
    );

    // Test $past with OR condition
    // At time index 0: past=false (no past), current=0 => false || 0 = false
    // At time index 1: past=0, current=1 => 0 || 1 = true
    // At time index 2: past=1, current=0 => 1 || 0 = true
    // At time index 3: past=0, current=1 => 0 || 1 = true
    // At time index 4: past=1, current=0 => 1 || 0 = true
    let events =
        find_conditional_events(&mut waveform, "$past(top.signal) || top.signal", 0, 4, -1)
            .expect("Should find events for OR with $past");

    assert_eq!(events.len(), 4, "Should find 4 events");
}

#[test]
fn test_past_edge_case() {
    // Create a simple VCD file
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 signal $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
#10\n\
10\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test that $past at time index 0 evaluates to false (no past value)
    // At time 0: past=false (no past value), signal=0 => false
    // At time 1: past=0 (from time 0), signal=1 => 0=false
    let events = find_conditional_events(&mut waveform, "$past(top.signal)", 0, 1, -1)
        .expect("Should handle $past at time 0");

    // Should find 0 events since past values are all 0 (false)
    assert_eq!(events.len(), 0, "Should find 0 events");
}

#[test]
fn test_past_with_and_expression() {
    // Create a VCD file with two signals that have overlapping true states
    // Signal pattern:
    // signal1: 0 -> 0 -> 1 -> 0 -> 1 -> 0
    // signal2: 1 -> 1 -> 1 -> 0 -> 1 -> 0
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 signal1 $end\n\
$var wire 1 1 signal2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
11\n\
#10\n\
10\n\
#20\n\
00\n\
#30\n\
10\n\
11\n\
#40\n\
00\n\
01\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test $past(signal1 && signal2)
    // We're checking if the AND expression was true in the previous time step
    // Timeline:
    // time 0: signal1=0, signal2=1 => $past(signal1 && signal2)=false (no past)
    // time 1: signal1=1, signal2=1 => $past(signal1 && signal2)=false (at time 0: 0 && 1 = false)
    // time 2: signal1=0, signal2=1 => $past(signal1 && signal2)=true  (at time 1: 1 && 1 = true)
    // time 3: signal1=1, signal2=1 => $past(signal1 && signal2)=false (at time 2: 0 && 1 = false)
    // time 4: signal1=0, signal2=0 => $past(signal1 && signal2)=true  (at time 3: 1 && 1 = true)
    let events =
        find_conditional_events(&mut waveform, "$past(top.signal1 && top.signal2)", 0, 4, -1)
            .expect("Should find events for $past with AND expression");

    // Should find 2 events at time indices 2 and 4 where the previous time had both signals true
    assert_eq!(
        events.len(),
        2,
        "Should find 2 events where $past(signal1 && signal2) is true"
    );
    assert!(
        events[0].contains("Time index 2 (20ns)"),
        "First event at time 2"
    );
    assert!(
        events[1].contains("Time index 4 (40ns)"),
        "Second event at time 4"
    );
}

#[test]
fn test_nested_past() {
    // Create a VCD file with a signal that toggles
    // Signal pattern: 0 -> 1 -> 0 -> 1 -> 0 -> 1
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 signal $end\n\
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
00\n\
#50\n\
10\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test nested $past($past(signal))
    // Timeline:
    // time 0: signal=0, $past=0(no past), $past($past)=0(no past)
    // time 1: signal=1, $past=0, $past($past)=0(no past at time -1)
    // time 2: signal=0, $past=1, $past($past)=0
    // time 3: signal=1, $past=0, $past($past)=1 (at time 2, $past was 1)
    // time 4: signal=0, $past=1, $past($past)=0
    // time 5: signal=1, $past=0, $past($past)=1 (at time 4, $past was 1)
    let events =
        find_conditional_events(&mut waveform, "$past($past(top.signal))", 0, 5, -1)
            .expect("Should find events for nested $past");

    // Should find 2 events at time indices 3 and 5 where $past($past) is true
    assert_eq!(
        events.len(),
        2,
        "Should find 2 events where $past($past(signal)) is true"
    );
    assert!(
        events[0].contains("Time index 3 (30ns)"),
        "First event at time 3"
    );
    assert!(
        events[1].contains("Time index 5 (50ns)"),
        "Second event at time 5"
    );
}

#[test]
fn test_bitwise_and() {
    // Create a VCD file with two 4-bit signals
    // signal1: 5 (0101), 3 (0011), 10 (1010), 6 (0110)
    // signal2: 3 (0011), 6 (0110), 5 (0101), 4 (0100)
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! signal1 $end\n\
$var wire 4 0 signal2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
b0011 0\n\
#10\n\
b0011 !\n\
b0110 0\n\
#20\n\
b1010 !\n\
b0101 0\n\
#30\n\
b0110 !\n\
b0100 0\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise AND with signal & literal (bitwise AND is non-zero)
    // time 0: 5 & 3 = 0101 & 0011 = 0001 = 1 (non-zero)
    // time 1: 3 & 6 = 0011 & 0110 = 0010 = 2 (non-zero)
    // time 2: 10 & 5 = 1010 & 0101 = 0000 = 0 (zero)
    // time 3: 6 & 4 = 0110 & 0100 = 0100 = 4 (non-zero)
    let events = find_conditional_events(&mut waveform, "top.signal1 & top.signal2", 0, 3, -1)
        .expect("Should find events for bitwise AND");

    // Should find 3 events where result is non-zero (times 0, 1, 3)
    assert_eq!(events.len(), 3, "Should find 3 events where bitwise AND is non-zero");
    assert!(
        events[0].contains("Time index 0 (0ns)"),
        "First event at time 0"
    );
}

#[test]
fn test_bitwise_or() {
    // Create a VCD file with two 4-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! signal1 $end\n\
$var wire 4 0 signal2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
b0011 0\n\
#10\n\
b0010 !\n\
b0100 0\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise OR (just check if non-zero, not specific value)
    // time 0: 5 | 3 = 0101 | 0011 = 0111 = 7 (non-zero)
    // time 1: 2 | 4 = 0010 | 0100 = 0110 = 6 (non-zero)
    let events = find_conditional_events(&mut waveform, "top.signal1 | top.signal2", 0, 1, -1)
        .expect("Should find events for bitwise OR");

    // Both time indices should have non-zero result
    assert_eq!(events.len(), 2, "Should find 2 events where bitwise OR is non-zero");
}

#[test]
fn test_bitwise_xor() {
    // Create a VCD file with two 4-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! signal1 $end\n\
$var wire 4 0 signal2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
b0011 0\n\
#10\n\
b0110 !\n\
b0101 0\n\
#20\n\
b1111 !\n\
b0000 0\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise XOR (just check if non-zero)
    // time 0: 5 ^ 3 = 0101 ^ 0011 = 0110 = 6 (non-zero)
    // time 1: 6 ^ 5 = 0110 ^ 0101 = 0011 = 3 (non-zero)
    // time 2: 15 ^ 0 = 1111 ^ 0000 = 1111 = 15 (non-zero)
    let events = find_conditional_events(&mut waveform, "top.signal1 ^ top.signal2", 0, 2, -1)
        .expect("Should find events for bitwise XOR");

    // All time indices should have non-zero result
    assert_eq!(events.len(), 3, "Should find 3 events where bitwise XOR is non-zero");
}

#[test]
fn test_bitwise_mixed_operations() {
    // Create a VCD file with three 4-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! signal1 $end\n\
$var wire 4 0 signal2 $end\n\
$var wire 4 1 signal3 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
b0011 0\n\
b0110 1\n\
#10\n\
b0011 !\n\
b0110 0\n\
b0100 1\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test mixed bitwise operations: (signal1 & signal2) | signal3
    // time 0: (5 & 3) | 6 = 1 | 6 = 7 (non-zero)
    // time 1: (3 & 6) | 4 = 2 | 4 = 6 (non-zero)
    let events = find_conditional_events(
        &mut waveform,
        "(top.signal1 & top.signal2) | top.signal3",
        0,
        1,
        -1,
    )
    .expect("Should find events for mixed bitwise operations");

    // Both time indices should have non-zero result
    assert_eq!(events.len(), 2, "Should find 2 events for mixed bitwise operations");
}

#[test]
fn test_bitwise_with_logical_operations() {
    // Create a VCD file with 1-bit signals (simpler test)
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 sig1 $end\n\
$var wire 1 1 sig2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
00\n\
11\n\
#10\n\
01\n\
10\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test combining bitwise and logical operations
    // time 0: sig1=0, sig2=1, sig1 & sig2 = 0, false && sig2 = 0 && 1 = false
    // time 1: sig1=0, sig2=1, sig1 & sig2 = 0, false && sig2 = 0 && 1 = false
    let events = find_conditional_events(
        &mut waveform,
        "top.sig1 & top.sig2 && top.sig2",
        0,
        1,
        -1,
    )
    .expect("Should find events for bitwise and logical operations");

    // No events should match (0 & 1 = 0 is false)
    assert_eq!(events.len(), 0, "Should find 0 events");
}

#[test]
fn test_bitwise_zero_result() {
    // Create a VCD file with 1-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 0 sig1 $end\n\
$var wire 1 1 sig2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
10\n\
01\n\
#10\n\
00\n\
00\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise AND that results in 0
    // time 0: sig1=1, sig2=0, 1 & 0 = 0 (zero)
    // time 1: sig1=0, sig2=1, 0 & 1 = 0 (zero)
    let events = find_conditional_events(&mut waveform, "top.sig1 & top.sig2", 0, 1, -1)
        .expect("Should find events for bitwise AND");

    // Both time indices should have zero result
    assert_eq!(events.len(), 0, "Should find 0 events where bitwise AND is zero");
}

#[test]
fn test_single_bit_extraction() {
    // Create a VCD file with a 4-bit signal
    // Value sequence: 0b0101 (5), 0b0011 (3), 0b1010 (10), 0b0110 (6)
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! counter $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
#10\n\
b0011 !\n\
#20\n\
b1010 !\n\
#30\n\
b0110 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test single bit extraction: counter[0] (LSB)
    // time 0: 0b0101 -> bit 0 = 1
    // time 1: 0b0011 -> bit 0 = 1
    // time 2: 0b1010 -> bit 0 = 0
    // time 3: 0b0110 -> bit 0 = 0
    let events = find_conditional_events(&mut waveform, "top.counter[0] == 1'b1", 0, 3, -1)
        .expect("Should find events");

    // Should find events at times 0 and 1 where LSB is 1
    assert_eq!(events.len(), 2, "Should find 2 events where LSB is 1");
    assert!(events[0].contains("Time index 0 (0ns)"), "First event at time 0");
    assert!(events[1].contains("Time index 1 (10ns)"), "Second event at time 1");
}

#[test]
fn test_bit_range_extraction() {
    // Create a VCD file with an 8-bit signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 8 ! data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b10101010 !\n\
#10\n\
b11001100 !\n\
#20\n\
b11110000 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bit range extraction: data[7:4] (upper nibble)
    // time 0: 0b10101010 -> bits[7:4] = 0b1010 = 10
    // time 1: 0b11001100 -> bits[7:4] = 0b1100 = 12
    // time 2: 0b11110000 -> bits[7:4] = 0b1111 = 15
    let events = find_conditional_events(&mut waveform, "top.data[7:4] == 4'b1010", 0, 2, -1)
        .expect("Should find events");

    assert_eq!(events.len(), 1, "Should find 1 event where upper nibble is 0b1010");
    assert!(events[0].contains("Time index 0 (0ns)"), "Event at time 0");

    // Test lower nibble: data[3:0]
    let events = find_conditional_events(&mut waveform, "top.data[3:0] == 4'b1100", 0, 2, -1)
        .expect("Should find events");

    assert_eq!(events.len(), 1, "Should find 1 event where lower nibble is 0b1100");
    assert!(events[0].contains("Time index 1 (10ns)"), "Event at time 1");
}

#[test]
fn test_bit_extraction_with_bitwise() {
    // Create a VCD file with two 8-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 8 ! sig1 $end\n\
$var wire 8 0 sig2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b10101010 !\n\
b11111111 0\n\
#10\n\
b11001100 !\n\
b00000000 0\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise operation on extracted bits
    // time 0: sig1[3:0] = 0b1010 = 10, sig2[3:0] = 0b1111 = 15, 10 & 15 = 10 (non-zero)
    // time 1: sig1[3:0] = 0b1100 = 12, sig2[3:0] = 0b0000 = 0, 12 & 0 = 0 (zero)
    let events = find_conditional_events(&mut waveform, "top.sig1[3:0] & top.sig2[3:0]", 0, 1, -1)
        .expect("Should find events");

    assert_eq!(events.len(), 1, "Should find 1 event where bitwise AND of extracted bits is non-zero");
    assert!(events[0].contains("Time index 0 (0ns)"), "Event at time 0");
}

#[test]
fn test_bit_extraction_equals_zero() {
    // Create a VCD file with a 4-bit signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! counter $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
#10\n\
b0000 !\n\
#20\n\
b1000 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test that extracted bits equal to 0 are handled correctly
    // time 0: counter[2] = 1 (not equal to 0)
    // time 1: counter[2] = 0 (equal to 0)
    // time 2: counter[2] = 0 (equal to 0)
    let events = find_conditional_events(&mut waveform, "top.counter[2] == 1'b0", 0, 2, -1)
        .expect("Should find events");

    assert_eq!(events.len(), 2, "Should find 2 events where extracted bit equals 0");
    assert!(events[0].contains("Time index 1 (10ns)"), "First event at time 1");
    assert!(events[1].contains("Time index 2 (20ns)"), "Second event at time 2");
}

#[test]
fn test_bitwise_not() {
    // Create a VCD file with 4-bit signals
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! sig1 $end\n\
$var wire 4 0 sig2 $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
b0011 0\n\
#10\n\
b0000 !\n\
b1111 0\n\
#20\n\
b1010 !\n\
b1100 0\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise NOT with explicit bit width
    // ~4'b0101 = 4'b1010 = 10 (non-zero)
    // ~4'b0000 = 4'b1111 = 15 (non-zero)
    let events = find_conditional_events(&mut waveform, "~4'b0101", 0, 2, -1)
        .expect("Should find events for bitwise NOT");

    // Time 0: ~5 = 10 (4'b1010) which is non-zero
    // Time 1: ~0 = 15 (4'b1111) which is non-zero
    // Time 2: ~10 = 5 (4'b0101) which is non-zero
    assert_eq!(events.len(), 3, "Should find 3 events where bitwise NOT is non-zero");
    assert!(events[0].contains("Time index 0 (0ns)"), "First event at time 0");
}

#[test]
fn test_bitwise_not_with_signal() {
    // Create a VCD file with 4-bit signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0000 !\n\
#10\n\
b1111 !\n\
#20\n\
b1010 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise NOT on signal
    // time 0: data=0b0000, ~data=0b1111=15 (non-zero)
    // time 1: data=0b1111, ~data=0b0000=0 (zero)
    // time 2: data=0b1010, ~data=0b0101=5 (non-zero)
    let events = find_conditional_events(&mut waveform, "~top.data", 0, 2, -1)
        .expect("Should find events for bitwise NOT on signal");

    assert_eq!(events.len(), 2, "Should find 2 events where bitwise NOT is non-zero");
    assert!(events[0].contains("Time index 0 (0ns)"), "First event at time 0");
}

#[test]
fn test_bitwise_not_with_bit_extract() {
    // Create a VCD file with 8-bit signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 8 ! data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b00000001 !\n\
#10\n\
b00000010 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test bitwise NOT on extracted bits
    // time 0: data[7:0]=1, ~data[7:0]=254 (0b11111110, non-zero)
    // time 1: data[7:0]=2, ~data[7:0]=253 (0b11111101, non-zero)
    let events = find_conditional_events(&mut waveform, "~top.data[7:0]", 0, 1, -1)
        .expect("Should find events for bitwise NOT on bit extraction");

    assert_eq!(events.len(), 2, "Should find 2 events where bitwise NOT of extracted bits is non-zero");
}

#[test]
fn test_bitwise_not_with_bitwise_ops() {
    // Create a VCD file with 4-bit signal
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 4 ! data $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
b0101 !\n\
#10\n\
b1010 !\n\
";

    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::fs::write(temp_file.path(), vcd_content).expect("Failed to write VCD file");

    let mut waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");

    // Test combining bitwise NOT with other bitwise operations
    // time 0: data=5 (0b0101), ~data=10 (0b1010), ~data & data=10 & 5 = 0 (zero)
    // time 1: data=10 (0b1010), ~data=5 (0b0101), ~data & data=5 & 10 = 0 (zero)
    let events = find_conditional_events(&mut waveform, "~top.data & top.data", 0, 1, -1)
        .expect("Should find events for bitwise NOT and AND");

    // Both should result in 0, so no events
    assert_eq!(events.len(), 0, "Should find 0 events where (~data & data) is zero");
}

