//! Profiling data format parsers, converters, and core profile structures.
//!
//! This module contains:
//! - Core profile data structures ([`Profile`], [`CCTree`], [`ThreadState`], etc.)
//! - [`jfr`] — JFR (Java Flight Recorder) binary format parser for async-profiler output.

pub mod jfr;

pub const BUCKET_WIDTH_MS: u64 = 100;

use crate::data::common::data_formats::Profiler;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cell::OnceCell;
use std::collections::HashMap;

/// A single profiling type's data (e.g., cpu, wall, allocation).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Profile graph visualization (to be removed after native visualization is supported)
    pub profile_graph: ProfileGraph,
    /// Time-ordered blocks of sample data, [thread_state_id -> node_id (index into context_tree) -> sample count]
    pub blocks: Vec<HashMap<u8, HashMap<usize, u64>>>,
    /// Block number time range where profile node aggregate counts are calculated
    pub time_range: (usize, usize),
    /// Tree nodes, index 0 is the root (index is node_id)
    pub context_tree: Vec<CCTreeNode>,
    /// Bidirectional map frame_id to Frame: each Frame stores its name and the node_ids that use it
    pub frame_map: FrameMap,
    /// Cache: thread_state_id -> total self_samples across all nodes (lazily computed in report time)
    #[serde(skip)]
    total_samples_per_thread_state: OnceCell<HashMap<u8, u64>>,
}

/// Information about a graph. TODO: Will remove after native profiler visualization is implemented
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ProfileGraph {
    /// The name of the graph.
    pub graph_name: String,
    /// The relative path to graph (value of the IFrame's src attribute).
    pub graph_path: String,
    /// The size of the graph, which can be used for graph ordering in the report.
    pub graph_size: Option<u64>,
}

impl ProfileGraph {
    pub fn new(graph_name: String, graph_path: String, graph_size: Option<u64>) -> Self {
        ProfileGraph {
            graph_name,
            graph_path,
            graph_size,
        }
    }
}

/// A node in the call tree. Each node represents a unique call path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CCTreeNode {
    /// Index of parent node in the nodes vec, None for root
    pub parent: Option<usize>,
    /// Frame ID, indexes into frame_map to get frame name
    pub frame_id: usize,
    /// Map thread_state_id -> sample count for this frame for time_range specified in CCTree
    pub sample_stats: HashMap<u8, SampleStats>,
    /// Map from child frame_id -> child node_id for fast tree insertion
    pub children: HashMap<usize, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleStats {
    /// Number of samples in this frame, including children
    pub total_samples: u64,
    /// Number of samples in this frame, excluding children
    pub self_samples: u64,
}

/// Frame id to name bidirectional mapping, with name→index lookup for insertion.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FrameMap {
    frame_id_to_frame: Vec<Frame>,
    #[serde(skip)]
    frame_name_to_frame_id: HashMap<String, usize>,
}

/// A frame: its name and all call tree nodes that have this frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub name: String,
    pub node_ids: Vec<usize>,
}

impl FrameMap {
    pub fn new() -> Self {
        Self {
            frame_id_to_frame: Vec::new(),
            frame_name_to_frame_id: HashMap::new(),
        }
    }

    /// Get or insert a frame name, returning its frame_id.
    pub fn get_or_insert(&mut self, name: &str) -> usize {
        if let Some(&id) = self.frame_name_to_frame_id.get(name) {
            return id;
        }
        let id = self.frame_id_to_frame.len();
        self.frame_id_to_frame.push(Frame {
            name: name.to_string(),
            node_ids: Vec::new(),
        });
        self.frame_name_to_frame_id.insert(name.to_string(), id);
        id
    }

    /// Get frame name by ID.
    pub fn name(&self, id: usize) -> &str {
        &self.frame_id_to_frame[id].name
    }

    /// Get node IDs for a given frame id.
    pub fn node_ids(&self, id: usize) -> &[usize] {
        &self.frame_id_to_frame[id].node_ids
    }

    /// Register a node_id under a frame.
    pub fn add_node(&mut self, frame_id: usize, node_id: usize) {
        self.frame_id_to_frame[frame_id].node_ids.push(node_id);
    }

    pub fn len(&self) -> usize {
        self.frame_id_to_frame.len()
    }
}

// Public functions for building Profiler
impl Profiler {
    /// Insert a stack sample into the appropriate profile and time block.
    ///
    /// Computes the block index from `sample_time_ms` relative to `self.start_time`
    /// and `self.block_width`, then delegates to [`Profile::insert_stack`].
    pub fn insert_stack(
        &mut self,
        profile_type: &str,
        sample_time_ms: i64,
        thread_state: ThreadState,
        frames: &[String],
        count: u64,
    ) {
        let profile = self
            .profiles
            .entry(profile_type.to_string())
            .or_insert_with(|| Profile::new());
        profile.insert_stack(
            sample_time_ms,
            self.start_time_ms,
            self.block_width_ms,
            thread_state,
            frames,
            count,
        );
    }

