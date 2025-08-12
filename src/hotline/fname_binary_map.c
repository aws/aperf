#include "fname_binary_map.h"

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <stdlib.h>

#include "log.h"

static const Dwfl_Callbacks callbacks = {
    .find_elf = dwfl_build_id_find_elf,
    .find_debuginfo = dwfl_standard_find_debuginfo,
    .section_address = dwfl_offline_section_address,
};

struct btree *fname_binary_map = NULL;

/// @brief Comparison function for filename binary mappings
/// @param a First fname_binary_entry to compare
/// @param b Second fname_binary_entry to compare
/// @param udata Unused
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int fname_binary_map_compare(const void *a, const void *b, void *udata) {
  (void)udata;
  const fname_binary_map_entry_t *ua = a;
  const fname_binary_map_entry_t *ub = b;

  return strcmp(ua->filename, ub->filename);
}

/// @brief Helper function to process ELF sections
/// @param info Binary info struct to populate with ELF information
/// @return true if successfully mapped file, false otherwise
bool process_elf_sections(binary_info_t *info) {
  for (int i = 0; i < info->ehdr->e_shnum; i++) {
    char *section_name = info->shstrtab + info->shdr[i].sh_name;

    if (strcmp(section_name, ".text") == 0) {
      info->text_section = (char *)info->map + info->shdr[i].sh_offset;
      info->text_addr = info->shdr[i].sh_addr;
      info->text_size = info->shdr[i].sh_size;
    } else if (strcmp(section_name, ".symtab") == 0) {
      info->symtab = (Elf64_Sym *)((char *)info->map + info->shdr[i].sh_offset);
      info->sym_count = info->shdr[i].sh_size / sizeof(Elf64_Sym);
    } else if (strcmp(section_name, ".strtab") == 0) {
      info->strtab = (char *)info->map + info->shdr[i].sh_offset;
    }
  }
  return (info->text_section && info->symtab && info->strtab);
}

/// @brief Helper function to initialize DWARD debuf info
/// @param info Debug info struct to populate
/// @param filename File to populate struct for
/// @return true if successfully initialized, false otherwise
bool init_dwarf_info(binary_info_t *info, const char *filename) {
  info->dwfl = dwfl_begin(&callbacks);
  if (!info->dwfl) {
    fprintf(stderr, "Failed to initialize DWARF reader\n");
    return false;
  }

  dwfl_report_begin(info->dwfl);
  Dwfl_Module *module = dwfl_report_elf(info->dwfl, filename, filename, -1, 0, false);
  dwfl_report_end(info->dwfl, NULL, NULL);

  if (!module) {
    fprintf(stderr, "Failed to load debug info\n");
    dwfl_end(info->dwfl);
    return false;
  }

  return true;
}

/// @brief Loads all the information for a binary and returns a pointer to the
/// struct
/// @param filename Binary file to load
/// @return Pointer to populated struct
binary_info_t *load_binary(const char *filename) {
  // Allocate and initialize binary info structure
  binary_info_t *info = malloc(sizeof(binary_info_t));
  ASSERT(info, "Failed to allocate memory for binary_info_t");
  memset(info, 0, sizeof(binary_info_t));

  // The following errors are graceful, because we may not always be able to map
  // a profiled binary bag with debug symbols.

  // Initialize Capstone
  if (cs_open(CS_ARCH_ARM64, CS_MODE_ARM, &info->cs_handle) != CS_ERR_OK) {
    fprintf(stderr, "Failed to initialize Capstone\n");
    goto cleanup_info;
  }

  // Open the binary file
  int fd = open(filename, O_RDONLY);
  if (fd < 0) {
    fprintf(stderr, "Non-fatal: Cannot open binary %s\n", filename);
    goto cleanup_capstone;
  }

  // Get file size
  struct stat st;
  if (fstat(fd, &st) < 0) {
    fprintf(stderr, "Failed to get file stats\n");
    goto cleanup_fd;
  }
  info->size = st.st_size;

  // Map file into memory
  info->map = mmap(NULL, info->size, PROT_READ, MAP_PRIVATE, fd, 0);
  ASSERT(info->map != MAP_FAILED, "Failed to mmap binary.");

  // Parse ELF header
  info->ehdr = (Elf64_Ehdr *)info->map;

  // This is a graceful cleanup without an ASSERT because sometimes files like
  // [vdso] and [stack] show up, and we don't want to crash on them.
  if (memcmp(info->ehdr->e_ident, ELFMAG, SELFMAG) != 0) {
    fprintf(stderr, "Not an ELF file\n");
    goto cleanup_mmap;
  }

  // Get section headers and string table
  info->shdr = (Elf64_Shdr *)((char *)info->map + info->ehdr->e_shoff);
  info->shstrtab = (char *)info->map + info->shdr[info->ehdr->e_shstrndx].sh_offset;

  // These are also graceful cleanups for the same reason. We don't want to
  // crash the whole program for a single file that may not be traceable back
  // with debug info. Find and process important sections
  if (!process_elf_sections(info)) {
    goto cleanup_mmap;
  }

  // Initialize DWARF debug info
  if (!init_dwarf_info(info, filename)) {
    goto cleanup_mmap;
  }

  return info;

  // Cleanup handlers
cleanup_mmap:
  munmap(info->map, info->size);
cleanup_fd:
  close(fd);
cleanup_capstone:
  cs_close(&info->cs_handle);
cleanup_info:
  free(info);
  return NULL;
}

