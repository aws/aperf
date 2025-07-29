#include "fname_map.h"

#include <ctype.h>
#include <dirent.h>
#include <stdio.h>
#include <string.h>

#include "log.h"
#include "sys.h"
#include "vec.h"

struct btree *fname_map = NULL;
const filename_entry_t *cached_entry[CACHE_DEPTH] = {NULL};

/// @brief B-Tree compare function for FNAME_MAP structs
/// @param a First entry to compare
/// @param b Second entry to compare
/// @param udata Unused
/// @return 1 if a > b, -1 if a < b, 0 if a = b
int fname_compare(const void *a, const void *b, void *udata) {
  (void)udata;
  const filename_entry_t *ua = a;
  const filename_entry_t *ub = b;

  if (ua->pid < ub->pid) return -1;
  if (ua->pid > ub->pid) return 1;

  return 0;
}

/// @brief Perf does not emit MMAP2 records for already running processes.
/// We will read through /proc/.../maps and get the virtual address mappings.
void insert_initial_mappings() {
  DIR *proc_dir;
  struct dirent *pid_entry;

  proc_dir = opendir("/proc");
  ASSERT(proc_dir != NULL, "Unable to open /proc.");

  while ((pid_entry = readdir(proc_dir))) {
    if (!isdigit(pid_entry->d_name[0])) continue;

    char maps_path[256];
    int res = snprintf(maps_path, sizeof(maps_path), "/proc/%.230s/maps", pid_entry->d_name);
    ASSERT(res > 0, "snprintf failed.");

    FILE *maps = fopen(maps_path, "r");
    if (maps == NULL) continue;

    pid_t pid = (pid_t)atol(pid_entry->d_name);
    ASSERT(pid != 0, "0 PID detected in /proc");
    char line[4096];

    while (fgets(line, sizeof(line), maps)) {
      unsigned long start, end;
      unsigned long offset;
      char perms[5];
      char path[4096] = "";

      int res =
          sscanf(line, "%lx-%lx %4s %lx %*x:%*x %*u %4095s", &start, &end, perms, &offset, path);

      // Some /proc mappings are anonymous, and do not have a file. Instead of
      // crashing, we ignore them by ensuring path[0] is not null.
      ASSERT(res >= 4 && res <= 5, "Incorrectly read from /proc/.");

      if (path[0]) {
        size_t filename_len = strlen(path);
        size_t total_size = sizeof(mmap2_record_t) + filename_len + 1;

        // Allocate continuous memory for record + filename
        mmap2_record_t *record = malloc(total_size);
        memset(record, 0, total_size);

        record->header.type = PERF_RECORD_MMAP2;
        record->header.size = total_size;
        record->pid = pid;
        record->addr = start;
        record->len = end - start;
        record->pgoff = offset;

        // Now copy filename to the space after the record
        memcpy(record->filename, path, filename_len + 1);

        finode_t finode;
        get_file_info(record->filename, &finode);
        record->ino = finode.ino;
        record->maj = finode.maj;
        record->min = finode.min;
        record->ino_generation = finode.ino_generation;

        insert_finode_entry(record);
        insert_fname_entry(record);

        free(record);
      }
    }
    fclose(maps);
  }
}

/// @brief Initializes FNAME_MAP data structures
void init_fname_map() {
  fname_map = btree_new(sizeof(filename_entry_t), 0, fname_compare, NULL);
  btree_clear(fname_map);

  insert_initial_mappings();
}