    /// Get sample count for a stack pattern in a specific profile type.
    /// Returns 0 if the profile type doesn't exist.
    pub fn get_samples(
        &self,
        profile_type: &str,
        pattern: &[&str],
        frame_type: Option<FrameType>,
        thread_states: &[ThreadState],
        total_samples: bool,
    ) -> u64 {
        self.profiles.get(profile_type).map_or(0, |p| {
            p.get_samples(pattern, frame_type, thread_states, total_samples)
        })
    }

    /// Get total samples across all nodes for a specific profile type.
    /// Returns 0 if the profile type doesn't exist.
    pub fn get_total_samples(&self, profile_type: &str, thread_states: &[ThreadState]) -> u64 {
        self.profiles
            .get(profile_type)
            .map_or(0, |p| p.get_total_samples(thread_states))
    }

    /// Generate the collapsed format of the current context tree, which has the aggregated sample
    /// counts for Profile.time_range.
    pub fn generate_collapsed(&self, profile_type: &str, thread_states: &[ThreadState]) -> String {
        match self.profiles.get(profile_type) {
            Some(profile) => profile.generate_collapsed(thread_states),
            None => String::new(),
        }
    }

    /// Update context_tree sample_counts to contain the aggregate of samples between the specified
    /// start and end times.
    pub fn set_time_range(
        &mut self,
        relative_start_time_ms: u64,
        relative_end_time_ms: u64,
    ) -> Result<()> {
        let start_idx = (relative_start_time_ms / self.block_width_ms) as usize;
        let end_idx = (relative_end_time_ms / self.block_width_ms) as usize;

        for (_profile_type, profile) in self.profiles.iter_mut() {
            profile.set_time_range(start_idx, end_idx)?;
        }
        Ok(())
    }
}

impl Profile {
    pub fn new() -> Self {
        let mut frame_map = FrameMap::new();
        frame_map.get_or_insert("[root]");
        Self {
            profile_graph: ProfileGraph::default(),
            blocks: Vec::new(),
            time_range: (0, 0),
            context_tree: vec![CCTreeNode {
                parent: None,
                frame_id: 0,
                sample_stats: HashMap::new(),
                children: HashMap::new(),
            }],
            frame_map,
            total_samples_per_thread_state: OnceCell::new(),
        }
    }

    pub fn with_graph(profile_graph: ProfileGraph) -> Self {
        let mut p = Self::new();
        p.profile_graph = profile_graph;
        p
    }

    /// Returns sum of sample counts for call graph nodes matching a stack pattern.
    /// Pattern frames must appear in order walking from root to leaf (i.e., parent→child order).
    /// Example: pattern ["A", "C"] matches nodes named "C" that have an ancestor named "A".
    ///
    /// Uses recursive DFS with regex index tracking:
    /// - total_samples: stops at the shallowest full match (total_samples includes descendants).
    /// - self_samples: continues to all nodes matching the last regex at any depth.
    fn get_samples(
        &self,
        pattern: &[&str],
        frame_type: Option<FrameType>,
        thread_states: &[ThreadState],
        total_samples: bool,
    ) -> u64 {
        if pattern.is_empty() || self.context_tree.is_empty() {
            return 0;
        }

        // Convert patterns to regex with frame_type suffix added to last pattern.
        let regexes: Vec<Regex> = pattern
            .iter()
            .enumerate()
            .map(|(idx, frame)| {
                let suffix = if idx == pattern.len() - 1 {
                    frame_type.map_or(FrameType::any_regex_suffix(), |ft| ft.regex_suffix())
                } else {
                    FrameType::any_regex_suffix()
                };
                Regex::new(&format!("^{}{}$", frame, suffix)).unwrap()
            })
            .collect();

        let thread_state_ids = self.resolve_thread_states(thread_states);
        self.dfs_sum_samples(0, 0, &regexes, &thread_state_ids, total_samples)
    }

