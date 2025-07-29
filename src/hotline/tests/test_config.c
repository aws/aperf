#include <assert.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "test.h"

void test_init() { cpu_system_config.page_size = 4096; }

void test_parse_arguments_defaults() {
  // Reset getopt's global state
  optind = 1;  // Reset to 1 before parsing arguments

  char *argv[] = {"test_program"};
  int argc = 1;

  parse_arguments(argc, argv);

  assert(profile_configuration.wakeup_period == PROFILE_DEFAULT_WAKEUP_PERIOD);
  assert(profile_configuration.hotline_frequency == PROFILE_DEFAULT_SPE_SAMPLE_FREQ);
  assert(profile_configuration.timeout == PROFILE_DEFAULT_TIMEOUT);
  assert(profile_configuration.num_to_report == PROFILE_DEFAULT_NUM_REPORT);
  assert(strcmp(profile_configuration.data_dir, "./data") == 0);
}

void test_parse_arguments_custom() {
  // Reset getopt's global state
  optind = 1;  // Reset to 1 before parsing arguments

  char *argv[] = {"test_program", "--wakeup_period", "5",  "--hotline_frequency",
                  "2000",         "--timeout",       "30", "--data_dir",
                  "/tmp/data"};
  int argc = 9;

  parse_arguments(argc, argv);

  assert(profile_configuration.wakeup_period == 5);
  assert(profile_configuration.hotline_frequency == 2000);
  assert(profile_configuration.timeout == 30);
  assert(strcmp(profile_configuration.data_dir, "/tmp/data") == 0);
}

void test_get_perf_buffer_sizes() {
  // Set up configuration
  profile_configuration.wakeup_period = 2;
  profile_configuration.hotline_frequency = 1000;

  perf_buffer_size_t buffer_sizes;
  get_perf_buffer_sizes(&buffer_sizes);

  // Verify calculations
  uint64_t raw_record_size = 16 * cpu_system_config.page_size * sizeof(switch_cpu_wide_record_t) *
                             profile_configuration.wakeup_period;
  uint64_t expected_record_buf =
      (uint64_t)pow(2, ceil(log2((double)raw_record_size))) + cpu_system_config.page_size;

  uint64_t expected_aux_buf_raw = profile_configuration.hotline_frequency *
                                  profile_configuration.wakeup_period * sizeof(spe_record_raw_t) *
                                  4;

  assert(buffer_sizes.perf_record_buf_sz == expected_record_buf);
  assert(buffer_sizes.perf_aux_off == expected_record_buf + cpu_system_config.page_size);
  assert(buffer_sizes.perf_aux_buf_sz >= expected_aux_buf_raw);
  assert((buffer_sizes.perf_aux_buf_sz & (buffer_sizes.perf_aux_buf_sz - 1)) == 0);
}

void test_get_perf_buffer_sizes_different_config() {
  profile_configuration.wakeup_period = 1;
  profile_configuration.hotline_frequency = 500;

  perf_buffer_size_t buffer_sizes;
  get_perf_buffer_sizes(&buffer_sizes);

  uint64_t raw_record_size = 16 * cpu_system_config.page_size * sizeof(switch_cpu_wide_record_t) *
                             profile_configuration.wakeup_period;
  uint64_t expected_record_buf =
      (uint64_t)pow(2, ceil(log2((double)raw_record_size))) + cpu_system_config.page_size;

  assert(buffer_sizes.perf_record_buf_sz == expected_record_buf);
  assert(buffer_sizes.perf_aux_off == expected_record_buf + cpu_system_config.page_size);
  assert((buffer_sizes.perf_aux_buf_sz & (buffer_sizes.perf_aux_buf_sz - 1)) == 0);
}

void test_config() {
  test_init();

  test_parse_arguments_defaults();
  test_parse_arguments_custom();
  test_get_perf_buffer_sizes();
  test_get_perf_buffer_sizes_different_config();
}