#include "sys.h"

#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/sysmacros.h>
#include <sys/types.h>
#include <unistd.h>

#include "log.h"

cpu_system_configuration_t cpu_system_config;

/// @brief Opens a file descriptor to cpuinfo
/// @return File Descriptor
FILE *open_cpu_info() {
  FILE *fp;
  fp = fopen("/proc/cpuinfo", "r");
  ASSERT(fp != NULL, "Error opening /proc/cpuinfo.");
  return fp;
}

/// @brief Uses the CPU file descriptor to read in the CPU part, so we can
/// assign latency bins
/// @return CPU part
uint64_t get_cpu_part() {
  FILE *fp = open_cpu_info();
  uint64_t part = CPU_PART_ID_GRV4;  // default to GRV4 if no part is found

  char line[256];
  while (fgets(line, sizeof(line), fp) != NULL) {
    if (sscanf(line, "CPU part\t: 0x%lx", &part) == 1) {
      break;
    }
  }

  fclose(fp);
  return part;
}

/// @brief Wrapper for page size to standardize everything
uint64_t get_page_size() {
  int page_sz = sysconf(_SC_PAGE_SIZE);
  ASSERT(page_sz != -1, "Failed to get page size.");
  return (uint64_t)page_sz;
}

/// @brief Gets the frequency of the machine in Hz using dmidecode
/// @return CPU Frequency in Hz
uint64_t get_frequency() {
  switch (get_cpu_part()) {
    case CPU_PART_ID_GRV2:
      return CPU_FREQ_GRV2;
    case CPU_PART_ID_GRV3:
      return CPU_FREQ_GRV3;
    case CPU_PART_ID_GRV4:
      return CPU_FREQ_GRV4;
    default:
      ASSERT(false, "Unkown part ID");
  }
}

/// @brief Wrapper to get num CPUs
uint64_t get_num_cpus() {
  int part = sysconf(_SC_NPROCESSORS_ONLN);
  ASSERT(part != -1, "Failed to get num CPUs.");
  return (uint64_t)part;
}

/// @brief Read the part number to bin latencies, which are later used for
///        histogramming data. Time granularity is picosecond so we preserve
///        the decimal in the ns, but don't cause unecessary floating point ops
///        or lose precision for larger amounts of samples. Picoseconds are also
///        well under the limit and won't saturate the counters for our use cases.
/// @param limits Bin to populate
void get_latency_bins(completion_latency_limits_t *limits) {
  uint64_t part = get_cpu_part();

  // These latencies were gathered from lat_mem_rd
  switch (part) {
    case CPU_PART_ID_GRV2:
      limits->l1_latency_cap_ps = 1800;   // 1.8 ns
      limits->l2_latency_cap_ps = 5700;   // 5.7 ns
      limits->l3_latency_cap_ps = 34000;  // 34 ns
      break;
    case CPU_PART_ID_GRV3:
      limits->l1_latency_cap_ps = 1800;   // 1.8 ns
      limits->l2_latency_cap_ps = 5700;   // 5.7 ns
      limits->l3_latency_cap_ps = 34000;  // 34 ns
      break;
    case CPU_PART_ID_GRV4:
      limits->l1_latency_cap_ps = 1500;   // 1.5 ns
      limits->l2_latency_cap_ps = 5000;   // 5.0 ns
      limits->l3_latency_cap_ps = 31000;  // 31 ns
      break;
    default:
      ASSERT(0, "Unknown CPU part.");
      break;
  }
}

/// @brief Configures the perf_event_open type, which is dependent on CPU generation
/// @return Perf type
uint64_t get_perf_event_type() {
  FILE *f = fopen("/sys/devices/arm_spe_0/type", "r");
  ASSERT(f != NULL, "Failed to open ARM SPE type file");

  char buffer[32];
  ASSERT(fgets(buffer, sizeof(buffer), f) != NULL, "Failed to read SPE type");

  fclose(f);

  errno = 0;
  uint64_t type = strtoull(buffer, NULL, 10);
  ASSERT(errno == 0, "Failed to convert SPE type to integer");

  return type;
}

/// @brief Initializes global CPU_SYSTEM_CONFIG struct
void init_sys_info() {
  cpu_system_config.cpu_part = get_cpu_part();
  cpu_system_config.frequency = get_frequency();
  cpu_system_config.page_size = get_page_size();
  cpu_system_config.num_cpus = get_num_cpus();

  get_latency_bins(&cpu_system_config.latency_limits);
  cpu_system_config.perf_event_type = get_perf_event_type();
  cpu_system_config.cyc_to_ps_conv_factor = SECOND_TO_PS / cpu_system_config.frequency;
}

/// @brief Returns the inode information associated with a file. Used for
/// initial mappings for /proc/maps
/// @param filename Filename to get information of
/// @param finode finode_t struct to populate
void get_file_info(const char *filename, finode_t *finode) {
  // Handle special cases
  if (strncmp(filename, "anon_inode:", 11) == 0 || filename[0] == '[') {
    finode->ino = 0;
    finode->maj = 0;
    finode->min = 0;
    finode->ino_generation = 0;
    return;
  }

  struct stat sb;
  if (lstat(filename, &sb) == -1) {
    finode->ino = 0;
    finode->maj = 0;
    finode->min = 0;
    finode->ino_generation = 0;
    return;
  }

  // General case
  finode->ino = sb.st_ino;
  dev_t dev = sb.st_dev;  // device containing the file
  finode->maj = major(dev);
  finode->min = minor(dev);
  finode->ino_generation = 0;
}
