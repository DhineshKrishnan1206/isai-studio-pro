/// Formats seconds as `m:ss.t`, e.g. `1:15.2`.
pub fn format_time(secs: f64) -> String {
    // Round to tenths first so 59.97 shows as 1:00.0, not 0:60.0.
    let tenths = (secs.max(0.0) * 10.0).round() as u64;
    let (minutes, rest) = (tenths / 600, tenths % 600);
    format!("{minutes}:{:02}.{}", rest / 10, rest % 10)
}

#[cfg(test)]
mod tests {
    #[test]
    fn formats_minutes_and_tenths() {
        assert_eq!(super::format_time(0.0), "0:00.0");
        assert_eq!(super::format_time(75.24), "1:15.2");
        assert_eq!(super::format_time(59.97), "1:00.0");
        assert_eq!(super::format_time(-3.0), "0:00.0");
    }
}
