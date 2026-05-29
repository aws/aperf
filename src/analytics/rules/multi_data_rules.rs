use crate::analytics::{
    AnalyticalFinding, DataFindings, MultiDataAnalyticalRule, MultiDataAnalyze, Score,
};
use crate::data::common::data_formats::ProcessedData;
use crate::data::common::processed_data_accessor::ProcessedDataAccessor;
use std::collections::HashMap;

const SYSTEMINFO: &str = "systeminfo";
const KERNEL_CONFIG: &str = "kernel_config";

// TODO: implement key-value data
const PREEMPT_LAZY_MESSAGE: &str = "Linux 7.0 changed the default preemption model to PREEMPT_LAZY. If you observe unexpected performance differences, consider setting the preemption model to PREEMPT_NONE via kernel boot parameter (preempt=none) or kernel config (CONFIG_PREEMPT_NONE=y), or implementing rseq (restartable sequences) in the application's hot functions. See: https://lore.kernel.org/all/20260403191942.21410-1-dipiets@amazon.it/T/#t";

/// Fires when kernel >= 7 and CONFIG_PREEMPT_LAZY=y.
/// Based on: https://lore.kernel.org/all/20260403191942.21410-1-dipiets@amazon.it/T/#t
pub struct PreemptLazyDetectedRule;

impl MultiDataAnalyze for PreemptLazyDetectedRule {
    fn analyze(
        &self,
        findings: &mut HashMap<String, DataFindings>,
        all_processed_data: &HashMap<String, &ProcessedData>,
        processed_data_accessor: &mut ProcessedDataAccessor,
    ) {
        let systeminfo = match all_processed_data.get(SYSTEMINFO) {
            Some(d) => *d,
            None => return,
        };
        let kconfig = match all_processed_data.get(KERNEL_CONFIG) {
            Some(d) => *d,
            None => return,
        };

        for run_name in systeminfo.runs.keys() {
            let version = match processed_data_accessor.key_value_value_by_key(
                systeminfo,
                run_name,
                "Kernel Version",
            ) {
                Some(v) => v,
                None => continue,
            };

            let major = version
                .split('.')
                .next()
                .and_then(|m| m.parse::<u32>().ok())
                .unwrap_or(0);
            if major < 7 {
                continue;
            }

            let is_lazy = processed_data_accessor
                .key_value_value_by_key(kconfig, run_name, "CONFIG_PREEMPT_LAZY")
                .map(|v| v == "y")
                .unwrap_or(false);
            if !is_lazy {
                continue;
            }

            let desc = format!(
                "Kernel {} in {} is running with PREEMPT_LAZY preemption model.",
                version, run_name
            );
            findings
                .entry(KERNEL_CONFIG.to_string())
                .or_insert(DataFindings::default())
                .insert_finding(
                    run_name,
                    "CONFIG_PREEMPT_LAZY",
                    AnalyticalFinding::new(
                        "PREEMPT_LAZY Detected".to_string(),
                        Score::Neutral.as_f64(),
                        desc,
                        PREEMPT_LAZY_MESSAGE.to_string(),
                    ),
                );
        }
    }
}

pub fn get_multi_data_rules() -> Vec<MultiDataAnalyticalRule> {
    vec![MultiDataAnalyticalRule::PreemptLazyDetectedRule(
        PreemptLazyDetectedRule,
    )]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::common::data_formats::{AperfData, DataFormat, KeyValueData, KeyValueGroup};

    fn kv_data(runs: Vec<(&str, Vec<(&str, &str)>)>) -> ProcessedData {
        let mut pd = ProcessedData::new("test".to_string());
        pd.data_format = DataFormat::KeyValue;
        for (run, pairs) in runs {
            let mut kv = KeyValueData::default();
            let mut group = KeyValueGroup::default();
            for (k, v) in pairs {
                group.key_values.insert(k.to_string(), v.to_string());
            }
            kv.key_value_groups.insert(String::new(), group);
            pd.runs.insert(run.to_string(), AperfData::KeyValue(kv));
        }
        pd
    }

    fn run_rule(sysinfo: &ProcessedData, kconfig: &ProcessedData) -> HashMap<String, DataFindings> {
        let mut all: HashMap<String, &ProcessedData> = HashMap::new();
        all.insert(SYSTEMINFO.to_string(), sysinfo);
        all.insert(KERNEL_CONFIG.to_string(), kconfig);
        let mut findings = HashMap::new();
        let mut acc = ProcessedDataAccessor::new();
        PreemptLazyDetectedRule.analyze(&mut findings, &all, &mut acc);
        findings
    }

    #[test]
    fn triggers_k7_preempt_lazy() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "7.0.1")])]);
        let kc = kv_data(vec![("r1", vec![("CONFIG_PREEMPT_LAZY", "y")])]);
        assert!(run_rule(&si, &kc).contains_key(KERNEL_CONFIG));
    }

    #[test]
    fn skips_k6() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "6.12.0")])]);
        let kc = kv_data(vec![("r1", vec![("CONFIG_PREEMPT_LAZY", "y")])]);
        assert!(!run_rule(&si, &kc).contains_key(KERNEL_CONFIG));
    }

    #[test]
    fn skips_no_preempt_lazy() {
        let si = kv_data(vec![("r1", vec![("Kernel Version", "7.0.1")])]);
        let kc = kv_data(vec![("r1", vec![])]);
        assert!(!run_rule(&si, &kc).contains_key(KERNEL_CONFIG));
    }
}