/// @brief Initializes the mapping structure
void init_fname_binary_btree() {
  fname_binary_map = btree_new(sizeof(fname_binary_map_entry_t), 0, fname_binary_map_compare, NULL);
  btree_clear(fname_binary_map);
}

/// @brief If entry exists, returns it. Otherwise loads the binary and puts it
/// in.
/// @param filename Binary to search for
/// @return Pointer to binary.
binary_info_t *get_fname_binary_map_entry(char *filename) {
  fname_binary_map_entry_t fname_entry;
  fname_entry.filename = filename;

  const fname_binary_map_entry_t *result = btree_get(fname_binary_map, &fname_entry);
  if (result == NULL) {  // need to load binary now
    binary_info_t *info = load_binary(filename);
    fname_entry.binary_info = info;
    btree_set(fname_binary_map, &fname_entry);
    return info;
  } else {
    return result->binary_info;  // already loaded previously, just recycle
  }
}

#ifdef __cplusplus
extern "C" {
#endif
char* __cxa_demangle(const char* mangled_name, char* output_buffer,
                     size_t* length, int* status);
#ifdef __cplusplus
}
#endif

/// @brief Demangles function names decoded from ELF file
/// @param mangled Mangled function name
/// @return Returns demangled name on success, mangled on failure
char* demangle(const char* mangled) {
    if (!mangled) return NULL;
    
    int status = 0;
    char* demangled = __cxa_demangle(mangled, NULL, NULL, &status);
    
    if (status == 0 && demangled) {
        return demangled;
    } else {
        if (demangled) free(demangled);
        return strdup(mangled);
    }
}

/// @brief Returns the function associated with the addr
/// @param info Binary info structure for file
/// @param addr Offset into file
/// @return Function name
char *get_function_name(binary_info_t *info, uint64_t addr) {
  if (!info || !info->symtab || !info->strtab) return NULL;

  for (int i = 0; i < info->sym_count; i++) {
    if (ELF64_ST_TYPE(info->symtab[i].st_info) == STT_FUNC) {
      if (addr >= info->symtab[i].st_value &&
          addr < info->symtab[i].st_value + info->symtab[i].st_size) {
        char *name = strdup(info->strtab + info->symtab[i].st_name);
        ASSERT(name != NULL, "Function name should not be null.");
        char *demangled = demangle(name);
        char *quoted_function = malloc(strlen(demangled) + 3);

        sprintf(quoted_function, "\"%s\"", demangled);
        free(demangled);
        return quoted_function;
      }
    }
  }
  return NULL;
}

/// @brief Returns the assembly associated with the addr
/// @param info Binary info structure for file
/// @param addr Offset into file
/// @return Assembly code
char *get_assembly(binary_info_t *info, uint64_t offset) {
  char *assembly = NULL;

  // Check if offset is in text section
  if (offset >= info->text_addr && offset < info->text_addr + info->text_size) {
    uint64_t roffset = offset - info->text_addr;
    cs_insn *insn;
    size_t count = cs_disasm(info->cs_handle, (uint8_t *)info->text_section + roffset,
                             4,  // Read 4 bytes for instruction
                             offset,
                             1,  // Disassemble 1 instruction
                             &insn);

    if (count > 0) {
      // Allocate and format assembly string
      size_t len = strlen(insn[0].mnemonic) + strlen(insn[0].op_str) + 2;
      assembly = malloc(len);
      ASSERT(assembly != NULL, "Failed to malloc space for assembly string.");
      if (assembly) {
        int res = snprintf(assembly, len, "%s %s", insn[0].mnemonic, insn[0].op_str);
        ASSERT(res > 0, "snprintf failed.");

        // Replace commas with spaces
        for (char *p = assembly; *p; p++) {
          if (*p == ',') *p = ' ';
        }
      }
      cs_free(insn, count);
    }
  }

  return assembly;
}

