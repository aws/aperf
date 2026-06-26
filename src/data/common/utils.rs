use anyhow::{bail, Error, Result};
use log::error;
use regex::Regex;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
#[cfg(target_os = "linux")]
use {anyhow::Context, log::debug};

use crate::data::common::data_formats::{Graph, GraphData};

pub fn get_data_name_from_type<T>() -> &'static str {
    let full_data_module_path = std::any::type_name::<T>();

    let mut data_identifier_found = false;
    let mut data_name: Option<&str> = None;
    for data_module_part in full_data_module_path.split("::") {
        if data_identifier_found {
            data_name = Some(data_module_part);
            break;
        }
        data_identifier_found = data_module_part == "data";
    }

    match data_name {
        Some(value) => value,
        None => panic!("Could not get data name"),
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Debug)]
pub struct CpuInfo {
    pub part: Option<String>,
    pub vendor_id: Option<String>,
    pub model_name: Option<String>,
}

#[cfg(target_os = "linux")]
impl CpuInfo {
    pub fn new() -> Result<Self> {
        let cpu_info_file = File::open("/proc/cpuinfo")?;
        let cpu_info_reader = BufReader::new(cpu_info_file);
        let mut part = None;
        let mut vendor_id = None;
        let mut model_name = None;
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
                _ => continue,
            }
        }

        Ok(Self {
            part,
            vendor_id,
            model_name,
        })
    }

    pub fn is_graviton(&self) -> bool {
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

    pub fn is_intel_icelake(&self) -> bool {
        self.model_name.as_ref().map_or(false, |model_name| {
            model_name == "Intel(R) Xeon(R) Platinum 8375C CPU @ 2.90GHz"
        })
    }

    pub fn is_intel_sapphire_rapids(&self) -> bool {
        self.model_name.as_ref().map_or(false, |model_name| {
            model_name == "Intel(R) Xeon(R) Platinum 8488C"
        })
    }

    pub fn is_amd(&self) -> bool {
        self.vendor_id
            .as_ref()
            .map_or(false, |vendor_id| vendor_id == "AuthenticAMD")
    }

    pub fn is_amd_genoa(&self) -> bool {
        self.model_name
            .as_ref()
            .map_or(false, |model_name| model_name == "AMD EPYC 9R14")
    }

    pub fn is_amd_milan(&self) -> bool {
        self.model_name
            .as_ref()
            .map_or(false, |model_name| model_name == "AMD EPYC 7R13")
    }
}

