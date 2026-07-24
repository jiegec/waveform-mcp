//! Formatting tests

use waveform_mcp::format_signal_value;
use waveform_mcp::format_time;
use wellen::SignalValueRef;

#[test]
fn test_format_signal_value() {
    // Test Event
    let event = SignalValueRef::Event;
    assert_eq!(format_signal_value(event), "Event");

    // Test Binary (2-bit)
    let binary_data: [u8; 1] = [2];
    let binary = SignalValueRef::bit_vec(wellen::States::Two, 2, &binary_data);
    assert_eq!(format_signal_value(binary), "2'b10");

    // Test Binary (1-bit)
    let binary_data1: [u8; 1] = [1];
    let binary1 = SignalValueRef::bit_vec(wellen::States::Two, 1, &binary_data1);
    assert_eq!(format_signal_value(binary1), "1'b1");

    // Test Binary (16-bit - should use hex)
    let binary_data16: [u8; 2] = [0x55, 0x55];
    let binary16 = SignalValueRef::bit_vec(wellen::States::Two, 16, &binary_data16);
    assert_eq!(format_signal_value(binary16), "16'h5555");

    // Test Binary (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xd];
    let binary8 = SignalValueRef::bit_vec(wellen::States::Two, 8, &binary_data8);
    assert_eq!(format_signal_value(binary8), "8'h0d");

    // Test Binary (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xcd];
    let binary8 = SignalValueRef::bit_vec(wellen::States::Two, 8, &binary_data8);
    assert_eq!(format_signal_value(binary8), "8'hcd");

    // Test Binary (9-bit - should use hex)
    let binary_data9: [u8; 2] = [0x1, 0xcd];
    let binary9 = SignalValueRef::bit_vec(wellen::States::Two, 9, &binary_data9);
    assert_eq!(format_signal_value(binary9), "9'h1cd");

    // Test FourValue
    let four_data: [u8; 1] = [0];
    let four = SignalValueRef::bit_vec(wellen::States::Four, 1, &four_data);
    assert_eq!(format_signal_value(four), "0");

    // Test NineValue
    let nine_data: [u8; 1] = [0];
    let nine = SignalValueRef::bit_vec(wellen::States::Nine, 1, &nine_data);
    assert_eq!(format_signal_value(nine), "0");

    // Test String
    let string = SignalValueRef::String("test");
    assert_eq!(format_signal_value(string), "test");

    // Test Real
    let real = SignalValueRef::Real(3.15);
    assert_eq!(format_signal_value(real), "3.15");
}

#[test]
fn test_format_time() {
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
