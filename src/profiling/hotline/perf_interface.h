#pragma once

#include <linux/perf_event.h>
#include <stdint.h>

#define AUX_PACKET_TYPE_LAT 0x49
#define AUX_PACKET_TYPE_BRANCH 0x4a
#define AUX_RECORD_TYPE_BCOND 0x01
#define AUX_PACKET_SATURATED 4095

#define AUX_EVENT_RETIRED 1 << 1
#define AUX_EVENT_BRANCH_NOT_TAKEN 1 << 6
#define AUX_EVENT_BRANCH_MISS 1 << 7

#define DATA_SOURCE_L1 0b0000
#define DATA_SOURCE_L2 0b1000
#define DATA_SOURCE_PEER_CORE 0b1001
#define DATA_SOURCE_LOCAL_CLUSTER 0b1010
#define DATA_SOURCE_SYSTEM_CACHE 0b1011
#define DATA_SOURCE_PEER_CLUSTER 0b1100
#define DATA_SOURCE_REMOTE 0b1101
#define DATA_SOURCE_DRAM 0b1110

/// @brief sample_id struct spec from
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
typedef struct sample_id {
  uint32_t pid, tid;
  uint64_t time;
  uint32_t cpu, res;
  uint64_t id;
} sample_id_t;

/// @brief aux record spec from
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
typedef struct __attribute__((packed)) aux_record {
  struct perf_event_header header;
  uint64_t aux_offset;
  uint64_t aux_size;
  uint64_t flags;
  struct sample_id sid;
} aux_record_t;

/// @brief mmap2 record spec from
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
typedef struct __attribute__((packed)) mmap2_record {
  struct perf_event_header header;
  uint32_t pid;
  uint32_t tid;
  uint64_t addr;
  uint64_t len;
  uint64_t pgoff;
  union {
    struct {
      uint32_t maj;
      uint32_t min;
      uint64_t ino;
      uint64_t ino_generation;
    };

    struct {
      uint8_t bbuild_id_size;
      uint8_t __reserved_1;
      uint16_t __reserved_2;
      uint8_t build_id[20];
    };
  };

  uint32_t prot;
  uint32_t flags;
  char filename[];
} mmap2_record_t;

/// @brief switch_cpu_wide spec from
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
typedef struct __attribute__((packed)) switch_cpu_wide_record {
  struct perf_event_header header;
  uint32_t next_prev_pid;
  uint32_t next_prev_tid;
  struct sample_id sid;
} switch_cpu_wide_record_t;

/// @brief process_exit spec from
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
typedef struct __attribute__((packed)) process_exit_record {
  struct perf_event_header header;
  uint32_t pid, ppid;
  uint32_t tid, ptid;
  uint64_t time;
  struct sample_id sid;
} process_exit_record_t;

/// @brief raw ARM SPE packet that the PMU emmits
/// The contents of this struct are config dependent, which
/// can be updated under hotline.c:init_perf_hardware_event
typedef struct __attribute__((packed)) spe_record_raw {
  uint8_t __reserved1;
  uint8_t pc[7];
  uint8_t __reserved2;
  uint8_t __reserved3[10];
  uint8_t type;
  uint8_t reg;
  uint8_t identifier;
  uint32_t events_packet;
  uint8_t __reserved4;
  uint16_t issue_lat;
  uint8_t __reserved5;
  uint16_t total_lat;
  uint64_t vaddr;
  uint8_t __reserved6;
  uint8_t __reserved7;
  uint16_t x_lat;
  uint8_t __reserved8[9];
  uint8_t __reserved9;
  uint8_t data_source;
  uint8_t __reserved10;
  uint64_t timestamp;
} spe_record_raw_t;