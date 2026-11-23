#pragma once

#include <capstone/capstone.h>
#include <dwarf.h>
#include <elf.h>
#include <elfutils/libdw.h>
#include <elfutils/libdwfl.h>
#include <unistd.h>

#include "btree.h"

/// @brief Open session information for a binary file
typedef struct binary_info {
  void *map;
  size_t size;
  Elf64_Ehdr *ehdr;
  Elf64_Shdr *shdr;
  char *shstrtab;
  Elf64_Sym *symtab;
  char *strtab;
  int sym_count;
  void *text_section;
  uint64_t text_addr;
  size_t text_size;
  Dwfl *dwfl;
  csh cs_handle;
} binary_info_t;

/// @brief There are often many offsets associated with the same file. Setting
/// up and reading into a file is expensive, so we can first check if we have
/// already set up this file first by indexing this B-Tree.
typedef struct fname_binary_map_entry {
  char *filename;
  binary_info_t *binary_info;
} fname_binary_map_entry_t;

typedef struct source_file_info {
  char *filename;
  uint64_t line_number;
} source_file_info_t;

typedef struct debug_info {
  char *src_file;
  uint64_t line_num;
  char *line;
  char *assembly;
  char *function;
} debug_info_t;

void init_fname_binary_btree();
binary_info_t *get_fname_binary_map_entry(char *filename);

debug_info_t *get_debug_info(char *filename, uint64_t offset);

char *get_absolute_source_path(const char *binary_path, const char *source_path);
char *get_line_at_line_number(const char *filename, int target_line);