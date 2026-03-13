use std::collections::HashMap;

/// For raw time-series data that are not available with a format in system pseudo files,
/// use the builder to format it, so that they can be collected and parsed with a unified logic.
pub struct TimeSeriesDataBuilder {
    raw_data: String,
}
/// The format is as following:
///
/// component_name_1:
/// metric_name_1 metric_value_1
/// metric_name_2 metric_value_2
/// ...
/// component_name_2:
/// metric_name_1 metric_value_1
/// metric_name_2 metric_value_2
/// ...
///
/// which can be naturally mapped to most time-series data.
impl TimeSeriesDataBuilder {
    pub fn new() -> Self {
        TimeSeriesDataBuilder {
            raw_data: String::new(),
        }
    }

    pub fn add_component_line(&mut self, component_name: &String) {
        self.raw_data.push_str(component_name);
        self.raw_data.push_str(":\n");
    }

    pub fn add_metric_line(&mut self, metric_name: &String, metric_value: &String) {
        let metric_line = format!("{} {}", metric_name, metric_value);
        self.raw_data.push_str(metric_line.trim());
        self.raw_data.push('\n');
    }

    pub fn get_data(self) -> String {
        self.raw_data
    }
}

/// Represent a snapshot of time-series data parsed from one piece of raw data at a time_diff.
/// The format is Map<metric name, Map<component (which usually are the series name), metric value>>.
pub type TimeSeriesDataSnapshot = HashMap<String, HashMap<String, f64>>;

