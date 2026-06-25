//! Formatting tests

use waveform_mcp::format_signal_value;
use waveform_mcp::format_time;

#[test]
fn test_format_signal_value() {
    use wellen::{SignalValueRef, States};

    // Test Event
    let event = SignalValueRef::Event;
    assert_eq!(format_signal_value(event), "Event");

    // Test two-state bit vector (2-bit)
    let binary_data: [u8; 1] = [2];
    let binary = SignalValueRef::bit_vec(States::Two, 2, &binary_data);
    assert_eq!(format_signal_value(binary), "2'b10");

    // Test two-state bit vector (1-bit)
    let binary_data1: [u8; 1] = [1];
    let binary1 = SignalValueRef::bit_vec(States::Two, 1, &binary_data1);
    assert_eq!(format_signal_value(binary1), "1'b1");

    // Test two-state bit vector (16-bit - should use hex)
    let binary_data16: [u8; 2] = [0x55, 0x55];
    let binary16 = SignalValueRef::bit_vec(States::Two, 16, &binary_data16);
    assert_eq!(format_signal_value(binary16), "16'h5555");

    // Test two-state bit vector (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xd];
    let binary8 = SignalValueRef::bit_vec(States::Two, 8, &binary_data8);
    assert_eq!(format_signal_value(binary8), "8'h0d");

    // Test two-state bit vector (8-bit - should use hex)
    let binary_data8: [u8; 1] = [0xcd];
    let binary8 = SignalValueRef::bit_vec(States::Two, 8, &binary_data8);
    assert_eq!(format_signal_value(binary8), "8'hcd");

    // Test two-state bit vector (9-bit - should use hex)
    let binary_data9: [u8; 2] = [0x1, 0xcd];
    let binary9 = SignalValueRef::bit_vec(States::Two, 9, &binary_data9);
    assert_eq!(format_signal_value(binary9), "9'h1cd");

    // Test four-state bit vector (rendered as binary since hex can't hold x/z)
    let four_data: [u8; 1] = [0];
    let four = SignalValueRef::bit_vec(States::Four, 1, &four_data);
    assert_eq!(format_signal_value(four), "1'b0");

    // Test nine-state bit vector (rendered as binary since hex can't hold x/z)
    let nine_data: [u8; 1] = [0];
    let nine = SignalValueRef::bit_vec(States::Nine, 1, &nine_data);
    assert_eq!(format_signal_value(nine), "1'b0");

    // Test String
    let string = SignalValueRef::String("test");
    assert_eq!(format_signal_value(string), "test");

    // Test Real
    let real = SignalValueRef::Real(3.15);
    assert_eq!(format_signal_value(real), "3.15");
}

// Four- and nine-state bit vectors can't be expressed in hex, so they are
// always rendered as a binary bit string (one char per bit, MSB first).
// wellen maps bit values to ASCII as: 0 1 x z h u w l -
#[test]
fn test_format_multistate_bit_vectors() {
    // Parsing an ASCII bit string infers the minimal States from its
    // characters, so the input doubles as the expected bit pattern.
    fn fmt(bits: &str) -> String {
        let value: wellen::SignalValue = bits.parse().expect("valid bit string");
        format_signal_value((&value).into())
    }

    // Four-state values carry x/z bits.
    assert_eq!(fmt("10xz"), "4'b10xz");

    // Wider than 4 bits, but the x/z bits keep it in binary form rather than
    // collapsing to hex.
    assert_eq!(fmt("1010xzxz"), "8'b1010xzxz");

    // Nine-state values use the extended set (high, unknown-weak, weak-low,
    // don't-care, weak).
    assert_eq!(fmt("hul-"), "4'bhul-");
    assert_eq!(fmt("0w1z"), "4'b0w1z");
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
