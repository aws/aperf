#include "lat_map.h"

#include <string.h>

#include "log.h"
#include "perf_interface.h"
#include "sys.h"

struct btree *lat_map = NULL;

/// @brief B-Tree comparison function for `lat_map_entry_t`
/// @param a First entry to compare
/// @param b Second entry to compare
/// @param udata Unused
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int lat_map_compare(const void *a, const void *b, void *udata) {
  (void)udata;
  const lat_map_entry_t *ua = a;
  const lat_map_entry_t *ub = b;

  // Compare ino first as it's likely to be unique most often
  if (ua->finode.ino > ub->finode.ino)
    return 1;
  else if (ua->finode.ino < ub->finode.ino)
    return -1;

  // then check offset as it should never be the same between two lat_map_entry_ts with the same
  // file
  if (ua->offset > ub->offset)
    return 1;
  else if (ua->offset < ub->offset)
    return -1;

  // then check the device associated info
  if (ua->finode.maj > ub->finode.maj)
    return 1;
  else if (ua->finode.maj < ub->finode.maj)
    return -1;

  if (ua->finode.min > ub->finode.min)
    return 1;
  else if (ua->finode.min < ub->finode.min)
    return -1;

  // this should rarely change so check if last
  if (ua->finode.ino_generation > ub->finode.ino_generation)
    return 1;
  else if (ua->finode.ino_generation < ub->finode.ino_generation)
    return -1;

  return 0;
}

/// @brief Initializes the lat_map
void init_lat_map() {
  lat_map = btree_new(sizeof(lat_map_entry_t), 0, lat_map_compare, NULL);
  btree_clear(lat_map);
}

static inline void update_histogram(completion_histogram_t *dst, const completion_histogram_t *src1,
                                    const completion_histogram_t *src2) {
  dst->l1_bound_bin = src1->l1_bound_bin + src2->l1_bound_bin;
  dst->l2_bound_bin = src1->l2_bound_bin + src2->l2_bound_bin;
  dst->l3_bound_bin = src1->l3_bound_bin + src2->l3_bound_bin;
  dst->dram_bound_bin = src1->dram_bound_bin + src2->dram_bound_bin;
}

/// @brief Inserts a `lat_map_entry_t` into the lat_map
/// @param entry_to_insert Entry to insert
inline void insert_lat_map_entry(lat_map_entry_t *entry_to_insert) {
  if (!entry_to_insert) return;
  lat_map_entry_t tmp_entry = {0};

  lat_map_entry_t *entry = (lat_map_entry_t *)btree_get(lat_map, entry_to_insert);

  if (entry == NULL) {
    entry = &tmp_entry;
    entry->finode = entry_to_insert->finode;
    entry->offset = entry_to_insert->offset;
  }

  lat_map_entry_t updated_entry = {0};  // copy over existing + new stats into a
                                        // new entry, then call btree_set
                                        // we need to btree_get again because
                                        // the btree data structure copies over
                                        // and malloc's it's own internal struct
  updated_entry.finode = entry->finode;
  updated_entry.offset = entry->offset;

  updated_entry.total_latency = entry->total_latency + entry_to_insert->total_latency;
  updated_entry.issue_latency = entry->issue_latency + entry_to_insert->issue_latency;
  updated_entry.translation_latency =
      entry->translation_latency + entry_to_insert->translation_latency;
  updated_entry.saturated = entry->saturated + entry_to_insert->saturated;
  updated_entry.count = entry->count + entry_to_insert->count;

  for (int i = 0; i < 4; i++) {
    update_histogram(&updated_entry.histograms[i], &entry->histograms[i],
                     &entry_to_insert->histograms[i]);
  }

  btree_set(lat_map, &updated_entry);
  ASSERT(!btree_oom(lat_map), "Lat Map OOM.");
}

/// @brief Parses a raw SPE entry into a struct that lat_map can use
/// @param record Raw record to parse
/// @param entry lat_map struct to parse into
/// @param filename Filename to associate, decoded from `pc_to_file_offset`
/// @param offset File offset to associate, decoded from `pc_to_file_offset`
inline void parse_lat_map_entry(spe_record_raw_t *record, lat_map_entry_t *entry, finode_t *finode,
                                uint64_t offset) {
  if (!record || !entry || !finode) return;
  entry->saturated = (record->issue_lat == AUX_PACKET_SATURATED) ? 1 : 0;
  entry->count = 1;
  entry->finode.ino = finode->ino;
  entry->finode.maj = finode->maj;
  entry->finode.min = finode->min;
  entry->finode.ino_generation = finode->ino_generation;
  entry->offset = offset;

  // don't update statistics if saturated
  if (entry->saturated) return;

  entry->total_latency = record->total_lat * cpu_system_config.cyc_to_ps_conv_factor;
  entry->issue_latency = record->issue_lat * cpu_system_config.cyc_to_ps_conv_factor;
  entry->translation_latency = record->x_lat * cpu_system_config.cyc_to_ps_conv_factor;

  // determine which bin we need to update based on data source
  completion_histogram_t *bin;
  switch (record->data_source) {
    case DATA_SOURCE_L1:
      bin = &entry->l1;
      break;
    case DATA_SOURCE_L2:
      bin = &entry->l2;
      break;
    case DATA_SOURCE_LOCAL_CLUSTER:
    case DATA_SOURCE_PEER_CLUSTER:
    case DATA_SOURCE_SYSTEM_CACHE:
      bin = &entry->l3;
      break;
    default:
      bin = &entry->dram;
  }

  uint64_t execution_latency =
      entry->total_latency - entry->issue_latency - entry->translation_latency;

  if (execution_latency <= cpu_system_config.latency_limits.l1_latency_cap_ps)
    bin->l1_bound_bin = 1;
  else if (execution_latency <= cpu_system_config.latency_limits.l2_latency_cap_ps)
    bin->l2_bound_bin = 1;
  else if (execution_latency <= cpu_system_config.latency_limits.l3_latency_cap_ps)
    bin->l3_bound_bin = 1;
  else
    bin->dram_bound_bin = 1;
}

/// @brief Combines parse and insert with inline because these are called very
/// frequently.
/// @param record Record to parse and insert
/// @param finode Finode of file associated with a virtual address
/// @param offset Offset into the file
void parse_and_insert_lat_entry(spe_record_raw_t *record, finode_t *finode, uint64_t offset) {
  lat_map_entry_t lat_entry = {0};
  // parse_lat_map_entry and insert_lat_map are inlined for potential
  // compiler optimizations
  parse_lat_map_entry(record, &lat_entry, finode, offset);
  insert_lat_map_entry(&lat_entry);
}