/// @brief Inserts a new MMAP2 record into FNAME_MAP.
/// @param record Record to insert
void insert_fname_entry(mmap2_record_t *record) {
  if (!record) return;
  ASSERT(record != NULL, "Input MMAP2 record is NULL.");
  filename_entry_t key = {.pid = record->pid};

  filename_entry_t *entry = (filename_entry_t *)btree_get(fname_map, &key);

  // If the key does not exist (NULL), set up a new key, and allocate a new
  // vector for the MMAP data
  if (entry == NULL) {
    entry = &key;
    entry->virtual_address_map = vector_create();
    btree_set(fname_map, entry);
    ASSERT(!btree_oom(fname_map), "Fname-Map OOM.");
  }

  // After that, when it is guaranteed an entry exists, extract it
  // and insert the new offset mapping into the vector

  pid_virtual_map_entry_t *virtual_entry = malloc(sizeof(pid_virtual_map_entry_t));
  virtual_entry->start = record->addr;
  virtual_entry->end = record->addr + record->len;
  virtual_entry->pgoff = record->pgoff;
  virtual_entry->finode.ino = record->ino;
  virtual_entry->finode.maj = record->maj;
  virtual_entry->finode.min = record->min;
  virtual_entry->finode.ino_generation = record->ino_generation;

  // Add the value directly to the vector
  vector_add(&entry->virtual_address_map, virtual_entry);
}

void free_filename_entry(filename_entry_t *entry_to_remove) {
  pid_virtual_map_entry_t **vmap = entry_to_remove->virtual_address_map;
  uint64_t vmap_size = vector_size(vmap);

  for (uint64_t j = 0; j < vmap_size; j++) {
    pid_virtual_map_entry_t *ventry = ((pid_virtual_map_entry_t **)vmap)[j];
    free(ventry);
  }

  vector_free(entry_to_remove->virtual_address_map);
  btree_delete(fname_map, entry_to_remove);
}

/// @brief Returns a cached entry or NULL if it doesn't exist.
/// @param entry PID to find
const filename_entry_t *get_filename_cached_entry(pid_t pid) {
  // if (cached_entry && cached_entry->pid == pid) return cached_entry;
  for (int i = 0; i < CACHE_DEPTH; i++) {
    if (cached_entry[i] && cached_entry[i]->pid == pid) return cached_entry[i];
  }
  return NULL;
}

/// @brief Updates the cache with a new entry
/// @param entry Entry to updated
void update_filename_cached_entry(const filename_entry_t *entry) {
  // Shift all entries down, discarding the oldest entry
  for (int i = CACHE_DEPTH - 1; i > 0; i--) {
    cached_entry[i] = cached_entry[i - 1];
  }

  // Put new entry at the front of the cache
  cached_entry[0] = entry;
}

void prune_filename_cache(pid_t pid) {
  for (int i = 0; i < CACHE_DEPTH; i++) {
    if (cached_entry[i] && cached_entry[i]->pid == pid) {
      cached_entry[i] = NULL;
    }
  }
}

/// @brief Removes all virtual offset mappings associated with a PID.
/// @param pid PID to remove mappings for
void remove_fname_entry(pid_t pid) {
  filename_entry_t *entry =
      (filename_entry_t *)btree_get(fname_map, &(filename_entry_t){.pid = pid});
  if (entry != NULL) {
    free_filename_entry(entry);
    prune_filename_cache(pid);
  }
}

/// @brief Converts an instruction pointer (program counter) into a filename and
/// file offset, given
///        the present active PID for the session.
/// @param va VA to convert
/// @param pid Active PID
/// @param filename Passed in to populate filename
/// @param offset Passed in to populate file offset
/// @return -1 on failure to map, 0 on success
bool va_to_file_offset(uint64_t va, pid_t pid, finode_t *finode, uint64_t *offset) {
  if (!finode || !offset) return -1;
  const filename_entry_t *entry = get_filename_cached_entry(pid);
  if (entry == NULL) entry = btree_get(fname_map, &(filename_entry_t){.pid = pid});

  if (entry == NULL || entry->pid != pid) return false;

  update_filename_cached_entry(entry);

  pid_virtual_map_entry_t **vmap = entry->virtual_address_map;
  uint64_t vmap_size = vector_size(vmap);

  for (size_t i = 0; i < vmap_size; i++) {
    pid_virtual_map_entry_t *ventry = ((pid_virtual_map_entry_t **)vmap)[i];

    if (va >= ventry->start && va < ventry->end) {
      finode->ino = ventry->finode.ino;
      finode->maj = ventry->finode.maj;
      finode->min = ventry->finode.min;
      finode->ino_generation = ventry->finode.ino_generation;

      *offset = va - ventry->start + ventry->pgoff;

      return true;
    }
  }

  return false;
}