#pragma once

#include <stdint.h>

#include "btree.h"
#include "finode_map.h"
#include "perf_interface.h"

/// @brief B-Tree struct definition for the SPE branch data.
typedef struct bmiss_map_entry {
  union {  // union allows data storing and report generation to use the same
           // parameters rather than having to redefine them each time.
    finode_t finode;
    char *filename;
  };
  uint64_t offset;
  uint64_t count;
  uint64_t mispredicted;
  uint8_t branch_type;
} bmiss_map_entry_t;

void init_bmiss_map();
void insert_bmiss_map(bmiss_map_entry_t *entry_to_insert);
void parse_bmiss_map_entry(spe_record_raw_t *record, bmiss_map_entry_t *entry, finode_t *finode,
                           uint64_t offset);
void parse_and_insert_bmiss_entry(spe_record_raw_t *record, finode_t *finode, uint64_t offset);
extern struct btree *bmiss_map;