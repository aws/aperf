#include "config.h"

#include <getopt.h>
#include <inttypes.h>
#include <math.h>
#include <string.h>

#include "hotline.h"
#include "log.h"
#include "perf_interface.h"
#include "sys.h"

/// @brief Exposed profile configuration after argument parsing
profile_config_t profile_configuration;

/// @brief Argument parsing logic for CLI input
void parse_arguments(int argc, char *argv[]) {
  static struct option long_options[] = {{"wakeup_period", required_argument, 0, 'p'},
                                         {"hotline_frequency", required_argument, 0, 's'},
                                         {"timeout", required_argument, 0, 't'},
                                         {"data_dir", required_argument, 0, 'd'},
                                         {"num_to_report", required_argument, 0, 'n'},
                                         {0, 0, 0, 0}};

  int option_index = 0;
  int c;

  // Reset getopt
  optind = 1;

  profile_configuration.wakeup_period = PROFILE_DEFAULT_WAKEUP_PERIOD;
  profile_configuration.hotline_frequency = PROFILE_DEFAULT_SPE_SAMPLE_FREQ;
  profile_configuration.timeout = PROFILE_DEFAULT_TIMEOUT;
  profile_configuration.data_dir = "./data";
  profile_configuration.bmiss_map_filename = "hotline_bmiss_map.csv";
  profile_configuration.lat_map_filename = "hotline_lat_map.csv";
  profile_configuration.num_to_report = PROFILE_DEFAULT_NUM_REPORT;

  while ((c = getopt_long_only(argc, argv, "", long_options, &option_index)) != -1) {
    switch (c) {
      case 'p':
        ASSERT(sscanf(optarg, "%" SCNu32, &profile_configuration.wakeup_period) == 1,
               "Failed to read wakeup period");
        break;
      case 's':
        ASSERT(sscanf(optarg, "%" SCNu32, &profile_configuration.hotline_frequency) == 1,
               "Failed to read sample frequency");
        break;
      case 't':
        ASSERT(sscanf(optarg, "%" SCNu32, &profile_configuration.timeout) == 1,
               "Failed to read timeout");
        break;
      case 'd':
        profile_configuration.data_dir = optarg;
        break;
      case 'n':
        ASSERT(sscanf(optarg, "%" SCNu32, &profile_configuration.num_to_report) == 1,
               "Failed to read number to report");
        break;
      case '?':
        printf("Unknown option or missing argument\n");
        printf(
            "Usage: ./<BINARY> --wakeup_period X --hotline_frequency X "
            "--timeout X "
            "--data_dir path\n");
        exit(EXIT_FAILURE);
        break;
      default:
        printf("Invalid command provided.\n");
        printf(
            "Usage: ./<BINARY> --wakeup_period X --hotline_frequency X "
            "--timeout X "
            "--data_dir path\n");
        exit(EXIT_FAILURE);
    }
  }

  ASSERT(profile_configuration.wakeup_period > 0, "Wakeup period must be greater than 0.");
  ASSERT(profile_configuration.hotline_frequency > 0 &&
             profile_configuration.hotline_frequency <= MAX_SPE_SAMPLE_FREQ,
         "SPE sample frequency provided is out of range.");
  ASSERT(profile_configuration.timeout > 0, "Timeout must be greater than 0.");
}

/// @brief computes the buffer sizes for the record and aux buffers
/// and returns it in a packaged struct.
/// @returns perf record buffer size, aux buffer size, and aux offset
void get_perf_buffer_sizes(perf_buffer_size_t *buffer_sizes) {
  uint64_t page_sz = cpu_system_config.page_size;

  // independent of sampling period, and hard to
  // predict due to context switches, so we statically
  // make it a large amount profiling shows that this
  // causes an increase in CPU util at the begining of
  // the tool, during setup, but does not have much of
  // an impact later onwards. We instead make it only
  // proportional to the wakeup period
  uint64_t perf_record_buf_sz = 16 * cpu_system_config.page_size *
                                sizeof(switch_cpu_wide_record_t) *
                                profile_configuration.wakeup_period;
  uint64_t perf_aux_buf_sz = profile_configuration.hotline_frequency *
                             profile_configuration.wakeup_period * sizeof(spe_record_raw_t) *
                             4;  // overestimate factor of 4x

  // round it to a power of 2, as
  // required by perf_event_open docs
  perf_aux_buf_sz = (uint64_t)pow(2, ceil(log2((double)perf_aux_buf_sz)));

  // round up to it is of the form 1 + 2^n pages
  perf_record_buf_sz =
      (uint64_t)pow(2, ceil(log2((double)perf_record_buf_sz))) + cpu_system_config.page_size;

  uint64_t perf_aux_off = perf_record_buf_sz + page_sz;

  buffer_sizes->perf_record_buf_sz = perf_record_buf_sz;
  buffer_sizes->perf_aux_buf_sz = perf_aux_buf_sz;
  buffer_sizes->perf_aux_off = perf_aux_off;
}