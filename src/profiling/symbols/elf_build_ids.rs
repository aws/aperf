use object::read::Object;
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
    build_ids: HashMap<String, Vec<u8>>,
}

impl ElfBuildIds {
    /// Store a pair of ELF file path and its Build-ID.
    pub fn add_build_id(&mut self, elf_file_path: &str, build_id: &[u8]) {
        self.build_ids
            .insert(elf_file_path.to_string(), build_id.to_vec());
    }

    /// Check ELF data found by path against the Build-ID recorded during the capture.
    /// Perf's symsrc__init applies the same rule: once a Build-ID is known for a path, data that
    /// does not carry a matching one is a different build of the file, and using it would resolve
    /// addresses to confidently wrong symbols rather than to nothing.
    pub fn matches_build_id(&self, elf_file_path: &str, elf_data: &[u8]) -> bool {
        let Some(recorded_build_id) = self.build_ids.get(elf_file_path) else {
            // Nothing was recorded for this path, so there is nothing to contradict.
            return true;
        };
        let Ok(elf_file) = object::File::parse(elf_data) else {
            return false;
        };
        match elf_file.build_id() {
            Ok(Some(build_id)) => build_id == recorded_build_id.as_slice(),
            // A file with no Build-ID cannot be shown to be the one that was profiled.
            _ => false,
        }
    }

