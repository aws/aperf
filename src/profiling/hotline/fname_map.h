#pragma once

#include <stdint.h>
#include <sys/types.h>

#include "btree.h"
#include "finode_map.h"
#include "perf_interface.h"

#define CACHE_DEPTH 5

/// @brief Virtual address start and end for each MMAP2 record. Will be stored
/// within a B-Tree.
typedef struct pid_virtual_map_entry {
  uint64_t start;  // virtual address start
  uint64_t end;    // virtual address end
  uint64_t pgoff;  // file offset
  // char *filename;
  finode_t finode;
} pid_virtual_map_entry_t;

/// @brief For each filename and pid pair, we will store an array of all the
/// mappings associated with it.
typedef struct filename_entry {
  pid_t pid;
  pid_virtual_map_entry_t **virtual_address_map;  // array of pointers

} filename_entry_t;

/// @brief we can't just use a pointer to virtual_address_map because the B-Tree
/// copies structs over, so our pointer may not point to the actual node in the
/// tree. We copy over the relevant information, and cache it.
typedef struct filename_entry_cache {
  pid_t pid;
  pid_virtual_map_entry_t **virtual_address_map;
  finode_t finode;
} filename_entry_cache_t;

void init_fname_map();
void insert_fname_entry(mmap2_record_t *record);
void remove_fname_entry(pid_t pid);

bool va_to_file_offset(uint64_t va, pid_t pid, finode_t *finode, uint64_t *offset);

int fname_compare(const void *a, const void *b,
                  void *udata);  // exposed for testing

/// @brief Exposed B-Tree structure for all file mappings
extern struct btree *fname_map;