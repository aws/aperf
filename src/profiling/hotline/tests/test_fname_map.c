#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "test.h"

void test_init_fname_map() {
  // Don't actually init it because we don't want to populate it with /proc.
  // This will synthetically init the B-Tree for testing.
  fname_map = btree_new(sizeof(filename_entry_t), 0, fname_compare, NULL);
  btree_clear(fname_map);
  assert(fname_map != NULL);
}

void test_insert_fname_entry() {
  test_init_fname_map();

  // Create mock MMAP2 record
  const char *test_filename = "/test/binary";
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
  record->ino = 100;
  record->maj = 8;
  record->min = 1;
  record->ino_generation = 1;

  // Copy the filename into the flexible array member
  strcpy(record->filename, test_filename);

  // Use the record...
  insert_fname_entry(record);

  // Free the allocated memory when done
  free(record);

  // Verify entry exists
  filename_entry_t key = {.pid = 1234};
  const filename_entry_t *entry = btree_get(fname_map, &key);
  assert(entry != NULL);
  assert(entry->pid == 1234);
  assert(entry->virtual_address_map != NULL);
}

void test_va_to_file_offset() {
  test_init_fname_map();

  // Insert test mapping
  size_t filename_len = strlen("/test/lib.so") + 1;  // +1 for null terminator
  size_t record_size = sizeof(mmap2_record_t) + filename_len;

  mmap2_record_t *record = malloc(record_size);
  if (record == NULL) {
    fprintf(stderr, "Memory allocation failed\n");
    return;
  }

  // Initialize the structure
  memset(record, 0, record_size);
  record->pid = 5678;
  record->addr = 0x400000;
  record->len = 0x2000;
  record->pgoff = 0;
  record->ino = 100;
  record->maj = 8;
  record->min = 1;
  record->ino_generation = 1;

  // Copy the filename into the flexible array member
  strcpy(record->filename, "/test/lib.so");

  insert_fname_entry(record);

  // Test successful mapping
  finode_t finode;
  uint64_t offset;
  int result = va_to_file_offset(0x401000, 5678, &finode, &offset);

  assert(result == 0);
  assert(finode.ino == 100);
  assert(finode.maj == 8);
  assert(finode.min == 1);
  assert(offset == 0x1000);  // 0x401000 - 0x400000 + 0x1000

  // Test address outside range
  result = va_to_file_offset(0x500000, 5678, &finode, &offset);
  assert(result == -1);

  // Test non-existent PID
  result = va_to_file_offset(0x401000, 9999, &finode, &offset);
  assert(result == -1);
}

void test_remove_fname_entry() {
  test_init_fname_map();

  // Insert test mapping
  size_t filename_len = strlen("/test/lib.so") + 1;  // +1 for null terminator
  size_t record_size = sizeof(mmap2_record_t) + filename_len;

  mmap2_record_t *record = malloc(record_size);
  if (record == NULL) {
    fprintf(stderr, "Memory allocation failed\n");
    return;
  }

  // Initialize the structure
  memset(record, 0, record_size);
  record->pid = 5678;
  record->addr = 0x400000;
  record->len = 0x2000;
  record->pgoff = 0;
  record->ino = 100;
  record->maj = 8;
  record->min = 1;
  record->ino_generation = 1;

  // Copy the filename into the flexible array member
  strcpy(record->filename, "/test/lib.so");

  insert_fname_entry(record);

  // Verify entry exists
  filename_entry_t key = {.pid = 5678};
  const filename_entry_t *entry = btree_get(fname_map, &key);
  assert(entry != NULL);

  // Remove entry
  remove_fname_entry(5678);

  // Verify entry is gone
  entry = btree_get(fname_map, &key);
  assert(entry == NULL);
}

void test_cache_functionality() {
  test_init_fname_map();

  // Insert test mapping
  size_t filename_len = strlen("/test/lib.so") + 1;  // +1 for null terminator
  size_t record_size = sizeof(mmap2_record_t) + filename_len;

  mmap2_record_t *record = malloc(record_size);
  if (record == NULL) {
    fprintf(stderr, "Memory allocation failed\n");
    return;
  }

  // Initialize the structure
  memset(record, 0, record_size);
  record->pid = 2222;
  record->addr = 0x400000;
  record->len = 0x9000;
  record->pgoff = 0;
  record->ino = 100;
  record->maj = 8;
  record->min = 1;
  record->ino_generation = 1;

  // Copy the filename into the flexible array member
  strcpy(record->filename, "/test/lib.so");

  insert_fname_entry(record);

  // First lookup should cache the entry
  finode_t finode;
  uint64_t offset;
  int result = va_to_file_offset(0x400500, 2222, &finode, &offset);
  assert(result == 0);

  // Second lookup should use cache
  result = va_to_file_offset(0x400600, 2222, &finode, &offset);
  assert(result == 0);
  assert(offset == 0x600);
}

void test_fname_map() {
  test_init_fname_map();
  test_insert_fname_entry();
  test_va_to_file_offset();
  test_remove_fname_entry();
  test_cache_functionality();
}
