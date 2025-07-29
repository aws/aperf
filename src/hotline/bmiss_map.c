#include "bmiss_map.h"

#include <string.h>

#include "log.h"

struct btree *bmiss_map = NULL;

/// @brief B-Tree function to compare the branch miss map entries. Order is
/// compared with ino first as
///        it is most likely to vary, then maj, min, and ino_generation (which
///        should rarely change).
/// @param a First element to compare
/// @param b Second element to compare
/// @param udata Unused
/// @return 1 if a > b, -1 if a < b, 0 if equal
int bmiss_map_compare(const void *a, const void *b, void *udata) {
  (void)udata;
  const bmiss_map_entry_t *ua = a;
  const bmiss_map_entry_t *ub = b;

  // Compare ino first as it's likely to be unique most often
  if (ua->finode.ino > ub->finode.ino)
    return 1;
  else if (ua->finode.ino < ub->finode.ino)
    return -1;

  // then check offset as it should never be the same between two lat_map_entry_ts with the same
  // file
  if (ua->offset < ub->offset)
    return -1;
  else if (ua->offset > ub->offset)
    return 1;

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

/// @brief Initializes the BMISS_MAP
void init_bmiss_map() {
  bmiss_map = btree_new(sizeof(bmiss_map_entry_t), 0, bmiss_map_compare, NULL);
  btree_clear(bmiss_map);
}

/// @brief Inserts a bmiss_map_entry_t into the BMISS_MAP
/// @param entry_to_insert Entry to insert
inline void insert_bmiss_map(bmiss_map_entry_t *entry_to_insert) {
  ASSERT(entry_to_insert, "Entry to insert is NULL.");

  bmiss_map_entry_t tmp_entry = {0};

  bmiss_map_entry_t *entry = (bmiss_map_entry_t *)btree_get(bmiss_map, entry_to_insert);

  if (entry == NULL) {
    entry = &tmp_entry;
    entry->finode = entry_to_insert->finode;
    entry->offset = entry_to_insert->offset;
  }

  bmiss_map_entry_t updated_entry = {0};

  updated_entry.finode = entry->finode;
  updated_entry.offset = entry->offset;

  updated_entry.count = entry->count + entry_to_insert->count;
  updated_entry.mispredicted = entry->mispredicted + entry_to_insert->mispredicted;
  updated_entry.branch_type = entry_to_insert->branch_type;

  btree_set(bmiss_map, &updated_entry);
  ASSERT(!btree_oom(bmiss_map), "B-Miss Map OOM.");
}

/// @brief Parses the raw SPE record into the format the BMISS_MAP uses
/// @param record Raw SPE record
/// @param entry B-Tree entry to populate
/// @param filename filename to assign into the entry, decoded from
/// `pc_to_file_offset`
/// @param offset offset to assign into the entry, decoded from
/// `pc_to_file_offset`
inline void parse_bmiss_map_entry(spe_record_raw_t *record, bmiss_map_entry_t *entry,
                                  finode_t *finode, uint64_t offset) {
  ASSERT(entry && record && finode, "Invalid arguments to parse.");

  memset(entry, 0, sizeof(bmiss_map_entry_t));

  entry->count = 1;
  entry->finode.ino = finode->ino;
  entry->finode.maj = finode->maj;
  entry->finode.min = finode->min;
  entry->finode.ino_generation = finode->ino_generation;
  entry->offset = offset;

  entry->mispredicted = (record->events_packet & AUX_EVENT_BRANCH_MISS) ? 1 : 0;
  entry->branch_type = record->type;
}

/// @brief Combines parse and insert with inline because these are called very
/// frequently.
/// @param record Record to parse and insert
/// @param finode Finode of file associated with a virtual address
/// @param offset Offset into the file
void parse_and_insert_bmiss_entry(spe_record_raw_t *record, finode_t *finode, uint64_t offset) {
  bmiss_map_entry_t bmiss_entry = {0};
  // parse_bmiss_map_entry and insert_bmiss_map are inlined for potential
  // compiler optimizations
  parse_bmiss_map_entry(record, &bmiss_entry, finode, offset);
  insert_bmiss_map(&bmiss_entry);
}