/// @brief Converts a relative path to an absolute path
/// @param binary_path Location of binary file
/// @param source_path Location of source *from* binary path
/// @return Absolute path of source file
char *get_absolute_source_path(const char *binary_path, const char *source_path) {
  if (!source_path) {
    return NULL;
  }

  // If it's already an absolute path, just return a copy
  if (source_path[0] == '/') {
    return strdup(source_path);
  }

  // For relative paths, resolve against the binary's directory
  char *binary_dir = strdup(binary_path);
  char *last_slash = strrchr(binary_dir, '/');
  if (last_slash) {
    *(last_slash + 1) = '\0';  // Cut off the filename, keep the slash
  }

  // Combine paths and resolve
  size_t full_len = strlen(binary_dir) + strlen(source_path) + 1;
  char *combined = malloc(full_len);
  if (!combined) {
    free(binary_dir);
    return NULL;
  }

  int res = snprintf(combined, full_len, "%s%s", binary_dir, source_path);
  ASSERT(res > 0, "snprintf failed.");
  free(binary_dir);

  // Use realpath to resolve the absolute path
  char *absolute = realpath(combined, NULL);
  free(combined);

  return absolute ? absolute : strdup(source_path);  // Fall back to original if realpath fails
}

/// @brief Gets the source code file from the binary file and offset
/// @param binary_path Binary file path
/// @param info Debug info struct containing the debug info
/// @param addr Offset into binary file
/// @return Source file information such as filename and line number
source_file_info_t *get_source_info(char *binary_path, binary_info_t *info, uint64_t addr) {
  if (!info || !info->dwfl) {
    return NULL;
  }

  Dwfl_Module *module = dwfl_addrmodule(info->dwfl, addr);
  if (!module) {
    return NULL;
  }

  Dwfl_Line *line_info = dwfl_getsrc(info->dwfl, addr);
  if (!line_info) {
    return NULL;
  }

  int line_number;
  const char *relative_path = dwfl_lineinfo(line_info, NULL, &line_number, NULL, NULL, NULL);
  if (!relative_path) {
    return NULL;
  }

  // Allocate the structure
  source_file_info_t *source_info = malloc(sizeof(source_file_info_t));
  if (!source_info) {
    return NULL;
  }

  // Get absolute path and store line number
  source_info->filename = get_absolute_source_path(binary_path, relative_path);
  if (!source_info->filename) {
    free(source_info);
    return NULL;
  }
  source_info->line_number = line_number;

  return source_info;
}

/// @brief Converts a source file and target line into a line of code
/// @param filename Source filename
/// @param target_line Line to get code of
/// @return Line of code at location
char *get_line_at_line_number(const char *filename, int target_line) {
  FILE *file = fopen(filename, "r");
  if (!file) {
    return NULL;
  }

  char *line = NULL;
  size_t len = 0;
  ssize_t read;
  int current_line = 1;

  // Skip to target line
  while (current_line < target_line) {
    read = getline(&line, &len, file);
    if (read == -1) {
      free(line);
      fclose(file);
      return NULL;
    }
    current_line++;
  }

  // Read target line
  read = getline(&line, &len, file);
  fclose(file);

  if (read == -1) {
    free(line);
    return NULL;
  }

  // Remove newline if present
  if (read > 0 && line[read - 1] == '\n') {
    line[read - 1] = '\0';
  }

  // Allocate new string with space for quotes and null terminator
  // Add quotes so that in CSV representation commas are not an issue
  size_t quoted_len = strlen(line) + 3;  // +2 for quotes, +1 for null
  char *quoted_line = malloc(quoted_len);
  if (!quoted_line) {
    free(line);
    return NULL;
  }

  int res = snprintf(quoted_line, quoted_len, "\"%s\"", line);
  ASSERT(res > 0, "snprintf failed.");
  free(line);

  return quoted_line;
}

/// @brief Gets all the debug info (function, asm, source file, line number) for
/// a filename and offset
/// @param filename Filename to get info of
/// @param offset Offset into file
/// @return Populated pointer to debug_info_t struct
debug_info_t *get_debug_info(char *filename, uint64_t offset) {
  binary_info_t *info = get_fname_binary_map_entry(filename);
  debug_info_t *dinfo = malloc(sizeof(debug_info_t));
  ASSERT(dinfo != NULL, "Failed to create debug info struct.");
  memset(dinfo, 0, sizeof(debug_info_t));

  if (info) {
    const source_file_info_t *source = get_source_info(filename, info, offset);
    if (source) {
      dinfo->src_file = strdup(source->filename);
      dinfo->line_num = source->line_number;
      dinfo->line = get_line_at_line_number(source->filename, source->line_number);
    } else {
      dinfo->src_file = strdup("(null)");
      dinfo->line = strdup("(null)");
      dinfo->line_num = 0;
    }

    char *function = get_function_name(info, offset);
    if (function) {
      dinfo->function = function;
    } else {
      dinfo->function = strdup("\"(null)\"");
    }

    char *assembly = get_assembly(info, offset);
    if (assembly) {
      dinfo->assembly = assembly;
    } else {
      dinfo->assembly = strdup("(null)");
    }
  }
  return dinfo;
}