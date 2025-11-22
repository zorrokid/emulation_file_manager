/// Format bytes into human-readable string (e.g., "2.3 MB")
/// # Arguments
///
/// * `bytes` - The number of bytes to format.
/// # Returns
/// A `String` representing the formatted byte size.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let bytes_f = bytes as f64;
    let i = (bytes_f.log10() / 3.0).floor() as usize;
    let i = i.min(UNITS.len() - 1);

    let size = bytes_f / 1000_f64.powi(i as i32);

    if i == 0 {
        format!("{} B", bytes)
    } else {
        format!("{:.1} {}", size, UNITS[i])
    }
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
    }

    #[test]
    fn test_format_bytes_kilobytes() {
        assert_eq!(format_bytes(1_000), "1.0 KB");
        assert_eq!(format_bytes(1_500), "1.5 KB");
        assert_eq!(format_bytes(10_000), "10.0 KB");
        assert_eq!(format_bytes(50_234), "50.2 KB");
        assert_eq!(format_bytes(999_999), "1000.0 KB");
    }

    #[test]
    fn test_format_bytes_megabytes() {
        assert_eq!(format_bytes(1_000_000), "1.0 MB");
        assert_eq!(format_bytes(2_500_000), "2.5 MB");
        assert_eq!(format_bytes(15_750_000), "15.8 MB");
        assert_eq!(format_bytes(999_999_999), "1000.0 MB");
    }

    #[test]
    fn test_format_bytes_gigabytes() {
        assert_eq!(format_bytes(1_000_000_000), "1.0 GB");
        assert_eq!(format_bytes(5_500_000_000), "5.5 GB");
        assert_eq!(format_bytes(42_123_456_789), "42.1 GB");
    }

    #[test]
    fn test_format_bytes_terabytes() {
        assert_eq!(format_bytes(1_000_000_000_000), "1.0 TB");
        assert_eq!(format_bytes(3_500_000_000_000), "3.5 TB");
        assert_eq!(format_bytes(10_000_000_000_000), "10.0 TB");
    }

    #[test]
    fn test_format_bytes_large_terabytes() {
        // Even larger values should cap at TB
        assert_eq!(format_bytes(100_000_000_000_000), "100.0 TB");
        assert_eq!(format_bytes(u64::MAX), "18446744.1 TB");
    }

    #[test]
    fn test_format_bytes_edge_cases() {
        // Just under each threshold
        assert_eq!(format_bytes(999), "999 B");
        assert_eq!(format_bytes(999_999), "1000.0 KB");
        assert_eq!(format_bytes(999_999_999), "1000.0 MB");
        assert_eq!(format_bytes(999_999_999_999), "1000.0 GB");
    }

    #[test]
    fn test_format_bytes_rounding() {
        // Test decimal rounding
        assert_eq!(format_bytes(1_234), "1.2 KB");
        assert_eq!(format_bytes(1_567), "1.6 KB");
        assert_eq!(format_bytes(1_234_567), "1.2 MB");
        assert_eq!(format_bytes(9_876_543), "9.9 MB");
    }
}
