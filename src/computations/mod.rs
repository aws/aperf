use serde::{Deserialize, Serialize, Serializer};
use strum_macros::Display;

/// Different statistics of the values contained in a Series
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Statistics {
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub avg: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub std: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub min: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub max: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p50: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p90: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p99: f64,
    #[serde(serialize_with = "serialize_f64_fixed2")]
    pub p99_9: f64,
}

impl Statistics {
    pub fn from_values(values: &Vec<f64>) -> Self {
        let n = values.len();
        if n == 0 {
            return Self::default();
        }

        let mut sum = 0.0;
        let mut min = values[0];
        let mut max = values[0];
        for &value in values {
            sum += value;
            min = min.min(value);
            max = max.max(value);
        }
        let avg = sum / n as f64;

        let mut sum_sq_diff = 0.0;
        for &value in values {
            let diff = value - avg;
            sum_sq_diff += diff * diff;
        }
        let std = (sum_sq_diff / n as f64).sqrt();

        let mut sorted_values = values.clone();
        sorted_values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50 = sorted_values[(0.5 * n as f64).floor() as usize];
        let p90 = sorted_values[(0.9 * n as f64).floor() as usize];
        let p99 = sorted_values[(0.99 * n as f64).floor() as usize];
        let p99_9 = sorted_values[(0.999 * n as f64).floor() as usize];

        Self {
            avg,
            std,
            min,
            max,
            p50,
            p90,
            p99,
            p99_9,
        }
    }
}

#[derive(Display, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum Stat {
    Average,
    Std,
    Min,
    Max,
    P50,
    P90,
    P99,
    P99_9,
}

impl Stat {
    pub fn get_stat(&self, statistics: &Statistics) -> f64 {
        match self {
            Stat::Average => statistics.avg,
            Stat::Std => statistics.std,
            Stat::Min => statistics.min,
            Stat::Max => statistics.max,
            Stat::P50 => statistics.p50,
            Stat::P90 => statistics.p90,
            Stat::P99 => statistics.p99,
            Stat::P99_9 => statistics.p99_9,
        }
    }
}

#[derive(Display, Clone, Copy)]
pub enum Comparator {
    #[strum(serialize = "equal to")]
    Equal,
    #[strum(serialize = "not equal to")]
    NotEqual,
    #[strum(serialize = "greater than")]
    Greater,
    #[strum(serialize = "greater than or equal to")]
    GreaterEqual,
    #[strum(serialize = "less than")]
    Less,
    #[strum(serialize = "less than or euqal to")]
    LessEqual,
}

impl Comparator {
    pub fn compare<T: PartialOrd>(&self, left: T, right: T) -> bool {
        match self {
            Comparator::Equal => left == right,
            Comparator::NotEqual => left != right,
            Comparator::Greater => left > right,
            Comparator::GreaterEqual => left >= right,
            Comparator::Less => left < right,
            Comparator::LessEqual => left <= right,
        }
    }
}

// custom Serde serializations
// allow f64 values to be truncated to 2 decimal places (to save spaces)
pub fn f64_to_fixed_2(value: f64) -> f64 {
    f64::trunc(value * 100.0) / 100.0
}

// custom serializing function for Vec<f64> to truncate all elements to 2 decimal places
pub fn serialize_f64_vec_fixed2<S>(values: &[f64], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    values
        .iter()
        .map(|&v| f64_to_fixed_2(v))
        .collect::<Vec<_>>()
        .serialize(serializer)
}
// custom serializing function for f64 to truncate all elements to 2 decimal places
pub fn serialize_f64_fixed2<S>(value: &f64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_f64(f64_to_fixed_2(*value))
}

pub fn ratio_to_percentage_string(ratio: f64) -> String {
    let percentage = f64_to_fixed_2(ratio * 100.0);
    format!("{:.2}%", percentage)
}

pub fn ratio_to_percentage_delta_string(ratio: f64) -> String {
    let abs_ratio_delta = f64_to_fixed_2((ratio - 1.0).abs());
    let relation_string = if ratio > 0.0 {
        "greater than"
    } else {
        "less than"
    };
    format!("{}% {}", abs_ratio_delta, relation_string)
}
