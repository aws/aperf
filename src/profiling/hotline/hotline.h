#pragma once

#include <stdint.h>
#include <sys/ioctl.h>
#include <sys/mman.h>
#include <sys/syscall.h>
#include <sys/types.h>

// Referenced from ARM Neoverse V2 Core TRM, Section 22
// and ARM SPE Performance Analysis Methodology White Paper, Section 2
#define PERF_ARM_SPE_RAW_CONFIG 0x10001  // enable load collection, branch collection
#define PERF_FORMAT_SPE 0x10

#define AUX_WATERMARK 64  // watermark notification for PERF_SAMPLE_AUX record generation

/// @brief populated from the first page of the buffers. Provides information on
/// how to convert between SPE time scale and perf time scale.
struct perf_tsc_conversion {
  uint16_t time_shift;
  uint32_t time_mult;
  uint64_t time_zero;
  uint64_t time_cycles;
  uint64_t time_mask;

  int cap_user_time_zero;
  int cap_user_time_short;
};

/// @brief CPU specific information
typedef struct cpu_session {
  uint32_t cpu;
  struct perf_tsc_conversion conv;  // ideally same across CPUs, but each core
                                    // emits it's own configuration
  uint64_t hardware_fd, software_fd;
  volatile struct perf_event_mmap_page *meta_page;
  void *perf_software_buffer;
  void *perf_aux_buffer;

  pid_t active_pid;
  uint64_t last_ctx_tail;

  // used for traversing buffers and updating B-Trees
  uint64_t last_aux_tail, last_record_tail;
  uint64_t last_aux_ts, last_record_ts;
} cpu_session_t;

void hotline(int argc, char *argv[]);