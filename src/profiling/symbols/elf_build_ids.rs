use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// An ELF Build-ID is a unique identifier embedded in an ELF file by linkers, which is to
/// uniquely identify a specific build of a binary or shared library. Tools like Perf use
/// this ID to cache an ELF file, in case the original one being profiled has changed or
/// becomes inaccessible.
/// This struct caches an ELF file's Build-ID, and uses a Build-ID to look for the ELF file
/// in common places.
#[derive(Default)]
pub struct ElfBuildIds {
    /// A map from an ELF file's path to its build id.
    build_ids: HashMap<String, String>,
}

impl ElfBuildIds {
    /// Store a pair of ELF file path and its Build-ID.
    pub fn add_build_id(&mut self, elf_file_path: &str, build_id: String) {
        self.build_ids.insert(elf_file_path.to_string(), build_id);
    }

    /// Find the original build version of the ELF file using the ELF file's Build-ID.
    pub fn find_original_elf_file(&self, elf_file_path: &str) -> Option<PathBuf> {
        let build_id = self.build_ids.get(elf_file_path)?;

        let home = std::env::var("HOME").ok()?;

        // The primary location where Perf writes the elf file to during capture:
        // ~/.debug/<path>/<buildid>/elf
        let mut original_elf_file_path = PathBuf::from(&home)
            .join(".debug")
            .join(elf_file_path.trim_start_matches('/'))
            .join(build_id)
            .join("elf");
        if let Ok(true) = fs::exists(&original_elf_file_path) {
            return Some(original_elf_file_path);
        }

        // Fallback location that Perf also populates:
        // ~/.debug/.build-id/<first 2 hex>/<rest>/elf
        if build_id.len() > 2 {
            let (head, tail) = build_id.split_at(2);
            original_elf_file_path = PathBuf::from(&home)
                .join(".debug/.build-id")
                .join(head)
                .join(tail)
                .join("elf");
            if let Ok(true) = fs::exists(&original_elf_file_path) {
                return Some(original_elf_file_path);
            }
            // Some perf versions use the full hash as the filename rather than
            // an 'elf' file inside a directory.
            original_elf_file_path = PathBuf::from(&home)
                .join(".debug/.build-id")
                .join(head)
                .join(tail);
            if let Ok(true) = fs::exists(&original_elf_file_path) {
                return Some(original_elf_file_path);
            }
        }

        None
    }
}
