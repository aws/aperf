#include <assert.h>
#include <stdint.h>
#include <stdio.h>
#include <string.h>

#include "test.h"

// Test init_lat_map function
void test_init_lat_map() {
  cpu_system_config.latency_limits.l1_latency_cap_ps = 10;
  cpu_system_config.latency_limits.l2_latency_cap_ps = 50;
  cpu_system_config.latency_limits.l3_latency_cap_ps = 200;
  cpu_system_config.cyc_to_ps_conv_factor = 1;

  init_lat_map();
  assert(lat_map != NULL);
}

// Test insert_lat_map_entry function
void test_insert_lat_map_entry() {
  init_lat_map();

  // Test inserting new entry
  lat_map_entry_t entry = {0};
  entry.finode.ino = 100;
  entry.finode.maj = 1;
  entry.finode.min = 2;
  entry.finode.ino_generation = 3;
  entry.offset = 1000;
  entry.total_latency = 100;
  entry.issue_latency = 60;
  entry.translation_latency = 20;
  entry.saturated = 1;
  entry.count = 1;
  entry.l1.l1_bound_bin = 5;
  entry.l2.l2_bound_bin = 3;

  insert_lat_map_entry(&entry);

  // Verify entry was inserted
  lat_map_entry_t key = {.finode = entry.finode, .offset = entry.offset};
  const lat_map_entry_t *result = btree_get(lat_map, &key);
  assert(result != NULL);
  assert(result->total_latency == 100);
  assert(result->issue_latency == 60);
  assert(result->translation_latency == 20);
  assert(result->saturated == 1);
  assert(result->count == 1);
  assert(result->l1.l1_bound_bin == 5);
  assert(result->l2.l2_bound_bin == 3);

  // Test updating existing entry
  lat_map_entry_t update_entry = entry;
  update_entry.total_latency = 50;
  update_entry.issue_latency = 30;
  update_entry.translation_latency = 10;
  update_entry.saturated = 2;
  update_entry.count = 1;
  update_entry.l1.l1_bound_bin = 2;
  update_entry.l2.l2_bound_bin = 1;

  insert_lat_map_entry(&update_entry);

  // Verify entry was updated (aggregated)
  result = btree_get(lat_map, &key);
  assert(result != NULL);
  assert(result->total_latency == 150);       // 100 + 50
  assert(result->issue_latency == 90);        // 60 + 30
  assert(result->translation_latency == 30);  // 20 + 10
  assert(result->saturated == 3);             // 1 + 2
  assert(result->count == 2);                 // 1 + 1
  assert(result->l1.l1_bound_bin == 7);       // 5 + 2
  assert(result->l2.l2_bound_bin == 4);       // 3 + 1
}

