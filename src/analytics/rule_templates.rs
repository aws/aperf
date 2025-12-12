pub mod key_value_key_expected_rule;
pub mod key_value_key_run_comparison_rule;
pub mod time_series_data_point_threshold_rule;
pub mod time_series_stat_intra_run_comparison_rule;
pub mod time_series_stat_run_comparison_rule;
pub mod time_series_stat_threshold_rule;

/*
   Rule Template Naming Convention

   <Data Format>_<Data Used>_<Behavior>_rule.rs

   - Data Format: time_series, key_value
   - Data Used: stat, data_point, key
   - Behavior: intra_run_difference, run_comparison, expected, threshold
*/
