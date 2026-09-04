use anyhow::{bail, Error, Result};
use log::error;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::Instant;
#[cfg(target_os = "linux")]
use {anyhow::Context, log::debug};

use crate::data::common::data_formats::{Graph, GraphData};
use crate::data_collection::InitParams;

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
pub struct CpuInfo {
    pub part: Option<String>,
    pub vendor_id: Option<String>,
    pub model_name: Option<String>,
    pub family: Option<u32>,
    pub model: Option<u32>,
    pub stepping: Option<u32>,
}

#[cfg(target_os = "linux")]
impl CpuInfo {
    pub fn new() -> Result<Self> {
        let cpu_info_file = File::open("/proc/cpuinfo")?;
        let cpu_info_reader = BufReader::new(cpu_info_file);
        let mut part = None;
        let mut vendor_id = None;
        let mut model_name = None;
        let mut family = None;
        let mut model = None;
        let mut stepping = None;
        for line in cpu_info_reader.lines() {
            let info_line = line?;
            if info_line.is_empty() {
                break;
            }
            let key_value: Vec<&str> = info_line.split(':').collect();
            if key_value.len() < 2 {
                continue;
            }
            let key = key_value[0].trim().to_string();
            let value = key_value[1].trim().to_string();
            match key.as_str() {
                "CPU part" => part = Some(value),
                "vendor_id" => vendor_id = Some(value),
                "model name" => model_name = Some(value),
                "cpu family" => family = value.parse::<u32>().ok(),
                "model" => model = value.parse::<u32>().ok(),
                "stepping" => stepping = value.parse::<u32>().ok(),
                _ => continue,
            }
        }

        Ok(Self {
            part,
            vendor_id,
            model_name,
            family,
            model,
            stepping,
        })
    }

    /// Whether this is an ARM CPU. ARM64 kernels print the "CPU part" field
    /// unconditionally for every core (from the MIDR_EL1 register; see the
    /// kernel's arch/arm64/kernel/cpuinfo.c), while x86's /proc/cpuinfo has no
    /// such field (arch/x86/kernel/cpu/proc.c).
    pub fn is_arm(&self) -> bool {
        self.part.is_some()
    }

    pub fn is_graviton_5(&self) -> bool {
        self.part.as_ref().map_or(false, |part| part == "0xd84")
    }

    pub fn is_intel(&self) -> bool {
        self.vendor_id
            .as_ref()
            .map_or(false, |vendor_id| vendor_id == "GenuineIntel")
    }

    /// Skylake-SP (1st Gen Xeon Scalable, e.g. EC2 m5/c5's 8175M/8124M):
    /// CPUID 06_55h with stepping 0-4 per the intel/perfmon mapfile.csv
    /// (GenuineIntel-6-55-[01234] -> SKX). Cascade Lake shares the same
    /// family/model and differs only by stepping.
    pub fn is_intel_skylake(&self) -> bool {
        self.family == Some(6)
            && self.model == Some(0x55)
            && self.stepping.map_or(false, |s| s <= 4)
    }

    /// Cascade Lake (2nd Gen Xeon Scalable, e.g. EC2 m5n/m5zn's 8259CL/8252,
    /// and the larger m5/c5 sizes): CPUID 06_55h with stepping 5+ per the
    /// intel/perfmon mapfile.csv (GenuineIntel-6-55-[56789ABCDEF] -> CLX).
    pub fn is_intel_cascade_lake(&self) -> bool {
        self.family == Some(6)
            && self.model == Some(0x55)
            && self.stepping.map_or(false, |s| s >= 5)
    }

    /// Ice Lake-SP/-D (3rd Gen Xeon Scalable, e.g. EC2 m6i/i4i's 8375C):
    /// CPUID 06_6Ah (SP) or 06_6Ch (D) per the intel/perfmon mapfile.csv
    /// (GenuineIntel-6-6[AC] -> ICX).
    pub fn is_intel_icelake(&self) -> bool {
        self.family == Some(6) && (self.model == Some(0x6A) || self.model == Some(0x6C))
    }

