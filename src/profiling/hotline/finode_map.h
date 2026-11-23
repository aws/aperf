#pragma once

#include <stdint.h>

#include "btree.h"
#include "perf_interface.h"

/// @brief Utility structure to package all inode information for a file.
typedef struct finode {
  uint32_t maj;
  uint32_t min;
  uint64_t ino;
  uint64_t ino_generation;
} finode_t;

/// @brief B-Tree structure for mapping inode information to a filename. This
/// was
///  created to avoid string comparisons in the lat_map and bmiss_map data
///  structures.
typedef struct finode_map_entry {
  finode_t finode;
  char *filename;
} finode_map_entry_t;

void init_finode_map();
void insert_finode_entry(mmap2_record_t *record);

extern struct btree *finode_map;