#[cfg(target_os = "linux")]
/// Return the IDs of all online CPUs by parsing /sys/devices/system/cpu/online,
/// a comma-separated list of single CPUs and inclusive ranges, e.g. "0-1,3-5,7".
pub fn get_online_cpu_ids() -> Result<Vec<usize>> {
    let mut ids = Vec::new();
    let cpu_list = fs::read_to_string("/sys/devices/system/cpu/online")?;
    let cpu_list = cpu_list.trim();
    if cpu_list.is_empty() {
        return Ok(ids);
    }
    for part in cpu_list.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part.split_once('-') {
            Some((low, high)) => {
                let low: usize = low.trim().parse()?;
                let high: usize = high.trim().parse()?;
                if high < low {
                    bail!("invalid CPU range '{part}' in cpu list '{cpu_list}'");
                }
                ids.extend(low..=high);
            }
            None => ids.push(part.parse()?),
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
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

pub fn no_tar_gz_file_name(path: &PathBuf) -> Option<String> {
    if path.file_name().is_none() {
        return None;
    }

    let file_name_str = path.file_name()?.to_str()?.to_string();

    if file_name_str.ends_with(".tar.gz") {
        return Some(file_name_str.strip_suffix(".tar.gz")?.to_string());
    }
    Some(file_name_str)
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

/// Returns the name of the first file in dir whose name matches the pattern regex but does
/// not match the optional exclude regex.
pub fn find_file(dir: &PathBuf, pattern: &str, exclude_pattern: Option<&str>) -> Result<String> {
    let regex = Regex::new(pattern)?;
    let exclude_regex = exclude_pattern.map(Regex::new).transpose()?;
    for entry in fs::read_dir(dir)? {
        let filename = entry?.file_name().into_string().unwrap();
        if regex.is_match(&filename)
            && !exclude_regex
                .as_ref()
                .is_some_and(|ex| ex.is_match(&filename))
        {
            return Ok(filename);
        }
    }
    match exclude_pattern {
        Some(exclude_pattern) => bail!(
            "Could not find any file matching /{pattern}/ (excluding /{exclude_pattern}/) in {}",
            dir.display()
        ),
        None => bail!(
            "Could not find any file matching /{pattern}/ in {}",
            dir.display()
        ),
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
    use super::{combine_value_ranges, find_file, topological_sort};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

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
        // Count should match sysconf(_SC_NPROCESSORS_ONLN) on a normal system.
        let nproc = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN as libc::c_int) } as usize;
        assert_eq!(ids.len(), nproc, "online CPU count should match sysconf");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_cpu_info() {
        use super::CpuInfo;

        let cpu_info = CpuInfo::new().expect("Should read and parse /proc/cpuinfo");

        assert!(
            cpu_info.is_graviton() || cpu_info.is_intel() || cpu_info.is_amd(),
            "The CPU should be recognized as one of Graviton, Intel, and AMD"
        );
    }

    #[test]
    fn test_find_file_prefix_match() {
        let dir = TempDir::new().unwrap();
        for f in &[
            "cpu_utilization.bin",
            "other_cpu_utilization.bin",
            "noise.txt",
        ] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        // Anchored at the start with `^`.
        assert_eq!(
            find_file(&path, "^cpu_utilization", None).unwrap(),
            "cpu_utilization.bin",
        );
        // No match returns Err.
        assert!(find_file(&path, "^missing", None).is_err());
    }

    #[test]
    fn test_find_file_suffix_match() {
        let dir = TempDir::new().unwrap();
        for f in &["data.bin", "data.bin.bak", "noise.txt"] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        // Anchored at the end with `$` (".bin" mid-name in "data.bin.bak" doesn't match).
        assert_eq!(find_file(&path, r"\.bin$", None).unwrap(), "data.bin");
        // No match returns Err.
        assert!(find_file(&path, r"\.missing$", None).is_err());
    }

    #[test]
    fn test_find_file_excludes_substring_collision() {
        // Regression test: the forward flamegraph lookup must not pick up
        // `reverse-flamegraph.svg`, whose name also ends in `flamegraph.svg`. Create the files
        // in both orders to defeat any reliance on directory read ordering.
        for order in [
            ["flamegraph.svg", "reverse-flamegraph.svg"],
            ["reverse-flamegraph.svg", "flamegraph.svg"],
        ] {
            let dir = TempDir::new().unwrap();
            for f in order {
                fs::File::create(dir.path().join(f)).unwrap();
            }
            let path = PathBuf::from(dir.path());
            // Forward: match `flamegraph.svg` but exclude the reverse variant.
            assert_eq!(
                find_file(
                    &path,
                    r"flamegraph\.svg$",
                    Some(r"reverse-flamegraph\.svg$")
                )
                .unwrap(),
                "flamegraph.svg",
            );
            // Reverse: matches only the reverse variant.
            assert_eq!(
                find_file(&path, r"reverse-flamegraph\.svg$", None).unwrap(),
                "reverse-flamegraph.svg",
            );
        }
    }

    #[test]
    fn test_find_file_excludes_legacy_run_prefixed_names() {
        // The same disambiguation must hold for the legacy `<run>-flamegraph.svg` naming.
        let dir = TempDir::new().unwrap();
        for f in &["myrun-flamegraph.svg", "myrun-reverse-flamegraph.svg"] {
            fs::File::create(dir.path().join(f)).unwrap();
        }
        let path = PathBuf::from(dir.path());
        assert_eq!(
            find_file(
                &path,
                r"flamegraph\.svg$",
                Some(r"reverse-flamegraph\.svg$")
            )
            .unwrap(),
            "myrun-flamegraph.svg",
        );
        assert_eq!(
            find_file(&path, r"reverse-flamegraph\.svg$", None).unwrap(),
            "myrun-reverse-flamegraph.svg",
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
