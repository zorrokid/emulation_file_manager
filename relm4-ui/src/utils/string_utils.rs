/// Format bytes into human-readable string using binary units (e.g., "2.3 MiB")
/// 
/// Uses IEC binary prefixes (1024 base):
/// - 1 KiB = 1,024 bytes
/// - 1 MiB = 1,048,576 bytes
/// - 1 GiB = 1,073,741,824 bytes
/// 
/// # Arguments
///
/// * `bytes` - The number of bytes to format.
/// 
/// # Returns
/// 
/// A `String` representing the formatted byte size.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    const BASE: f64 = 1024.0;

    if bytes == 0 {
        return "0 B".to_string();
    }

    if bytes < 1024 {
        return format!("{} B", bytes);
    }

    let bytes_f = bytes as f64;
    let exponent = (bytes_f.ln() / BASE.ln()).floor() as usize;
    let exponent = exponent.min(UNITS.len() - 1);

    let size = bytes_f / BASE.powi(exponent as i32);

    format!("{:.1} {}", size, UNITS[exponent])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes_zero() {
        assert_eq!(format_bytes(0), "0 B");
    }

    #[test]
    fn test_format_bytes_single_byte() {
        assert_eq!(format_bytes(1), "1 B");
    }

    #[test]
    fn test_format_bytes_multiple_bytes() {
        assert_eq!(format_bytes(500), "500 B");
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(1023), "1023 B");
    }

    #[test]
    fn test_format_bytes_kibibytes() {
        assert_eq!(format_bytes(1_024), "1.0 KiB");
        assert_eq!(format_bytes(1_536), "1.5 KiB"); // 1.5 * 1024
        assert_eq!(format_bytes(10_240), "10.0 KiB");
        assert_eq!(format_bytes(51_200), "50.0 KiB");
        assert_eq!(format_bytes(1_048_575), "1024.0 KiB"); // Just under 1 MiB
    }

    #[test]
    fn test_format_bytes_mebibytes() {
        assert_eq!(format_bytes(1_048_576), "1.0 MiB"); // 1024^2
        assert_eq!(format_bytes(2_621_440), "2.5 MiB"); // 2.5 * 1024^2
        assert_eq!(format_bytes(16_777_216), "16.0 MiB");
        assert_eq!(format_bytes(1_073_741_823), "1024.0 MiB"); // Just under 1 GiB
    }

    #[test]
    fn test_format_bytes_gibibytes() {
        assert_eq!(format_bytes(1_073_741_824), "1.0 GiB"); // 1024^3
        assert_eq!(format_bytes(5_905_580_032), "5.5 GiB");
        assert_eq!(format_bytes(42_949_672_960), "40.0 GiB");
    }

    #[test]
    fn test_format_bytes_tebibytes() {
        assert_eq!(format_bytes(1_099_511_627_776), "1.0 TiB"); // 1024^4
        assert_eq!(format_bytes(3_848_290_697_216), "3.5 TiB");
        assert_eq!(format_bytes(10_995_116_277_760), "10.0 TiB");
    }

    #[test]
    fn test_format_bytes_large_tebibytes() {
        // Even larger values should cap at TiB
        assert_eq!(format_bytes(109_951_162_777_600), "100.0 TiB");
        assert_eq!(format_bytes(u64::MAX), "16777216.0 TiB");
    }

    #[test]
    fn test_format_bytes_edge_cases() {
        // Just under each threshold
        assert_eq!(format_bytes(1023), "1023 B");
        assert_eq!(format_bytes(1_048_575), "1024.0 KiB");
        assert_eq!(format_bytes(1_073_741_823), "1024.0 MiB");
        assert_eq!(format_bytes(1_099_511_627_775), "1024.0 GiB");
    }

    #[test]
    fn test_format_bytes_rounding() {
        // Test decimal rounding with binary units
        assert_eq!(format_bytes(1_228), "1.2 KiB"); // ~1.199 KiB
        assert_eq!(format_bytes(1_638), "1.6 KiB"); // ~1.599 KiB
        assert_eq!(format_bytes(1_258_291), "1.2 MiB"); // ~1.2 MiB
        assert_eq!(format_bytes(10_380_902), "9.9 MiB"); // ~9.9 MiB
    }
}
