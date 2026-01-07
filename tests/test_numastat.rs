use aperf::data::numastat::{Numastat, NumastatRaw};
use aperf::data::{Data, ProcessData, TimeEnum};
use aperf::visualizer::ReportParams;
use chrono::Utc;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct ExpectedNumastatStats {
    pub stats: HashMap<String, HashMap<String, u64>>,
}

impl ExpectedNumastatStats {
    fn new() -> Self {
        Self {
            stats: HashMap::new(),
        }
    }

    fn set_node_stat(&mut self, node: &str, metric: &str, value: u64) {
        self.stats
            .entry(node.to_string())
            .or_insert_with(HashMap::new)
            .insert(metric.to_string(), value);
    }
}

fn generate_numastat_raw_data(
    expected_per_sample_stats: &Vec<ExpectedNumastatStats>,
    interval_seconds: u64,
) -> Vec<Data> {
    let mut raw_data = Vec::new();

    for (sample_idx, expected_stats) in expected_per_sample_stats.iter().enumerate() {
        let mut numastat_data = String::new();

        for (node, node_stats) in &expected_stats.stats {
            numastat_data.push_str(&format!("{}:\n", node));
            for (metric, &value) in node_stats {
                numastat_data.push_str(&format!("{} {}\n", metric, value));
            }
        }

        let raw_numastat = NumastatRaw {
            time: TimeEnum::DateTime(
                Utc::now()
                    + chrono::Duration::seconds((sample_idx as i64) * (interval_seconds as i64)),
            ),
            data: numastat_data,
        };

        raw_data.push(Data::NumastatRaw(raw_numastat));
    }

    raw_data
}

#[test]
fn test_numastat_process_raw_data_single_node() {
    let mut expected_stats = vec![
        ExpectedNumastatStats::new(),
        ExpectedNumastatStats::new(),
        ExpectedNumastatStats::new(),
    ];

    // Sample 1
    expected_stats[0].set_node_stat("node0", "numa_hit", 1000);
    expected_stats[0].set_node_stat("node0", "numa_miss", 100);
    expected_stats[0].set_node_stat("node0", "numa_foreign", 50);

    // Sample 2
    expected_stats[1].set_node_stat("node0", "numa_hit", 1500);
    expected_stats[1].set_node_stat("node0", "numa_miss", 150);
    expected_stats[1].set_node_stat("node0", "numa_foreign", 75);

    // Sample 3
    expected_stats[2].set_node_stat("node0", "numa_hit", 2000);
    expected_stats[2].set_node_stat("node0", "numa_miss", 200);
    expected_stats[2].set_node_stat("node0", "numa_foreign", 100);

    let raw_data = generate_numastat_raw_data(&expected_stats, 1);
    let mut numastat = Numastat::new();
    let report_params = ReportParams::new();

    let result = numastat.process_raw_data(report_params, raw_data);
    assert!(result.is_ok());

    let aperf_data = result.unwrap();
    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = aperf_data {
        // Check that we have metrics for each stat type
        assert!(time_series_data.metrics.contains_key("numa_hit"));
        assert!(time_series_data.metrics.contains_key("numa_miss"));
        assert!(time_series_data.metrics.contains_key("numa_foreign"));

        // Check numa_hit metric has both node0 series and aggregate series
        let numa_hit_metric = &time_series_data.metrics["numa_hit"];
        assert_eq!(numa_hit_metric.series.len(), 2); // node0 + aggregate

        // Find node0 series
        let node0_series = numa_hit_metric
            .series
            .iter()
            .find(|s| s.series_name.as_ref() == Some(&"node0".to_string()))
            .expect("Should have node0 series");
        assert_eq!(node0_series.values.len(), 3);
        assert_eq!(node0_series.values[0], 0.0); // First sample, no previous value
        assert_eq!(node0_series.values[1], 500.0); // 1500 - 1000
        assert_eq!(node0_series.values[2], 500.0); // 2000 - 1500

        // Find aggregate series
        let aggregate_series = numa_hit_metric
            .series
            .iter()
            .find(|s| s.is_aggregate)
            .expect("Should have aggregate series");
        assert_eq!(aggregate_series.values.len(), 3);
        assert_eq!(aggregate_series.values[0], 0.0);
        assert_eq!(aggregate_series.values[1], 500.0); // Same as node0 since only one node
        assert_eq!(aggregate_series.values[2], 500.0);
    } else {
        panic!("Expected TimeSeries data format");
    }
}