/// Parse a snapshot of raw data created by TimeSeriesDataBuilder
pub fn parse_common_raw_time_series_data(raw_data: &String) -> TimeSeriesDataSnapshot {
    let mut parsed_data: TimeSeriesDataSnapshot = HashMap::new();
    let mut cur_component_name = String::new();

    for line in raw_data.lines() {
        if line.chars().last() == Some(':') {
            cur_component_name = line[..(line.len() - 1)].to_string();
        } else {
            let mut parts = line.split_whitespace();
            if let (Some(metric_name), Some(value_str)) = (parts.next(), parts.next()) {
                let metric_value = match value_str.parse::<f64>() {
                    Ok(metric_value) => metric_value,
                    Err(_) => continue,
                };
                parsed_data
                    .entry(metric_name.to_string())
                    .or_insert(HashMap::new())
                    .insert(cur_component_name.clone(), metric_value);
            }
        }
    }

    parsed_data
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // TimeSeriesDataBuilder tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_builder_empty() {
        let builder = TimeSeriesDataBuilder::new();
        assert_eq!(builder.get_data(), "");
    }

    #[test]
    fn test_builder_single_component_single_metric() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"eth0".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"1234".to_string());
        assert_eq!(builder.get_data(), "eth0:\ntx_bytes 1234\n");
    }

    #[test]
    fn test_builder_multiple_components() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"eth0".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"100".to_string());
        builder.add_metric_line(&"rx_bytes".to_string(), &"200".to_string());
        builder.add_component_line(&"eth1".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"300".to_string());

        let data = builder.get_data();
        assert_eq!(
            data,
            "eth0:\ntx_bytes 100\nrx_bytes 200\neth1:\ntx_bytes 300\n"
        );
    }

    #[test]
    fn test_builder_trims_outer_whitespace() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"c".to_string());
        // trim() only strips leading/trailing whitespace from the combined "name value" line.
        // Internal whitespace (from padded values like "  42  ") is preserved between name and value.
        builder.add_metric_line(&"counter".to_string(), &"42".to_string());
        let data = builder.get_data();
        assert_eq!(data, "c:\ncounter 42\n");
    }

    #[test]
    fn test_builder_metric_value_with_padding_preserves_internal_space() {
        // When /sys files return values with whitespace like "  42  ", the builder
        // preserves the internal spacing. The parser handles this via split_whitespace.
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"c".to_string());
        builder.add_metric_line(&"counter".to_string(), &"  42  ".to_string());
        let data = builder.get_data();
        // trim() strips leading/trailing but "counter   42" has internal spaces
        assert!(data.contains("counter"), "got: {}", data);

        // Verify the parser still handles it correctly
        let parsed = parse_common_raw_time_series_data(&data);
        assert_eq!(parsed["counter"]["c"], 42.0);
    }

    // -----------------------------------------------------------------------
    // Builder → Parser round-trip tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_roundtrip_single_component() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"eth0".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"100".to_string());
        builder.add_metric_line(&"rx_bytes".to_string(), &"200".to_string());

        let parsed = parse_common_raw_time_series_data(&builder.get_data());

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed["tx_bytes"]["eth0"], 100.0);
        assert_eq!(parsed["rx_bytes"]["eth0"], 200.0);
    }

    #[test]
    fn test_roundtrip_multiple_components_same_metrics() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"eth0".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"100".to_string());
        builder.add_metric_line(&"rx_bytes".to_string(), &"200".to_string());
        builder.add_component_line(&"eth1".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"300".to_string());
        builder.add_metric_line(&"rx_bytes".to_string(), &"400".to_string());

        let parsed = parse_common_raw_time_series_data(&builder.get_data());

        assert_eq!(parsed.len(), 2); // 2 metrics
        assert_eq!(parsed["tx_bytes"].len(), 2); // 2 components
        assert_eq!(parsed["tx_bytes"]["eth0"], 100.0);
        assert_eq!(parsed["tx_bytes"]["eth1"], 300.0);
        assert_eq!(parsed["rx_bytes"]["eth0"], 200.0);
        assert_eq!(parsed["rx_bytes"]["eth1"], 400.0);
    }

    #[test]
    fn test_roundtrip_components_with_different_metrics() {
        let mut builder = TimeSeriesDataBuilder::new();
        builder.add_component_line(&"efa0".to_string());
        builder.add_metric_line(&"tx_bytes".to_string(), &"100".to_string());
        builder.add_component_line(&"efa1".to_string());
        builder.add_metric_line(&"rdma_write_bytes".to_string(), &"500".to_string());

        let parsed = parse_common_raw_time_series_data(&builder.get_data());

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed["tx_bytes"].len(), 1);
        assert_eq!(parsed["tx_bytes"]["efa0"], 100.0);
        assert_eq!(parsed["rdma_write_bytes"].len(), 1);
        assert_eq!(parsed["rdma_write_bytes"]["efa1"], 500.0);
    }

    // -----------------------------------------------------------------------
    // Parser edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn test_parse_empty_string() {
        let parsed = parse_common_raw_time_series_data(&String::new());
        assert!(parsed.is_empty());
    }

    #[test]
    fn test_parse_non_numeric_value_skipped() {
        let raw = "comp:\nmetric_a 100\nmetric_b not_a_number\nmetric_c 300\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);

        assert_eq!(parsed.len(), 2);
        assert!(parsed.contains_key("metric_a"));
        assert!(!parsed.contains_key("metric_b"));
        assert!(parsed.contains_key("metric_c"));
    }

    #[test]
    fn test_parse_floating_point_values() {
        let raw = "comp:\nmetric 1.23\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert!((parsed["metric"]["comp"] - 1.23).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_negative_values() {
        let raw = "comp:\nmetric -42\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert_eq!(parsed["metric"]["comp"], -42.0);
    }

    #[test]
    fn test_parse_metric_before_any_component() {
        // Metric lines before any component line get associated with empty-string component.
        // This is an edge case — callers should always emit a component line first.
        let raw = "orphan_metric 999\ncomp:\nreal_metric 100\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);

        assert_eq!(parsed["orphan_metric"][""], 999.0);
        assert_eq!(parsed["real_metric"]["comp"], 100.0);
    }

    #[test]
    fn test_parse_blank_lines_ignored() {
        let raw = "comp:\n\nmetric 100\n\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed["metric"]["comp"], 100.0);
    }

    #[test]
    fn test_parse_component_with_no_metrics() {
        let raw = "empty_comp:\nnext_comp:\nmetric 42\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        // empty_comp has no metrics, so it shouldn't appear in the output
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed["metric"]["next_comp"], 42.0);
    }

    #[test]
    fn test_parse_same_metric_same_component_last_wins() {
        // If the same metric appears twice under the same component, the last value wins
        let raw = "comp:\nmetric 100\nmetric 200\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert_eq!(parsed["metric"]["comp"], 200.0);
    }

    #[test]
    fn test_parse_component_name_with_slash() {
        // EFA uses component names like "efa0/1" for port-specific counters
        let raw = "efa0/1:\ntx_bytes 500\n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert_eq!(parsed["tx_bytes"]["efa0/1"], 500.0);
    }

    #[test]
    fn test_parse_extra_whitespace_in_metric_line() {
        let raw = "comp:\n  metric   100  \n".to_string();
        let parsed = parse_common_raw_time_series_data(&raw);
        assert_eq!(parsed["metric"]["comp"], 100.0);
    }

    #[test]
    fn test_parse_large_values() {
        let raw = "comp:\nbig 18446744073709551615\n".to_string(); // u64::MAX
        let parsed = parse_common_raw_time_series_data(&raw);
        assert!(parsed["big"]["comp"] > 1e18);
    }

    #[test]
    fn test_parse_many_components_many_metrics() {
        let mut builder = TimeSeriesDataBuilder::new();
        for c in 0..50 {
            builder.add_component_line(&format!("comp_{}", c));
            for m in 0..20 {
                builder.add_metric_line(&format!("metric_{}", m), &format!("{}", c * 1000 + m));
            }
        }
        let parsed = parse_common_raw_time_series_data(&builder.get_data());

        assert_eq!(parsed.len(), 20); // 20 distinct metrics
        for m in 0..20 {
            let metric = &parsed[&format!("metric_{}", m)];
            assert_eq!(metric.len(), 50); // 50 components
            for c in 0..50 {
                assert_eq!(metric[&format!("comp_{}", c)], (c * 1000 + m) as f64);
            }
        }
    }
}
