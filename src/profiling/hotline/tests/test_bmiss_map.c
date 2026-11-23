#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "test.h"

// Test init_bmiss_map function
void test_init_bmiss_map() {
  init_bmiss_map();
  assert(bmiss_map != NULL);
}

// Test insert_bmiss_map function
void test_insert_bmiss_map() {
  init_bmiss_map();

  // Test inserting new entry
  bmiss_map_entry_t entry = {0};
  entry.finode.ino = 100;
  entry.finode.maj = 1;
  entry.finode.min = 2;
  entry.finode.ino_generation = 3;
  entry.offset = 1000;
  entry.count = 1;
  entry.mispredicted = 1;
  entry.branch_type = AUX_RECORD_TYPE_BCOND;

  insert_bmiss_map(&entry);

  // Verify entry was inserted
  bmiss_map_entry_t key = {.finode = entry.finode, .offset = entry.offset};
  const bmiss_map_entry_t *result = btree_get(bmiss_map, &key);
  assert(result != NULL);
  assert(result->count == 1);
  assert(result->mispredicted == 1);
  assert(result->branch_type == AUX_RECORD_TYPE_BCOND);

  // Test updating existing entry
  bmiss_map_entry_t update_entry = entry;
  update_entry.count = 1;
  update_entry.mispredicted = 0;

  insert_bmiss_map(&update_entry);

  // Verify entry was updated (aggregated)
  result = btree_get(bmiss_map, &key);
  assert(result != NULL);
  assert(result->count == 2);         // 1 + 1
  assert(result->mispredicted == 1);  // 1 + 0

  // Verify original entry was not affected
  result = btree_get(bmiss_map, &key);
  assert(result != NULL);
}

// Test parse_bmiss_map_entry function
void test_parse_bmiss_map_entry() {
  // Test normal record parsing
  spe_record_raw_t record = {0};
  record.total_lat = 100;
  record.issue_lat = 60;
  record.events_packet = AUX_EVENT_BRANCH_MISS;
  record.type = AUX_RECORD_TYPE_BCOND;

  finode_t finode = {.ino = 200, .maj = 3, .min = 4, .ino_generation = 5};
  uint64_t offset = 2000;

  bmiss_map_entry_t entry = {0};
  parse_bmiss_map_entry(&record, &entry, &finode, offset);

  assert(entry.finode.ino == 200);
  assert(entry.finode.maj == 3);
  assert(entry.finode.min == 4);
  assert(entry.finode.ino_generation == 5);
  assert(entry.offset == 2000);
  assert(entry.count == 1);
  assert(entry.branch_type == AUX_RECORD_TYPE_BCOND);

  // Test saturated record parsing
  spe_record_raw_t saturated_record = {0};
  saturated_record.issue_lat = AUX_PACKET_SATURATED;
  saturated_record.total_lat = 200;
  saturated_record.events_packet = AUX_EVENT_BRANCH_NOT_TAKEN;

  bmiss_map_entry_t saturated_entry = {0};
  parse_bmiss_map_entry(&saturated_record, &saturated_entry, &finode, offset);

  assert(saturated_entry.mispredicted == 0);  // Should not update when saturated

  // Test branch not taken parsing
  spe_record_raw_t not_taken_record = {0};
  not_taken_record.issue_lat = 40;
  not_taken_record.total_lat = 80;
  not_taken_record.events_packet = AUX_EVENT_BRANCH_NOT_TAKEN;

  bmiss_map_entry_t not_taken_entry = {0};
  parse_bmiss_map_entry(&not_taken_record, &not_taken_entry, &finode, offset);

  assert(not_taken_entry.mispredicted == 0);
  assert(not_taken_entry.count == 1);
}

// Test integration scenario
void test_bmiss_integration() {
  init_bmiss_map();

  // Simulate processing multiple SPE records for the same location
  spe_record_raw_t record1 = {0};
  record1.total_lat = 50;
  record1.issue_lat = 30;
  record1.type = AUX_RECORD_TYPE_BCOND;

  spe_record_raw_t record2 = {0};
  record2.total_lat = 70;
  record2.issue_lat = 40;
  record2.events_packet = AUX_EVENT_BRANCH_MISS;
  record2.type = AUX_RECORD_TYPE_BCOND;

  finode_t finode = {.ino = 300, .maj = 5, .min = 6, .ino_generation = 7};
  uint64_t offset = 3000;

  // Parse and insert first record
  bmiss_map_entry_t entry1 = {0};
  parse_bmiss_map_entry(&record1, &entry1, &finode, offset);
  insert_bmiss_map(&entry1);

  // Parse and insert second record (same location)
  bmiss_map_entry_t entry2 = {0};
  parse_bmiss_map_entry(&record2, &entry2, &finode, offset);
  insert_bmiss_map(&entry2);

  // Verify aggregated results
  bmiss_map_entry_t key = {.finode = finode, .offset = offset};
  const bmiss_map_entry_t *result = btree_get(bmiss_map, &key);
  assert(result != NULL);
  assert(result->count == 2);         // 1 + 0
  assert(result->mispredicted == 1);  // 0 + 1
}

void test_bmiss_map() {
  // test_bmiss_map_compare();
  test_init_bmiss_map();
  test_insert_bmiss_map();
  test_parse_bmiss_map_entry();
  test_bmiss_integration();
}