#[test]
fn test_numastat_process_raw_data_multiple_nodes() {
    let mut expected_stats = vec![ExpectedNumastatStats::new(), ExpectedNumastatStats::new()];

    // Sample 1
    expected_stats[0].set_node_stat("node0", "numa_hit", 1000);
    expected_stats[0].set_node_stat("node0", "numa_miss", 100);
    expected_stats[0].set_node_stat("node1", "numa_hit", 800);
    expected_stats[0].set_node_stat("node1", "numa_miss", 80);

    // Sample 2
    expected_stats[1].set_node_stat("node0", "numa_hit", 1500);
    expected_stats[1].set_node_stat("node0", "numa_miss", 150);
    expected_stats[1].set_node_stat("node1", "numa_hit", 1200);
    expected_stats[1].set_node_stat("node1", "numa_miss", 120);

    let raw_data = generate_numastat_raw_data(&expected_stats, 1);
    let mut numastat = Numastat::new();
    let report_params = ReportParams::new();

    let result = numastat.process_raw_data(report_params, raw_data);
    assert!(result.is_ok());

    let aperf_data = result.unwrap();
    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = aperf_data {
        // Check that we have metrics for each stat type
        assert!(time_series_data.metrics.contains_key("numa_hit"));
        assert!(time_series_data.metrics.contains_key("numa_miss"));

        // Check numa_hit metric has node0, node1, and aggregate series
        let numa_hit_metric = &time_series_data.metrics["numa_hit"];
        assert_eq!(numa_hit_metric.series.len(), 3); // node0 + node1 + aggregate

        // Find node0 series
        let node0_series = numa_hit_metric
            .series
            .iter()
            .find(|s| s.series_name.as_ref() == Some(&"node0".to_string()))
            .expect("Should have node0 series");
        assert_eq!(node0_series.values.len(), 2);
        assert_eq!(node0_series.values[0], 0.0); // First sample
        assert_eq!(node0_series.values[1], 500.0); // 1500 - 1000

        // Find node1 series
        let node1_series = numa_hit_metric
            .series
            .iter()
            .find(|s| s.series_name.as_ref() == Some(&"node1".to_string()))
            .expect("Should have node1 series");
        assert_eq!(node1_series.values.len(), 2);
        assert_eq!(node1_series.values[0], 0.0); // First sample
        assert_eq!(node1_series.values[1], 400.0); // 1200 - 800

        // Find aggregate series (average of both nodes)
        let aggregate_series = numa_hit_metric
            .series
            .iter()
            .find(|s| s.is_aggregate)
            .expect("Should have aggregate series");
        assert_eq!(aggregate_series.values.len(), 2);
        assert_eq!(aggregate_series.values[0], 0.0); // First sample
        assert_eq!(aggregate_series.values[1], 450.0); // Average of (500 + 400) / 2
    } else {
        panic!("Expected TimeSeries data format");
    }
}

#[test]
fn test_numastat_empty_data() {
    let raw_data = vec![Data::NumastatRaw(NumastatRaw {
        time: TimeEnum::DateTime(Utc::now()),
        data: String::new(),
    })];

    let mut numastat = Numastat::new();
    let report_params = ReportParams::new();

    let result = numastat.process_raw_data(report_params, raw_data);
    assert!(result.is_ok());

    let aperf_data = result.unwrap();
    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = aperf_data {
        assert!(time_series_data.metrics.is_empty());
    } else {
        panic!("Expected TimeSeries data format");
    }
}

#[test]
fn test_numastat_malformed_data() {
    let raw_data = vec![Data::NumastatRaw(NumastatRaw {
        time: TimeEnum::DateTime(Utc::now()),
        data: "invalid data\nnode0:\ninvalid line\n".to_string(),
    })];

    let mut numastat = Numastat::new();
    let report_params = ReportParams::new();

    let result = numastat.process_raw_data(report_params, raw_data);
    assert!(result.is_ok());

    let aperf_data = result.unwrap();
    if let aperf::data::data_formats::AperfData::TimeSeries(time_series_data) = aperf_data {
        // Should handle malformed data gracefully
        assert!(time_series_data.metrics.is_empty());
    } else {
        panic!("Expected TimeSeries data format");
    }
}