    /// Find the original build version of the ELF file using the ELF file's Build-ID.
    pub fn find_original_elf_file(&self, elf_file_path: &str) -> Option<PathBuf> {
        let build_id: String = self
            .build_ids
            .get(elf_file_path)?
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        let home = std::env::var("HOME").ok()?;

        // The primary location where Perf writes the elf file to during capture:
        // ~/.debug/<path>/<buildid>/elf
        let mut original_elf_file_path = PathBuf::from(&home)
            .join(".debug")
            .join(elf_file_path.trim_start_matches('/'))
            .join(&build_id)
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

#[cfg(test)]
mod tests {
    use super::*;

    const ELF_FILE_PATH: &str = "/usr/lib/libtest.so";
    const BUILD_ID: [u8; 20] = [
        0x8b, 0x8f, 0x4c, 0x1d, 0x2a, 0x00, 0xe1, 0x34, 0x77, 0x90, 0x5c, 0xd3, 0x0f, 0x21, 0x66,
        0xab, 0xcd, 0xef, 0x01, 0x23,
    ];
    const NT_GNU_ABI_TAG: u32 = 1;
    const NT_GNU_BUILD_ID: u32 = 3;

    /// An ELF64 holding nothing but one GNU note section, which is all matches_build_id reads.
    fn minimal_elf(note_type: u32, note_desc: &[u8]) -> Vec<u8> {
        const ELF_HEADER_SIZE: usize = 64;
        const SECTION_HEADER_SIZE: u16 = 64;
        const SECTION_NAMES: &[u8] = b"\0.note.gnu.build-id\0.shstrtab\0";
        const SHSTRTAB_NAME_OFFSET: u32 = 20;
        const SHT_STRTAB: u32 = 3;
        const SHT_NOTE: u32 = 7;

        let mut note = Vec::new();
        note.extend_from_slice(&4u32.to_le_bytes()); // n_namesz, for "GNU\0"
        note.extend_from_slice(&(note_desc.len() as u32).to_le_bytes()); // n_descsz
        note.extend_from_slice(&note_type.to_le_bytes()); // n_type
        note.extend_from_slice(b"GNU\0");
        note.extend_from_slice(note_desc);
        while note.len() % 4 != 0 {
            note.push(0);
        }

        let note_offset = ELF_HEADER_SIZE;
        let names_offset = note_offset + note.len();
        let section_headers_offset = (names_offset + SECTION_NAMES.len()).next_multiple_of(8);

        let mut elf = Vec::new();
        elf.extend_from_slice(&[0x7f, b'E', b'L', b'F', 2, 1, 1, 0]); // 64-bit, little-endian
        elf.extend_from_slice(&[0; 8]); // e_ident padding
        elf.extend_from_slice(&3u16.to_le_bytes()); // e_type: ET_DYN
        elf.extend_from_slice(&0xb7u16.to_le_bytes()); // e_machine: AArch64
        elf.extend_from_slice(&1u32.to_le_bytes()); // e_version
        elf.extend_from_slice(&0u64.to_le_bytes()); // e_entry
        elf.extend_from_slice(&0u64.to_le_bytes()); // e_phoff
        elf.extend_from_slice(&(section_headers_offset as u64).to_le_bytes()); // e_shoff
        elf.extend_from_slice(&0u32.to_le_bytes()); // e_flags
        elf.extend_from_slice(&(ELF_HEADER_SIZE as u16).to_le_bytes()); // e_ehsize
        elf.extend_from_slice(&0u16.to_le_bytes()); // e_phentsize
        elf.extend_from_slice(&0u16.to_le_bytes()); // e_phnum
        elf.extend_from_slice(&SECTION_HEADER_SIZE.to_le_bytes()); // e_shentsize
        elf.extend_from_slice(&3u16.to_le_bytes()); // e_shnum
        elf.extend_from_slice(&2u16.to_le_bytes()); // e_shstrndx: .shstrtab
        assert_eq!(elf.len(), ELF_HEADER_SIZE);

        elf.extend_from_slice(&note);
        elf.extend_from_slice(SECTION_NAMES);
        elf.resize(section_headers_offset, 0);

        for (name_offset, section_type, offset, size) in [
            (0, 0, 0, 0),
            (1, SHT_NOTE, note_offset as u64, note.len() as u64),
            (
                SHSTRTAB_NAME_OFFSET,
                SHT_STRTAB,
                names_offset as u64,
                SECTION_NAMES.len() as u64,
            ),
        ] {
            elf.extend_from_slice(&name_offset.to_le_bytes()); // sh_name
            elf.extend_from_slice(&section_type.to_le_bytes()); // sh_type
            elf.extend_from_slice(&0u64.to_le_bytes()); // sh_flags
            elf.extend_from_slice(&0u64.to_le_bytes()); // sh_addr
            elf.extend_from_slice(&offset.to_le_bytes()); // sh_offset
            elf.extend_from_slice(&size.to_le_bytes()); // sh_size
            elf.extend_from_slice(&0u32.to_le_bytes()); // sh_link
            elf.extend_from_slice(&0u32.to_le_bytes()); // sh_info
            elf.extend_from_slice(&4u64.to_le_bytes()); // sh_addralign
            elf.extend_from_slice(&0u64.to_le_bytes()); // sh_entsize
        }

        elf
    }

    fn elf_build_ids_with(build_id: &[u8]) -> ElfBuildIds {
        let mut elf_build_ids = ElfBuildIds::default();
        elf_build_ids.add_build_id(ELF_FILE_PATH, build_id);
        elf_build_ids
    }

    #[test]
    fn only_the_recorded_build_id_matches() {
        let elf_build_ids = elf_build_ids_with(&BUILD_ID);
        assert!(
            elf_build_ids.matches_build_id(ELF_FILE_PATH, &minimal_elf(NT_GNU_BUILD_ID, &BUILD_ID))
        );

        let mut other_build_id = BUILD_ID;
        other_build_id[19] ^= 0xff;
        assert!(!elf_build_ids.matches_build_id(
            ELF_FILE_PATH,
            &minimal_elf(NT_GNU_BUILD_ID, &other_build_id)
        ));
        // A truncated Build-ID must not pass as a prefix match.
        assert!(!elf_build_ids.matches_build_id(
            ELF_FILE_PATH,
            &minimal_elf(NT_GNU_BUILD_ID, &BUILD_ID[..16])
        ));
    }

    #[test]
    fn unverifiable_data_matches_only_without_a_recorded_build_id() {
        let no_build_id_elf = minimal_elf(NT_GNU_ABI_TAG, &[0, 0, 0, 0]);

        // Nothing recorded for the path leaves nothing to contradict.
        assert!(ElfBuildIds::default().matches_build_id(ELF_FILE_PATH, &no_build_id_elf));
        assert!(ElfBuildIds::default().matches_build_id(ELF_FILE_PATH, b"not an ELF file"));

        let elf_build_ids = elf_build_ids_with(&BUILD_ID);
        assert!(!elf_build_ids.matches_build_id(ELF_FILE_PATH, &no_build_id_elf));
        assert!(!elf_build_ids.matches_build_id(ELF_FILE_PATH, b"not an ELF file"));
    }
}