    /// Sapphire Rapids (4th Gen Xeon Scalable, e.g. EC2 m7i/u7i's 8488C):
    /// CPUID 06_8Fh per the intel/perfmon mapfile.csv (GenuineIntel-6-8F -> SPR).
    pub fn is_intel_sapphire_rapids(&self) -> bool {
        self.family == Some(6) && self.model == Some(0x8F)
    }

    /// Emerald Rapids (5th Gen Xeon Scalable, e.g. EC2 i7i/i7ie): CPUID
    /// 06_CFh per the intel/perfmon mapfile.csv (GenuineIntel-6-CF -> EMR).
    pub fn is_intel_emerald_rapids(&self) -> bool {
        self.family == Some(6) && self.model == Some(0xCF)
    }

    /// Granite Rapids (Intel Xeon 6 P-core, e.g. EC2 m8i/c8i/r8i): CPUID
    /// 06_ADh (SP) or 06_AEh (AP) per the intel/perfmon mapfile.csv
    /// (GenuineIntel-6-A[DE] -> GNR).
    pub fn is_intel_granite_rapids(&self) -> bool {
        self.family == Some(6) && (self.model == Some(0xAD) || self.model == Some(0xAE))
    }

    pub fn is_amd(&self) -> bool {
        self.vendor_id
            .as_ref()
            .map_or(false, |vendor_id| vendor_id == "AuthenticAMD")
    }

    /// Genoa (Zen4, 4th Gen EPYC, e.g. EC2 m7a's 9R14): CPUID family 19h with
    /// the Zen4 model ranges (10h-1Fh, 60h-AFh) per the family/model -> ZEN4
    /// mapping in the kernel's arch/x86/kernel/cpu/amd.c.
    pub fn is_amd_genoa(&self) -> bool {
        self.family == Some(0x19)
            && self.model.map_or(false, |m| {
                (0x10..=0x1F).contains(&m) || (0x60..=0xAF).contains(&m)
            })
    }

    /// Milan (Zen3, 3rd Gen EPYC, e.g. EC2 m6a's 7R13): CPUID family 19h with
    /// the Zen3 model ranges (00h-0Fh, 20h-5Fh) per the family/model -> ZEN3
    /// mapping in the kernel's arch/x86/kernel/cpu/amd.c.
    pub fn is_amd_milan(&self) -> bool {
        self.family == Some(0x19)
            && self
                .model
                .map_or(false, |m| m <= 0x0F || (0x20..=0x5F).contains(&m))
    }

    /// Turin (Zen5, 5th Gen EPYC, e.g. EC2 m8a/c8a/r8a = 9R45, m8azn/x8aedz =
    /// 9R05): CPUID family 1Ah per the family -> ZEN5 mapping in the kernel's
    /// arch/x86/kernel/cpu/amd.c.
    pub fn is_amd_turin(&self) -> bool {
        self.family == Some(0x1A)
    }

    /// Naples (Zen1, 1st Gen EPYC, e.g. EC2 m5a/r5a/t3a = EPYC 7571): CPUID
    /// family 17h with the Zen1 model ranges (00h-2Fh, 50h-5Fh) per the
    /// family/model -> ZEN1 mapping in the kernel's arch/x86/kernel/cpu/amd.c.
    pub fn is_amd_naples(&self) -> bool {
        self.family == Some(0x17)
            && self
                .model
                .map_or(false, |m| m <= 0x2F || (0x50..=0x5F).contains(&m))
    }

    /// Rome (Zen2, 2nd Gen EPYC, e.g. EC2 c5a/c5ad = EPYC 7R32, plus
    /// g4ad/g5 hosts): CPUID family 17h in the Zen2 model ranges (30h-4Fh,
    /// 60h-AFh) per the family/model -> ZEN2 mapping in the kernel's
    /// arch/x86/kernel/cpu/amd.c — i.e. any family-17h model that is not in
    /// the Zen1 ranges.
    pub fn is_amd_rome(&self) -> bool {
        self.family == Some(0x17) && !self.is_amd_naples() && self.model.is_some()
    }
}

