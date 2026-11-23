#include "finode_map.h"

#include <stdio.h>
#include <string.h>
#include <sys/types.h>

#include "log.h"

struct btree *finode_map = NULL;

/// @brief Comparison function for the file inode structures
/// @param a First finode_map_entry to compare
/// @param b Second finode_map_entry to compare
/// @param udata Unused
/// @return 1 if a > b, -1 if a < b, 0 otherwise
int finode_compare(const void *a, const void *b, void *udata) {
  (void)udata;
  const finode_t *fa = &((const finode_map_entry_t *)a)->finode;
  const finode_t *fb = &((const finode_map_entry_t *)b)->finode;

  // Compare ino first as it's likely to be unique most often
  if (fa->ino != fb->ino) return (fa->ino > fb->ino) ? 1 : -1;

  // Then device numbers
  if (fa->maj != fb->maj) return (fa->maj > fb->maj) ? 1 : -1;

  if (fa->min != fb->min) return (fa->min > fb->min) ? 1 : -1;

  // Finally generation number
  if (fa->ino_generation != fb->ino_generation)
    return (fa->ino_generation > fb->ino_generation) ? 1 : -1;

  return 0;
}

void init_finode_map() {
  finode_map = btree_new(sizeof(finode_map_entry_t), 0, finode_compare, NULL);
  btree_clear(finode_map);
}

void insert_finode_entry(mmap2_record_t *record) {
  finode_map_entry_t entry;
  entry.finode.ino = record->ino;
  entry.finode.maj = record->maj;
  entry.finode.min = record->min;
  entry.finode.ino_generation = record->ino_generation;

  entry.filename = strdup(record->filename);
  btree_set(finode_map, &entry);
  ASSERT(!btree_oom(finode_map), "Finode-Map OOM.");
}