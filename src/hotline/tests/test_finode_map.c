#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "test.h"

void test_init_finode_map() {
  init_finode_map();
  assert(finode_map != NULL);
}

void test_insert_finode_entry() {
  init_finode_map();

  const char *test_filename = "/test/file";
  size_t filename_len = strlen(test_filename) + 1;  // +1 for null terminator
  size_t record_size = sizeof(mmap2_record_t) + filename_len;

  // Allocate memory for the entire structure including the flexible array
  // member
  mmap2_record_t *record = malloc(record_size);
  if (record == NULL) {
    fprintf(stderr, "Memory allocation failed\n");
    return;
  }

  // Initialize the structure
  memset(record, 0, record_size);
  record->pid = 1234;
  record->addr = 0x400000;
  record->len = 0x1000;
  record->pgoff = 0;
  record->ino = 123;
  record->maj = 8;
  record->min = 1;
  record->ino_generation = 1;

  strcpy(record->filename, test_filename);

  insert_finode_entry(record);

  // Verify entry exists
  finode_map_entry_t key = {{.ino = 123, .maj = 8, .min = 1, .ino_generation = 1}, NULL};
  const finode_map_entry_t *entry = btree_get(finode_map, &key);
  assert(entry != NULL);
  assert(entry->finode.ino == 123);
  assert(strcmp(entry->filename, "/test/file") == 0);
}

void test_finode_map() {
  test_init_finode_map();
  test_insert_finode_entry();
}