    /// DFS with regex index tracking. On full pattern match:
    /// - total_samples=true: sum total_samples, stop (already includes descendants).
    /// - total_samples=false: sum self_samples, keep descending for deeper last-regex matches.
    fn dfs_sum_samples(
        &self,
        node_id: usize,
        regex_idx: usize,
        regexes: &[Regex],
        thread_state_ids: &[u8],
        total_samples: bool,
    ) -> u64 {
        let mut result: u64 = 0;
        for (&child_frame_id, &child_node_id) in &self.context_tree[node_id].children {
            // Increment regex index if current frame matches
            let next_idx = if regexes[regex_idx].is_match(self.frame_map.name(child_frame_id)) {
                regex_idx + 1
            } else {
                regex_idx
            };

            if next_idx == regexes.len() {
                let node = &self.context_tree[child_node_id];
                result += thread_state_ids
                    .iter()
                    .filter_map(|ts| {
                        node.sample_stats.get(ts).map(|s| {
                            if total_samples {
                                s.total_samples
                            } else {
                                s.self_samples
                            }
                        })
                    })
                    .sum::<u64>();
                if !total_samples {
                    result += self.dfs_sum_samples(
                        child_node_id,
                        regex_idx,
                        regexes,
                        thread_state_ids,
                        false,
                    );
                }
            } else {
                result += self.dfs_sum_samples(
                    child_node_id,
                    next_idx,
                    regexes,
                    thread_state_ids,
                    total_samples,
                );
            }
        }
        result
    }

    /// Get total samples across all nodes for specified thread states.
    /// If thread_states is empty, sums all thread states.
    fn get_total_samples(&self, thread_states: &[ThreadState]) -> u64 {
        let map = self.total_samples_per_thread_state.get_or_init(|| {
            let mut m = HashMap::new();
            for node in &self.context_tree {
                for (&thread_state_id, stats) in &node.sample_stats {
                    *m.entry(thread_state_id).or_default() += stats.self_samples;
                }
            }
            m
        });
        let thread_state_ids = self.resolve_thread_states(thread_states);
        thread_state_ids
            .iter()
            .filter_map(|thread_state_id| map.get(thread_state_id))
            .sum()
    }

    /// This function calculates the appropriate bucket index and inserts a stack frame with count
    /// into Profile.
    pub fn insert_stack(
        &mut self,
        sample_time_ms: i64,
        start_time_ms: i64,
        bucket_width_ms: u64,
        thread_state: ThreadState,
        frames: &[String],
        count: u64,
    ) {
        // Calculate block index and extend blocks vec if necessary
        let thread_state_id = thread_state.id();
        let offset_ms = (sample_time_ms - start_time_ms).max(0) as u64;
        let block_idx = (offset_ms / bucket_width_ms) as usize;

        while self.blocks.len() <= block_idx {
            self.blocks.push(HashMap::new());
        }

        // Iterate through tree layers by frame
        let mut curr_node_id = 0usize;

        for frame_name in frames {
            let frame_id = self.frame_map.get_or_insert(frame_name);

            curr_node_id = if let Some(&child_node_id) =
                self.context_tree[curr_node_id].children.get(&frame_id)
            {
                child_node_id
            } else {
                // New call context found: insert new node, and update parent node and frames map
                let new_node_id = self.context_tree.len();
                self.context_tree.push(CCTreeNode {
                    parent: Some(curr_node_id),
                    frame_id,
                    sample_stats: HashMap::new(),
                    children: HashMap::new(),
                });
                self.context_tree[curr_node_id]
                    .children
                    .insert(frame_id, new_node_id);
                self.frame_map.add_node(frame_id, new_node_id);
                new_node_id
            };

            // Update sample stats total_samples for non-leaf nodes
            self.context_tree[curr_node_id]
                .sample_stats
                .entry(thread_state_id)
                .or_insert(SampleStats {
                    total_samples: 0,
                    self_samples: 0,
                })
                .total_samples += count;
        }

        // For the leaf node of stack, update self samples
        self.context_tree[curr_node_id]
            .sample_stats
            .entry(thread_state_id)
            .or_insert(SampleStats {
                total_samples: 0,
                self_samples: 0,
            })
            .self_samples += count;

        // Update aggregate count in blocks vec
        *self.blocks[block_idx]
            .entry(thread_state_id)
            .or_default()
            .entry(curr_node_id)
            .or_default() += count;
    }

    fn resolve_thread_states(&self, thread_states: &[ThreadState]) -> Vec<u8> {
        if thread_states.is_empty() {
            ThreadState::ALL
                .iter()
                .map(|thread_state| thread_state.id())
                .collect()
        } else {
            thread_states
                .iter()
                .map(|thread_state| thread_state.id())
                .collect()
        }
    }

    /// DFS through call tree, print current path if node self samples is > 0.
    /// collapsed format consists lines of call stack and sample count:
    ///
    /// frame1;frame2;frame3 10
    /// frame1;frame4 20
    pub fn generate_collapsed(&self, thread_states: &[ThreadState]) -> String {
        let thread_state_ids = self.resolve_thread_states(thread_states);
        let mut result = String::new();
        let mut path: Vec<usize> = Vec::new();
        self.dfs_collapsed(0, &thread_state_ids, &mut path, &mut result);
        result
    }

