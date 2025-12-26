//! Formatting utilities for time and signal values.

use wellen;

/// Format a time value with its timescale into a human-readable string.
///
/// # Arguments
/// * `time_value` - The raw time value from waveform
/// * `timescale` - Optional timescale information for proper formatting
///
/// # Examples
/// ```
/// use waveform_mcp::formatting::format_time;
///
/// let timescale = wellen::Timescale {
///     factor: 1,
///     unit: wellen::TimescaleUnit::NanoSeconds,
/// };
/// assert_eq!(format_time(10, Some(&timescale)), "10ns");
/// ```
pub fn format_time(time_value: u64, timescale: Option<&wellen::Timescale>) -> String {
    match timescale {
        Some(ts) => {
            let unit = match ts.unit {
                wellen::TimescaleUnit::ZeptoSeconds => "zs",
                wellen::TimescaleUnit::AttoSeconds => "as",
                wellen::TimescaleUnit::FemtoSeconds => "fs",
                wellen::TimescaleUnit::PicoSeconds => "ps",
                wellen::TimescaleUnit::NanoSeconds => "ns",
                wellen::TimescaleUnit::MicroSeconds => "us",
                wellen::TimescaleUnit::MilliSeconds => "ms",
                wellen::TimescaleUnit::Seconds => "s",
                wellen::TimescaleUnit::Unknown => "unknown",
            };
            format!("{}{}", time_value * ts.factor as u64, unit)
        }
        None => format!("{} (unknown timescale)", time_value),
    }
}

/// Format a signal value into a human-readable string.
///
/// # Arguments
/// * `signal_value` - The signal value to format
///
/// # Returns
/// A string representation of signal value.
///
/// Format mimics Verilog representation:
/// - Short signals (<= 4 bits): binary format like `3'b101`
/// - Longer signals: hex format like `16'h1a2b`
pub fn format_signal_value(signal_value: wellen::SignalValue) -> String {
    match signal_value {
        wellen::SignalValue::Event => "Event".to_string(),
        wellen::SignalValue::Binary(data, bits) => format_binary_verilog(data, bits),
        wellen::SignalValue::FourValue(data, _) => format!("{:?}", data),
        wellen::SignalValue::NineValue(data, _) => format!("{:?}", data),
        wellen::SignalValue::String(s) => s.to_string(),
        wellen::SignalValue::Real(r) => format!("{}", r),
    }
}

/// Format a binary value (Vec<u8>) in Verilog style.
fn format_binary_verilog(data: &[u8], bits: u32) -> String {
    // For short signals (<= 4 bits), use binary format
    if bits <= 4 {
        let mut bin_str = String::new();
        for bit_idx in (0..bits).rev() {
            let byte_idx = 0;
            let bit_in_byte = bit_idx;
            if (data[byte_idx] >> bit_in_byte) & 1 == 1 {
                bin_str.push('1');
            } else {
                bin_str.push('0');
            }
        }
        return format!("{}'b{}", bits, bin_str);
    }

    // For longer signals, use hex format
    // Convert bytes to hex representation
    let hex_chars = (bits as usize).div_ceil(4); // Number of hex characters needed
    let full_hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
    if hex_chars % 2 == 1 {
        format!("{}'h{}", bits, &full_hex[1..])
    } else {
        format!("{}'h{}", bits, full_hex)
    }
}