// Test parse_lat_map_entry function
void test_parse_lat_map_entry() {
  // Test NULL inputs
  finode_t finode = {.ino = 200, .maj = 3, .min = 4, .ino_generation = 5};
  uint64_t offset = 2000;
  lat_map_entry_t entry = {0};
  spe_record_raw_t record = {0};

  parse_lat_map_entry(NULL, &entry, &finode, offset);
  assert(entry.total_latency == 0);

  parse_lat_map_entry(&record, NULL, &finode, offset);
  parse_lat_map_entry(&record, &entry, NULL, offset);

  // Test L1 data source with L1-bound latency
  record.total_lat = 100;
  record.issue_lat = 60;
  record.x_lat = 20;
  record.data_source = DATA_SOURCE_L1;
  record.events_packet = AUX_EVENT_RETIRED;

  parse_lat_map_entry(&record, &entry, &finode, offset);

  assert(entry.finode.ino == 200);
  assert(entry.total_latency == 100);
  assert(entry.issue_latency == 60);
  assert(entry.translation_latency == 20);
  assert(entry.saturated == 0);
  assert(entry.count == 1);
  assert(entry.l1.l1_bound_bin == 0);  // execution_latency = 20, <= 10 is false, <= 50 is true
  assert(entry.l1.l2_bound_bin == 1);

  // Test DRAM data source with DRAM-bound latency
  spe_record_raw_t dram_record = {0};
  dram_record.total_lat = 500;
  dram_record.issue_lat = 50;
  dram_record.x_lat = 30;
  dram_record.data_source = DATA_SOURCE_DRAM;

  lat_map_entry_t dram_entry = {0};
  parse_lat_map_entry(&dram_record, &dram_entry, &finode, offset);

  assert(dram_entry.dram.dram_bound_bin == 1);  // execution_latency = 420, > 200
  assert(dram_entry.dram.l1_bound_bin == 0);
  assert(dram_entry.dram.l2_bound_bin == 0);
  assert(dram_entry.dram.l3_bound_bin == 0);

  // Test saturated record
  spe_record_raw_t saturated_record = {0};
  saturated_record.issue_lat = AUX_PACKET_SATURATED;
  saturated_record.total_lat = 200;

  lat_map_entry_t saturated_entry = {0};
  parse_lat_map_entry(&saturated_record, &saturated_entry, &finode, offset);

  assert(saturated_entry.saturated == 1);
  assert(saturated_entry.total_latency == 0);  // Should not update when saturated
  assert(saturated_entry.issue_latency == 0);

  // Test L3 data source (system cache)
  spe_record_raw_t l3_record = {0};
  l3_record.total_lat = 150;
  l3_record.issue_lat = 40;
  l3_record.x_lat = 10;
  l3_record.data_source = DATA_SOURCE_SYSTEM_CACHE;

  lat_map_entry_t l3_entry = {0};
  parse_lat_map_entry(&l3_record, &l3_entry, &finode, offset);

  assert(l3_entry.l3.l3_bound_bin == 1);  // execution_latency = 100, <= 200 but > 50

  // Test invalid data source
  spe_record_raw_t invalid_record = {0};
  invalid_record.data_source = 99;  // Invalid value
  lat_map_entry_t invalid_entry = {0};
  parse_lat_map_entry(&invalid_record, &invalid_entry, &finode, offset);
  assert(invalid_entry.total_latency == 0);
}

// Test integration scenario
void test_lat_integration() {
  init_lat_map();

  // Simulate processing multiple SPE records for the same location
  spe_record_raw_t record1 = {0};
  record1.total_lat = 80;
  record1.issue_lat = 40;
  record1.x_lat = 10;
  record1.data_source = DATA_SOURCE_L1;
  record1.events_packet = AUX_EVENT_RETIRED;

  spe_record_raw_t record2 = {0};
  record2.total_lat = 120;
  record2.issue_lat = 60;
  record2.x_lat = 20;
  record2.data_source = DATA_SOURCE_L2;

  finode_t finode = {.ino = 300, .maj = 5, .min = 6, .ino_generation = 7};
  uint64_t offset = 3000;

  // Parse and insert first record
  lat_map_entry_t entry1 = {0};
  parse_lat_map_entry(&record1, &entry1, &finode, offset);
  insert_lat_map_entry(&entry1);

  // Parse and insert second record (same location)
  lat_map_entry_t entry2 = {0};
  parse_lat_map_entry(&record2, &entry2, &finode, offset);
  insert_lat_map_entry(&entry2);

  // Verify aggregated results
  lat_map_entry_t key = {.finode = finode, .offset = offset};
  const lat_map_entry_t *result = btree_get(lat_map, &key);
  assert(result != NULL);
  assert(result->total_latency == 200);       // 80 + 120
  assert(result->issue_latency == 100);       // 40 + 60
  assert(result->translation_latency == 30);  // 10 + 20
  assert(result->count == 2);                 // 1 + 1
  assert(result->l1.l2_bound_bin == 1);       // First record: execution_latency = 30
  assert(result->l2.l2_bound_bin == 1);       // Second record: execution_latency = 40
}

void test_lat_map() {
  test_init_lat_map();
  test_insert_lat_map_entry();
  test_parse_lat_map_entry();
  test_lat_integration();
}
