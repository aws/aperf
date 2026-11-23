#pragma once

#include <stdint.h>

#include "btree.h"
#include "finode_map.h"

/// @brief Struct to package latency binning information for completion node
/// data report
typedef struct completion_histogram {
  uint64_t l1_bound_bin;
  uint64_t l2_bound_bin;
  uint64_t l3_bound_bin;
  uint64_t dram_bound_bin;
} completion_histogram_t;

/// @brief B-Tree structure for SPE latency information of load/store
/// instructions
typedef struct lat_map_entry {
  union {  // union allows data storing and report generation to use the same
           // parameters rather than having to redefine them each time.
    finode_t finode;
    char *filename;
  };
  uint64_t offset;
  uint64_t total_latency;
  uint64_t issue_latency;
  uint64_t translation_latency;
  uint64_t saturated;
  uint64_t count;
  union {
    struct {
      completion_histogram_t l1, l2, l3, dram;
    };
    completion_histogram_t histograms[4];  // array access option
  };
} lat_map_entry_t;

void init_lat_map();
void insert_lat_map_entry(lat_map_entry_t *entry);
void parse_lat_map_entry(spe_record_raw_t *record, lat_map_entry_t *entry, finode_t *finode,
                         uint64_t offset);
void parse_and_insert_lat_entry(spe_record_raw_t *record, finode_t *finode, uint64_t offset);

extern struct btree *lat_map;