//! Hierarchy tests

use std::io::Write;
use tempfile::NamedTempFile;
use waveform_mcp::find_scope_by_path;
use waveform_mcp::find_signal_by_path;
use waveform_mcp::list_signals;
use waveform_mcp::read_hierarchy;

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
    for var_ref in hierarchy.all_vars() {
        let var = &hierarchy[var_ref];
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
    for var_ref in hierarchy.all_vars() {
        let var = &hierarchy[var_ref];
        let path = var.full_name(hierarchy);
        signals.push(format!("{} ({})", path, var.signal_ref().index()));
    }

    println!("Found signals: {}", signals.join("\n"));
    assert!(!signals.is_empty(), "Should have found at least one signal");
}

#[test]
fn test_find_signal_by_path() {
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
    for var_ref in hierarchy.all_vars() {
        println!("Signal path: {}", hierarchy[var_ref].full_name(hierarchy));
    }

    // Test finding existing signal - use the correct path format
    let result = find_signal_by_path(hierarchy, "top.clk");
    assert!(result.is_some(), "Should find 'top.clk' signal");

    // Test finding non-existing signal
    let result = find_signal_by_path(hierarchy, "nonexistent");
    assert!(result.is_none(), "Should not find 'nonexistent' signal");
}

#[test]
fn test_find_signal_by_path_nested() {
    // Hierarchy with three levels and a shadowed signal name (`clk` exists in
    // both `top` and `top.sub`) so we can verify the lookup respects scope.
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$var wire 1 ! clk $end\n\
$scope module sub $end\n\
$var wire 1 @ clk $end\n\
$var wire 8 # data $end\n\
$scope module inner $end\n\
$var wire 1 $ ready $end\n\
$upscope $end\n\
$upscope $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
0!\n\
1@\n\
b00001010 #\n\
1$";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    // Lookups at every depth resolve.
    let top_clk = find_signal_by_path(hierarchy, "top.clk").expect("top.clk should resolve");
    let sub_clk =
        find_signal_by_path(hierarchy, "top.sub.clk").expect("top.sub.clk should resolve");
    let sub_data =
        find_signal_by_path(hierarchy, "top.sub.data").expect("top.sub.data should resolve");
    let inner_ready = find_signal_by_path(hierarchy, "top.sub.inner.ready")
        .expect("top.sub.inner.ready should resolve");

    // Shadowed name resolves to the right scope (top.clk and top.sub.clk are distinct signals).
    assert_ne!(
        top_clk, sub_clk,
        "top.clk and top.sub.clk must refer to different signals"
    );
    // And these are all distinct from one another.
    assert_ne!(top_clk, sub_data);
    assert_ne!(sub_clk, sub_data);
    assert_ne!(sub_data, inner_ready);

    // Right leaf name but wrong scope returns None.
    assert!(
        find_signal_by_path(hierarchy, "top.data").is_none(),
        "top.data does not exist (data lives in top.sub)"
    );
    assert!(
        find_signal_by_path(hierarchy, "top.ready").is_none(),
        "top.ready does not exist (ready lives in top.sub.inner)"
    );
    assert!(
        find_signal_by_path(hierarchy, "top.sub.ready").is_none(),
        "top.sub.ready does not exist (ready lives one level deeper)"
    );

    // Wrong root returns None.
    assert!(
        find_signal_by_path(hierarchy, "wrong.clk").is_none(),
        "wrong.clk has no matching root scope"
    );

    // Malformed paths return None rather than panicking.
    assert!(find_signal_by_path(hierarchy, "").is_none(), "empty path");
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
    for var_ref in hierarchy.all_vars() {
        all_signals.push(hierarchy[var_ref].full_name(hierarchy));
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
        "Should find 'clk' at top level"
    );

    // Test non-recursive mode at submodule level
    let mut submodule1_signals: Vec<String> = Vec::new();

    if let Some(submodule1_ref) = find_scope_by_path(hierarchy, "top.submodule1") {
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
fn test_read_hierarchy_lib_recursive() {
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

    let lines = read_hierarchy(hierarchy, Some("top"), true, None)
        .expect("Should read hierarchy for 'top'");

    assert_eq!(
        lines,
        vec!["top", "  submodule1", "    inner", "  submodule2",]
    );
}

#[test]
fn test_read_hierarchy_lib_non_recursive_scope() {
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
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
0!\n\
0@\n\
0#";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    let lines = read_hierarchy(hierarchy, Some("top.submodule1"), false, None)
        .expect("Should read hierarchy for 'top.submodule1'");

    assert_eq!(lines, vec!["top.submodule1", "  inner"]);
}

#[test]
fn test_read_hierarchy_lib_with_limit() {
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

    let lines =
        read_hierarchy(hierarchy, Some("top"), true, Some(3)).expect("Should read hierarchy");

    assert_eq!(
        lines,
        vec![
            "top",
            "  submodule1",
            "    inner",
            "... truncated after 3 items",
        ]
    );
}

#[test]
fn test_read_hierarchy_lib_missing_scope() {
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

    let result = read_hierarchy(hierarchy, Some("top.missing"), true, None);
    assert_eq!(result.unwrap_err(), "Scope not found: top.missing");
}

#[test]
fn test_read_hierarchy_lib_skips_non_module_scopes() {
    let vcd_content = "\
$date 2024-01-01 $end\n\
$version Test VCD file $end\n\
$timescale 1ns $end\n\
$scope module top $end\n\
$scope begin genblk1 $end\n\
$scope module submodule1 $end\n\
$var wire 1 ! data1 $end\n\
$scope module inner $end\n\
$var wire 1 @ data2 $end\n\
$upscope $end\n\
$upscope $end\n\
$upscope $end\n\
$upscope $end\n\
$enddefinitions $end\n\
#0\n\
0!\n\
0@";

    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{}", vcd_content).expect("Failed to write VCD content");
    temp_file.flush().expect("Failed to flush");

    let waveform = wellen::simple::read(temp_file.path()).expect("Failed to read VCD file");
    let hierarchy = waveform.hierarchy();

    let lines = read_hierarchy(hierarchy, Some("top"), true, None)
        .expect("Should read hierarchy for 'top'");

    assert_eq!(lines, vec!["top", "  submodule1", "    inner",]);
}
