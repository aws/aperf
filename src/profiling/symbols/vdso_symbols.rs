use std::fs;
use std::io::{Read, Seek, SeekFrom};

/// Read the vDSO ELF data from the current process's memory. The vDSO is identical
/// for all processes on the same kernel.
pub fn read_vdso_elf_data() -> Option<Vec<u8>> {
    // Find vDSO base address from /proc/self/auxv:
    // Each entry is two native-sized words: a_type (a small integer tag defined in elf.h as AT_*)
    // followed by a_un.a_val (an u64 whose meaning depends on the tag). The list ends with a_type == AT_NULL (0).
    let auxv = fs::read("/proc/self/auxv").ok()?;
    let mut vdso_base: u64 = 0;
    let mut i = 0;
    while i + 16 <= auxv.len() {
        let tag = u64::from_ne_bytes(auxv[i..i + 8].try_into().ok()?);
        let val = u64::from_ne_bytes(auxv[i + 8..i + 16].try_into().ok()?);
        // Tag AT_SYSINFO_EHDR = 33 contains the base address of the vDSO ELF image in this process's memory
        if tag == 33 {
            vdso_base = val;
            break;
        }
        if tag == 0 {
            break;
        }
        i += 16;
    }
    if vdso_base == 0 {
        return None;
    }

    // Locate and read the vDSO ELF data from the process's own memory
    let mut virtual_memory = fs::File::open("/proc/self/mem").ok()?;
    virtual_memory.seek(SeekFrom::Start(vdso_base)).ok()?;
    let mut elf_header = [0u8; 64];
    virtual_memory.read_exact(&mut elf_header).ok()?;
    // Verify ELF magic
    if &elf_header[0..4] != b"\x7fELF" {
        return None;
    }

    // Get program header and section header info to determine total size
    let program_header_table_offset = u64::from_le_bytes(elf_header[32..40].try_into().ok()?);
    let program_header_entry_size = u16::from_le_bytes(elf_header[54..56].try_into().ok()?) as u64;
    let num_program_header_entries = u16::from_le_bytes(elf_header[56..58].try_into().ok()?) as u64;
    let section_header_table_offset = u64::from_le_bytes(elf_header[40..48].try_into().ok()?);
    let section_header_entry_size = u16::from_le_bytes(elf_header[58..60].try_into().ok()?) as u64;
    let num_section_entries = u16::from_le_bytes(elf_header[60..62].try_into().ok()?) as u64;

    // Ensure that both of the program header and section header tables are included
    let mut elf_data_end: u64 =
        section_header_table_offset + num_section_entries * section_header_entry_size;
    elf_data_end = elf_data_end
        .max(program_header_table_offset + num_program_header_entries * program_header_entry_size);
    // Check all LOAD segments to ensure that we read the complete ELF data.
    virtual_memory
        .seek(SeekFrom::Start(vdso_base + program_header_table_offset))
        .ok()?;
    for _ in 0..num_program_header_entries {
        let mut program_header_entry = vec![0u8; program_header_entry_size as usize];
        virtual_memory.read_exact(&mut program_header_entry).ok()?;
        let p_type = u32::from_le_bytes(program_header_entry[0..4].try_into().ok()?);
        if p_type == 1 {
            // PT_LOAD
            let p_offset = u64::from_le_bytes(program_header_entry[8..16].try_into().ok()?);
            let p_file_size = u64::from_le_bytes(program_header_entry[32..40].try_into().ok()?);
            elf_data_end = elf_data_end.max(p_offset + p_file_size)
        }
    }

    // Read the entire vDSO
    virtual_memory.seek(SeekFrom::Start(vdso_base)).ok()?;
    let mut elf_data = vec![0u8; elf_data_end as usize];
    virtual_memory.read_exact(&mut elf_data).ok()?;

    Some(elf_data)
}