    fn dfs_collapsed(
        &self,
        node_id: usize,
        thread_state_ids: &[u8],
        path: &mut Vec<usize>,
        result: &mut String,
    ) {
        let node = &self.context_tree[node_id];

        // Emit line if this node has self_samples for any requested thread state
        if !path.is_empty() {
            let self_samples: u64 = thread_state_ids
                .iter()
                .filter_map(|ts| node.sample_stats.get(ts).map(|s| s.self_samples))
                .sum();
            if self_samples > 0 {
                let stack: String = path
                    .iter()
                    .map(|&nid| self.frame_map.name(self.context_tree[nid].frame_id))
                    .collect::<Vec<_>>()
                    .join(";");
                result.push_str(&format!("{} {}\n", stack, self_samples));
            }
        }

        for (&_frame_id, &child_node_id) in &node.children {
            path.push(child_node_id);
            self.dfs_collapsed(child_node_id, thread_state_ids, path, result);
            path.pop();
        }
    }

    /// Iterates through the corresponding time blocks in Profile.blocks and
    /// accumulates the sample counts in nodes. Counts are accumulated to a nodes self_samples
    /// and every of its ancestors total samples. Then Profile.time_range is updated.
    pub fn set_time_range(&mut self, start_idx: usize, end_idx: usize) -> Result<()> {
        // Clear existing sample stats
        for node in &mut self.context_tree {
            node.sample_stats.clear();
        }

        // Reset cached totals
        self.total_samples_per_thread_state = OnceCell::new();

        let end = end_idx.min(self.blocks.len());
        for block_idx in start_idx..end {
            for (&thread_state_id, node_map) in &self.blocks[block_idx] {
                for (&node_id, &count) in node_map {
                    // Add self_samples to the leaf node
                    self.context_tree[node_id]
                        .sample_stats
                        .entry(thread_state_id)
                        .or_insert(SampleStats {
                            total_samples: 0,
                            self_samples: 0,
                        })
                        .self_samples += count;

                    // Walk up ancestors adding total_samples
                    let mut cur = node_id;
                    loop {
                        self.context_tree[cur]
                            .sample_stats
                            .entry(thread_state_id)
                            .or_insert(SampleStats {
                                total_samples: 0,
                                self_samples: 0,
                            })
                            .total_samples += count;
                        match self.context_tree[cur].parent {
                            Some(parent) => cur = parent,
                            None => break,
                        }
                    }
                }
            }
        }

        self.time_range = (start_idx, end_idx);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FrameType {
    Jit,         // _[j]
    Inlined,     // _[i]
    Kernel,      // _[k]
    Interpreted, // _[0]
    C1,          // _[1]
    Native,      // no suffix
}

impl FrameType {
    pub(crate) fn literal_suffix(&self) -> &'static str {
        match self {
            FrameType::Jit => "_[j]",
            FrameType::Inlined => "_[i]",
            FrameType::Kernel => "_[k]",
            FrameType::Interpreted => "_[0]",
            FrameType::C1 => "_[1]",
            FrameType::Native => "",
        }
    }

    pub(crate) fn regex_suffix(&self) -> &'static str {
        match self {
            FrameType::Jit => "_\\[j\\]",
            FrameType::Inlined => "_\\[i\\]",
            FrameType::Kernel => "_\\[k\\]",
            FrameType::Interpreted => "_\\[0\\]",
            FrameType::C1 => "_\\[1\\]",
            FrameType::Native => "",
        }
    }

    /// Regex pattern that matches any frame type suffix (including none).
    pub(crate) fn any_regex_suffix() -> &'static str {
        "(_\\[(0|j|i|k|1)\\])?"
    }

    pub(crate) fn from_jfr_name(name: &str) -> Self {
        match name {
            "Interpreted" => FrameType::Interpreted,
            "JIT compiled" => FrameType::Jit,
            "Inlined" => FrameType::Inlined,
            "Kernel" => FrameType::Kernel,
            "C1 compiled" => FrameType::C1,
            _ => FrameType::Native,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ThreadState {
    None = 0,
    AsyncRunnable = 1,
    AsyncSleeping = 2,
    AsyncDefault = 3,
}

impl ThreadState {
    pub const ALL: [ThreadState; 3] = [
        ThreadState::AsyncRunnable,
        ThreadState::AsyncSleeping,
        ThreadState::AsyncDefault,
    ];

    pub fn from_str(name: &str) -> Self {
        match name {
            "STATE_RUNNABLE" => ThreadState::AsyncRunnable,
            "STATE_SLEEPING" => ThreadState::AsyncSleeping,
            "STATE_DEFAULT" => ThreadState::AsyncDefault,
            _ => ThreadState::None,
        }
    }

    pub fn id(self) -> u8 {
        self as u8
    }
}