/// Parse a CPU list, a comma-separated list of single CPUs and inclusive
/// ranges, e.g. "0-1,3-5,7", into sorted and de-duplicated CPU IDs. This is
/// the format the kernel uses in the CPU masks under /sys/devices/system/cpu,
/// and the format accepted by the --pmu-cpus option.
pub fn parse_cpu_list(cpu_list: &str) -> Result<Vec<usize>> {
    let mut ids = Vec::new();
    let cpu_list = cpu_list.trim();
    if cpu_list.is_empty() {
        return Ok(ids);
    }

    fn parse_cpu_id(cpu_id: &str, cpu_list: &str) -> Result<usize> {
        match cpu_id.trim().parse() {
            Ok(cpu) => Ok(cpu),
            Err(e) => bail!("invalid CPU '{cpu_id}' in cpu list '{cpu_list}': {e}"),
        }
    }

    for part in cpu_list.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((low, high)) => {
                let low = parse_cpu_id(low, cpu_list)?;
                let high = parse_cpu_id(high, cpu_list)?;
                // Prevent bad user input from attempting to allocate too many memories.
                if high < low || high - low > 10000 {
                    bail!("invalid CPU range '{part}' in cpu list '{cpu_list}'");
                }
                ids.extend(low..=high);
            }
            None => ids.push(parse_cpu_id(part, cpu_list)?),
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// Helper function to retrieve the CPU ID of a per-CPU line in /proc/stat.
pub fn proc_stat_line_cpu_id(line: &str) -> Option<usize> {
    line.split_whitespace()
        .next()?
        .strip_prefix("cpu")?
        .parse()
        .ok()
}

/// Return the IDs of all online CPUs, from /sys/devices/system/cpu/online, or fall back to
/// per-CPU lines of /proc/stat when sysfs cannot be read.
#[cfg(target_os = "linux")]
pub fn get_online_cpu_ids() -> Result<Vec<usize>> {
    match fs::read_to_string("/sys/devices/system/cpu/online") {
        Ok(cpu_list) => parse_cpu_list(&cpu_list),
        Err(e) => {
            debug!("Failed to read the online CPUs from sysfs ({e}), falling back to /proc/stat.");
            let mut cpu_ids: Vec<usize> = read_virtual_file("/proc/stat")?
                .lines()
                .filter_map(proc_stat_line_cpu_id)
                .collect();
            cpu_ids.sort_unstable();
            Ok(cpu_ids)
        }
    }
}

const VIRTUAL_FILE_READ_CAPACITY: usize = 8192;

/// Read `file`'s entire contents from its current position, refilling a
/// fixed-size buffer until a true zero-byte read confirms EOF.
#[cfg(target_os = "linux")]
fn read_virtual_file_from_current_position(file: &mut File) -> Result<String> {
    let mut buf = [0u8; VIRTUAL_FILE_READ_CAPACITY];
    let mut out = String::new();
    loop {
        let len = file.read(&mut buf)?;
        if len == 0 {
            break;
        }
        out.push_str(std::str::from_utf8(&buf[..len])?);
    }
    Ok(out)
}

/// Read a pseudo-file (e.g. under /proc or /sys)
#[cfg(target_os = "linux")]
pub fn read_virtual_file<P: AsRef<Path>>(path: P) -> Result<String> {
    read_virtual_file_from_current_position(&mut File::open(&path)?)
}

/// Read a pseudo-file through a handle the caller keeps open across multiple
/// collection intervals.
#[cfg(target_os = "linux")]
pub fn read_open_virtual_file(file: &mut File) -> Result<String> {
    file.seek(SeekFrom::Start(0))?;
    read_virtual_file_from_current_position(file)
}

/// Check the current fd limit and raise it if the number of required fd is larger.
#[cfg(target_os = "linux")]
pub fn raise_fd_limit(num_required_fds: u64) -> Result<()> {
    let (cur_fd_limit, max_fd_limit) = rlimit::Resource::NOFILE
        .get()
        .context("Failed to read fd limit")?;
    if num_required_fds > cur_fd_limit {
        if num_required_fds >= max_fd_limit {
            bail!("The number of required fds ({num_required_fds}) is larger than the max fds ({max_fd_limit}).")
        }
        debug!("Increasing fd limit from {cur_fd_limit} to {num_required_fds}");
        rlimit::increase_nofile_limit(num_required_fds)
            .with_context(|| format!("Failed to increase the fd limit to {num_required_fds}"))?;
    }
    Ok(())
}

/// Compute how long a subprocess should run for, based on the current time
/// and the expected end time of the run.
pub fn get_sub_process_duration_seconds(init_params: &InitParams) -> u64 {
    let current_time = Instant::now();
    if current_time >= init_params.expected_end_time {
        return 0;
    }
    let duration = init_params.expected_end_time - current_time;
    // Round up the seconds
    return duration.as_secs() + 1;
}

/// Copy a graph file to the report data dir and update the GraphData with its info.
/// The destination file is prefixed with the (deduplicated, hence unique) `run_name` so that
/// graphs from different runs do not collide in the flat `data/js/` report directory.
pub fn copy_graph_and_update_graph_data(
    source_dir: &PathBuf,
    dest_dir: &PathBuf,
    filename: &str,
    run_name: &str,
    graph_group_name: &str,
    graph_key: &str,
    graph_name: String,
    graph_data: &mut GraphData,
) {
    let source_graph_path = source_dir.join(&filename);
    if !source_graph_path.exists() {
        return;
    }
    let run_prefix = format!("{run_name}-");
    let dest_filename = if filename.starts_with(&run_prefix) {
        filename.to_string()
    } else {
        format!("{run_prefix}{filename}")
    };
    let relative_graph_path = PathBuf::from("data").join("js").join(&dest_filename);
    let dest_graph_path = dest_dir.join(&relative_graph_path);

    if let Err(e) = std::fs::copy(&source_graph_path, &dest_graph_path) {
        error!("Failed to copy graph file: {e}");
    } else {
        graph_data
            .graph_groups
            .iter_mut()
            .find(|graph_group| graph_group.group_name == graph_group_name)
            .map(|graph_group| {
                graph_group.graphs.insert(
                    graph_key.to_string(),
                    Graph {
                        graph_name,
                        graph_path: relative_graph_path.to_string_lossy().into_owned(),
                        graph_size: None,
                    },
                );
            });
    }
}

/// Collects the paths of all files in a dir and returns a map from file names to file paths,
/// if the file system read was successful
pub fn collect_file_paths_in_dir(dir: &PathBuf) -> Result<HashMap<String, PathBuf>> {
    match fs::read_dir(dir) {
        Ok(hardware_counters_entries) => {
            let mut hardware_counter_file_paths: HashMap<String, PathBuf> = HashMap::new();
            for hardware_counter_entry in hardware_counters_entries {
                let hardware_counter_entry = match hardware_counter_entry {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };
                if let Ok(file_type) = hardware_counter_entry.file_type() {
                    if file_type.is_file() {
                        let port_counter_name = hardware_counter_entry
                            .file_name()
                            .to_string_lossy()
                            .into_owned();
                        hardware_counter_file_paths
                            .insert(port_counter_name, hardware_counter_entry.path());
                    }
                }
            }
            Ok(hardware_counter_file_paths)
        }
        Err(e) => Err(Error::from(e)),
    }
}

pub fn get_cpu_series_name(cpu: usize) -> String {
    format!("CPU{cpu}")
}

pub fn get_aggregate_series_name() -> String {
    "Aggregate".to_string()
}

/// Perform topological sort on a list of vectors and produce an ordered vector. Every input vector
/// represents the order between its contained values.
pub fn topological_sort(inputs: &Vec<&Vec<String>>) -> Result<Vec<String>> {
    let mut dependency_graph: HashMap<String, HashSet<String>> = HashMap::new();
    let mut in_degree_map: HashMap<String, u64> = HashMap::new();

    for &input in inputs {
        for (index, item) in input.iter().enumerate() {
            in_degree_map.insert(item.clone(), 0);
            if !dependency_graph.contains_key(item) {
                dependency_graph.insert(item.clone(), HashSet::new());
            }
            if index > 0 {
                let parent_dependencies = dependency_graph
                    .get_mut(input.get(index - 1).unwrap())
                    .unwrap();
                parent_dependencies.insert(item.clone());
            }
        }
    }
    for dependencies in dependency_graph.values() {
        for dependency in dependencies {
            *in_degree_map.get_mut(dependency).unwrap() += 1;
        }
    }

    let mut result: Vec<String> = Vec::new();

    let mut queue: VecDeque<String> = VecDeque::new();
    for (item, in_degree) in &in_degree_map {
        if *in_degree == 0 {
            queue.push_back(item.clone());
        }
    }

    while !queue.is_empty() {
        let cur_item = queue.pop_front().unwrap();
        result.push(cur_item.clone());
        for dependency in dependency_graph.get(&cur_item).unwrap() {
            let dependency_in_degree = in_degree_map.get_mut(dependency).unwrap();
            *dependency_in_degree -= 1;
            if *dependency_in_degree == 0 {
                queue.push_back(dependency.clone());
            }
        }
    }

    if result.len() != dependency_graph.len() {
        return Err(Error::msg(
            "Conflicting orders in inputs. Cannot perform topological sort.",
        ));
    }

    Ok(result)
}

/// Combine a list of input value ranges into one value range. The result value range's min is
/// the minimum of all value ranges' min, and its max is the maximum of all value ranges' max
pub fn combine_value_ranges(value_ranges: Vec<(u64, u64)>) -> (u64, u64) {
    if value_ranges.is_empty() {
        return (0, 0);
    }

    let mut min = value_ranges[0].0;
    let mut max = value_ranges[0].1;
    for value_range in value_ranges {
        min = min.min(value_range.0);
        max = max.max(value_range.1);
    }

    (min, max)
}

#[cfg(test)]
mod utils_test {
    use super::{combine_value_ranges, topological_sort};

    #[cfg(target_os = "linux")]
    #[test]
    fn test_get_online_cpu_ids() {
        use super::get_online_cpu_ids;
        let ids = get_online_cpu_ids().expect("should read /sys/devices/system/cpu/online");
        // At least CPU 0 is always online.
        assert!(!ids.is_empty(), "expected at least one online CPU");
        assert!(ids.contains(&0), "CPU 0 should be online");
        // Result is sorted and de-duplicated.
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(ids, sorted, "ids should be sorted and unique");
        // Compare against /proc/stat since it is an independent source of the same online CPU set.
        let mut proc_stat_ids: Vec<usize> = super::read_virtual_file("/proc/stat")
            .expect("should read /proc/stat")
            .lines()
            .filter_map(super::proc_stat_line_cpu_id)
            .collect();
        proc_stat_ids.sort_unstable();
        assert_eq!(ids, proc_stat_ids, "sysfs and /proc/stat should agree");
    }

    #[test]
    fn test_proc_stat_line_cpu_id() {
        use super::proc_stat_line_cpu_id;
        assert_eq!(proc_stat_line_cpu_id("cpu0 1 2 3"), Some(0));
        assert_eq!(proc_stat_line_cpu_id("cpu12 1 2 3"), Some(12));
        assert_eq!(proc_stat_line_cpu_id("cpu  1 2 3"), None);
        assert_eq!(proc_stat_line_cpu_id("intr 1 2 3"), None);
        assert_eq!(proc_stat_line_cpu_id("cpuX 1 2"), None);
        assert_eq!(proc_stat_line_cpu_id(""), None);
    }

    #[test]
    fn test_parse_cpu_list() {
        use super::parse_cpu_list;

        // Out-of-order single CPUs and ranges are expanded and sorted.
        let mut expected = vec![1, 5, 12, 16, 17, 18, 19];
        expected.extend(41..=55);
        assert_eq!(parse_cpu_list("12,5,41-55,16-19,1").unwrap(), expected);
        // Surrounding whitespace is ignored, and duplicates from repeated values
        // and overlapping ranges are collapsed.
        assert_eq!(parse_cpu_list(" 3 , 1-3 ,\t3, 2 ").unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_cpu_list("7").unwrap(), vec![7]);
        // Both ends of a range are included.
        assert_eq!(parse_cpu_list("4-4").unwrap(), vec![4]);
        // No CPU is requested.
        assert!(parse_cpu_list("").unwrap().is_empty());
        assert!(parse_cpu_list("  ").unwrap().is_empty());
        // A reversed range is a typo, and must not be read as no CPU at all.
        assert!(parse_cpu_list("5-3").is_err());
        // Values that are not CPU IDs are rejected.
        assert!(parse_cpu_list("1,a").is_err());
        assert!(parse_cpu_list("-1").is_err());
        assert!(parse_cpu_list("1-").is_err());
        // User input that might lead to memory exhaustion is rejected.
        assert!(parse_cpu_list("1-1234567890").is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_cpu_info() {
        use super::CpuInfo;

        let cpu_info = CpuInfo::new().expect("Should read and parse /proc/cpuinfo");

        assert!(
            cpu_info.is_arm() || cpu_info.is_intel() || cpu_info.is_amd(),
            "The CPU should be recognized as one of ARM, Intel, and AMD"
        );
    }

    #[test]
    fn test_topological_sort_fixed_result() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["a", "b", "d", "g", "i", "j"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["b", "c", "d", "f", "h", "i"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["a", "d", "e", "g", "h", "j", "k"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["c", "e", "f"].iter().map(|&s| s.to_string()).collect(),
            vec!["f", "g"].iter().map(|&s| s.to_string()).collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        if let Ok(output) = topological_sort(&inputs) {
            assert_eq!(
                output,
                vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]
            );
        } else {
            panic!("Conflicting orders in inputs");
        }
    }

    #[test]
    fn test_topological_sort_multiple_result() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["apple", "orange", "pear"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["watermelon", "grape"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["orange", "grape", "peach", "avocado", "pear", "dragonfruit"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["apple", "peach", "pear"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["strawberry", "apple"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["apple", "watermelon"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        let possible_outputs: Vec<Vec<String>> = vec![
            vec![
                "strawberry",
                "apple",
                "orange",
                "watermelon",
                "grape",
                "peach",
                "avocado",
                "pear",
                "dragonfruit",
            ],
            vec![
                "strawberry",
                "apple",
                "watermelon",
                "orange",
                "grape",
                "peach",
                "avocado",
                "pear",
                "dragonfruit",
            ],
        ]
        .iter()
        .map(|possible_output| possible_output.iter().map(|&s| s.to_string()).collect())
        .collect();

        if let Ok(output) = topological_sort(&inputs) {
            assert!(
                possible_outputs.iter().any(|expected| expected == &output),
                "Expected {:?} to be one of {:?}",
                output,
                possible_outputs,
            )
        } else {
            panic!("Conflicting orders in inputs");
        }
    }

    #[test]
    fn test_topological_sort_circular_dependency() {
        let inputs_raw: Vec<Vec<String>> = vec![
            vec!["IAD", "PDX", "DUB", "NRT", "SYD", "FRA"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["HKG", "DUB", "CMH", "KUL"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["BOM", "CMH", "PDX"]
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            vec!["SIN", "FRA"].iter().map(|&s| s.to_string()).collect(),
            vec!["ZAZ"].iter().map(|&s| s.to_string()).collect(),
        ];

        let mut inputs: Vec<&Vec<String>> = Vec::new();
        for input_raw in &inputs_raw {
            inputs.push(input_raw);
        }

        if let Ok(output) = topological_sort(&inputs) {
            panic!(
                "Expected conflicting orders in inputs, but got output {:?}",
                output
            );
        }
    }

    #[test]
    fn test_combine_value_ranges() {
        let ranges: Vec<(u64, u64)> = vec![];
        assert_eq!(combine_value_ranges(ranges), (0, 0));

        let ranges = vec![(5, 10)];
        assert_eq!(combine_value_ranges(ranges), (5, 10));

        let ranges = vec![(5, 10), (3, 8), (7, 15)];
        assert_eq!(combine_value_ranges(ranges), (3, 15));

        let ranges = vec![(1, 5), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (1, 7));

        let ranges = vec![(0, 5), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (0, 7));

        let ranges = vec![(5, u64::MAX), (3, 7), (4, 6)];
        assert_eq!(combine_value_ranges(ranges), (3, u64::MAX));

        let ranges = vec![(5, 10), (5, 15), (5, 8)];
        assert_eq!(combine_value_ranges(ranges), (5, 15));

        let ranges = vec![(5, 10), (3, 10), (7, 10)];
        assert_eq!(combine_value_ranges(ranges), (3, 10));

        let ranges = vec![(5, 5), (5, 5), (5, 5)];
        assert_eq!(combine_value_ranges(ranges), (5, 5));
    }
}
