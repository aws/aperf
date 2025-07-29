#pragma once

#include <stdint.h>

#define PROFILE_DEFAULT_WAKEUP_PERIOD 1       // 1s
#define PROFILE_DEFAULT_SPE_SAMPLE_FREQ 1000  // 1kHz
#define PROFILE_DEFAULT_TIMEOUT 10            // 10s
#define PROFILE_DEFAULT_NUM_REPORT 1000
#define MAX_SPE_SAMPLE_FREQ 4096  // cycles

/// @brief Global profile configuration. Will be accessed from online data
/// collection and offline report generation.
typedef struct profile_config {
  uint32_t wakeup_period;
  uint32_t hotline_frequency;
  uint32_t timeout;
  uint32_t num_to_report;
  const char *data_dir;
  char *bmiss_map_filename;
  char *lat_map_filename;
} profile_config_t;

/// @brief Utility struct to package all the buffer sizes together.
typedef struct perf_buffer_size_t {
  uint64_t perf_record_buf_sz;
  uint64_t perf_aux_buf_sz;
  uint64_t perf_aux_off;
} perf_buffer_size_t;

void parse_arguments(int argc, char *argv[]);

void get_perf_buffer_sizes(perf_buffer_size_t *buffer_sizes);

extern profile_config_t profile_configuration;