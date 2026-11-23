#include "hotline.h"

#include <fcntl.h>
#include <signal.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#include "bmiss_map.h"
#include "config.h"
#include "finode_map.h"
#include "fname_map.h"
#include "lat_map.h"
#include "log.h"
#include "perf_interface.h"
#include "sys.h"

cpu_session_t *sessions = NULL;
bool terminate_flag = 0;

/// @brief syscall wrapper for invoking the perf event open syscall
/// @param hw_event event attribute struct
/// @param pid pid to profile, -1 if pid independent
/// @param cpu cpu to profile, -1 if cpu independent
/// @param group_fd fd to forward data to
/// @param flags additional configuration flags
/// @return result of the syscall
int perf_event_open(struct perf_event_attr *hw_event, pid_t pid, int cpu, int group_fd,
                    unsigned long flags) {
  int ret;
  ret = syscall(SYS_perf_event_open, hw_event, pid, cpu, group_fd, flags);
  return ret;
}

/// @brief Initializes the hardware perf event for the PMU
/// @param session Active CPU session
void init_perf_hardware_event(cpu_session_t *session) {
  int fd;
  struct perf_event_attr attr;
  memset(&attr, 0, sizeof(attr));
  attr.type = cpu_system_config.perf_event_type;
  attr.config = PERF_ARM_SPE_RAW_CONFIG;
  attr.size = sizeof(attr);
  attr.disabled = 1;
  attr.inherit = 1;
  attr.read_format = PERF_FORMAT_ID | PERF_FORMAT_SPE;
  attr.sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_CPU |
                     PERF_SAMPLE_DATA_SRC | PERF_SAMPLE_IDENTIFIER | PERF_SAMPLE_BRANCH_STACK;
  attr.sample_period = cpu_system_config.frequency / profile_configuration.hotline_frequency;
  attr.sample_id_all = 1;
  attr.context_switch = 1;
  attr.aux_watermark = AUX_WATERMARK;
  attr.exclude_guest = 1;
  attr.branch_sample_type = PERF_SAMPLE_BRANCH_ANY;

  // assign pid=-1 to profile for all processes, on this particular CPU
  fd = perf_event_open(&attr, -1, session->cpu, -1, PERF_FLAG_FD_CLOEXEC);
  ASSERT(fd != -1, "Failed to open perf hardware event.");

  session->hardware_fd = fd;
}

/// @brief Initializes the software perf event for the PMU, used for emitting
/// context switches/MMAP2/exits
/// @param session Active CPU session
void init_perf_software_event(cpu_session_t *session) {
  struct perf_event_attr attr;
  int fd;
  memset(&attr, 0, sizeof(attr));
  attr.type = PERF_TYPE_SOFTWARE;
  attr.size = sizeof(attr);
  attr.config = PERF_COUNT_SW_DUMMY;
  attr.sample_period = cpu_system_config.frequency / profile_configuration.hotline_frequency;
  attr.sample_type = PERF_SAMPLE_IP | PERF_SAMPLE_TID | PERF_SAMPLE_TIME | PERF_SAMPLE_CPU |
                     PERF_SAMPLE_IDENTIFIER;
  attr.read_format = PERF_FORMAT_ID | PERF_FORMAT_SPE;
  attr.disabled = 1;
  // attr.inherit = 1;
  attr.exclude_kernel = 1;
  attr.exclude_hv = 1;
  attr.mmap = 1;
  // attr.comm = 1;
  // attr.task = 1;
  attr.sample_id_all = 1;
  attr.exclude_guest = 1;
  attr.mmap2 = 1;
  // attr.comm_exec = 1;
  attr.context_switch = 1;
  // attr.ksymbol = 1;
  // attr.bpf_event = 1;
  attr.watermark = 1;

  fd = perf_event_open(&attr, -1, session->cpu, -1, PERF_FLAG_FD_CLOEXEC);
  ASSERT(fd != -1, "Failed to open perf software event.");

  session->software_fd = fd;
}

/// @brief MMAPs record and aux buffers for the perf events
/// @param session Active CPU session
void mmap_perf_buffers(cpu_session_t *session) {
  perf_buffer_size_t buffer_sizes;
  get_perf_buffer_sizes(&buffer_sizes);

  struct perf_event_mmap_page *meta_page = (struct perf_event_mmap_page *)mmap(
      NULL, buffer_sizes.perf_record_buf_sz, PROT_READ | PROT_WRITE, MAP_SHARED,
      session->hardware_fd, 0);

  ASSERT(meta_page != MAP_FAILED, "Failed to mmap perf buffer.");

  meta_page->aux_offset = buffer_sizes.perf_aux_off;
  meta_page->aux_size = buffer_sizes.perf_aux_buf_sz;

  void *aux_buffer = mmap(NULL, buffer_sizes.perf_aux_buf_sz, PROT_READ | PROT_WRITE, MAP_SHARED,
                          session->hardware_fd, buffer_sizes.perf_aux_off);

  ASSERT(aux_buffer != MAP_FAILED, "Failed to mmap aux buffer.");

  session->meta_page = meta_page;
  session->perf_software_buffer = (char *)meta_page + cpu_system_config.page_size;
  session->perf_aux_buffer = aux_buffer;
}

/// @brief initializes the hardware and software perf events for a CPU session
void init_perf_events(cpu_session_t *session) {
  init_perf_hardware_event(session);
  init_perf_software_event(session);
  mmap_perf_buffers(session);

  int ret = fcntl(session->hardware_fd, F_SETFL, O_RDONLY | O_NONBLOCK);
  ASSERT(ret != -1, "Failed to set hardware event to non-blocking.");
  ret = ioctl(session->software_fd, PERF_EVENT_IOC_SET_OUTPUT, session->hardware_fd);
  ASSERT(ret != -1, "Failed to set software event output to hardware event.");
  ret = fcntl(session->software_fd, F_SETFL, O_RDONLY | O_NONBLOCK);
  ASSERT(ret != -1, "Failed to set software event to non-blocking.");
}

/// @brief Toggles the PMU to either enable, disable, or reset
/// @param session Active CPU session
/// @param toggle Flag to toggle to
void toggle_pmu(cpu_session_t *session, uint64_t toggle) {
  int ret;
  ret = ioctl(session->hardware_fd, toggle, 0);
  ret = ioctl(session->software_fd, toggle, 0);
  // we don't need to toggle the software_fd after this. This is because
  // hardware_fd is the leader of the group, and both are going to be scheduled
  // together, so if hardware_fd is disabled, software_fd is essentially
  // disabled (cannot be scheduled).
  ASSERT(ret != -1, "Failed to toggle hardware PMU");
}

/// @brief Configures the time conversions for the perf event so we can convert
/// from SPE to perf
/// @param session Active CPU session
void configure_session_conv(cpu_session_t *session) {
  session->conv.cap_user_time_short = 1;
  session->conv.cap_user_time_zero = 1;
  session->conv.time_cycles = session->meta_page->time_cycles;
  session->conv.time_mask = session->meta_page->time_mask;
  session->conv.time_mult = session->meta_page->time_mult;
  session->conv.time_shift = session->meta_page->time_shift;
  session->conv.time_zero = session->meta_page->time_zero;
}

/// @brief Initializes all the perf events for each CPU
void init_sessions() {
  sessions = malloc(sizeof(cpu_session_t) * cpu_system_config.num_cpus);
  memset(sessions, 0, sizeof(cpu_session_t) * cpu_system_config.num_cpus);
  ASSERT(sessions != NULL, "Failed to malloc sessions.");
  for (uint64_t i = 0; i < cpu_system_config.num_cpus; i++) {
    sessions[i].cpu = i;
    init_perf_events(&sessions[i]);
    configure_session_conv(&sessions[i]);
  }
}

/// @brief Enables perf profiling across all CPUs
void enable_perf_profiling() {
  for (uint64_t i = 0; i < cpu_system_config.num_cpus; i++) {
    toggle_pmu(&sessions[i], PERF_EVENT_IOC_ENABLE);
  }
}

/// @brief Conversion function from SPE time scale to Perf time scale, so we can
/// directly compare the two records.
/// referenced from linux perf: linux/tools/perf/util/tsc.c and
/// https://man7.org/linux/man-pages/man2/perf_event_open.2.html
/// @param cyc cycles in SPE time scale
/// @param session CPU session, which has meta_page and conversion info
/// @return perf time
// uint64_t tsc_to_perf_time(uint64_t cyc, struct perf_tsc_conversion *tc) {
uint64_t tsc_to_perf_time(uint64_t cyc, cpu_session_t *session) {
  uint64_t quot, rem;

  volatile struct perf_event_mmap_page *meta = session->meta_page;

  if (meta->cap_user_time_short)
    cyc = meta->time_cycles + ((cyc - meta->time_cycles) & meta->time_mask);

  quot = cyc >> meta->time_shift;
  rem = cyc & (((uint64_t)1 << meta->time_shift) - 1);
  return meta->time_zero + quot * meta->time_mult + ((rem * meta->time_mult) >> meta->time_shift);
}

/// @brief Given a perf record, gets the timestamp from it. Special care is
///       required for MMAP2 records. Returns `0` on no event found, and the
///       timestamp of the record.
uint64_t get_perf_event_timestamp(struct perf_event_header *header) {
  switch (header->type) {
    case PERF_RECORD_AUX: {
      return ((aux_record_t *)header)->sid.time;
    }

    // We need to handle the PERF_RECORD_MMAP2 separately because of the `char
    // filename[];`. The sample sid struct is *after* the filename, so we use
    // full size of the record, offset it by the sample id, and extract the
    // timestamp.
    case PERF_RECORD_MMAP2: {
      mmap2_record_t *mmap2_rec = (mmap2_record_t *)header;
      size_t fixed_size = offsetof(mmap2_record_t, filename);
      size_t filename_len = header->size - fixed_size - sizeof(struct sample_id);
      sample_id_t *sid = (sample_id_t *)((char *)mmap2_rec + fixed_size + filename_len);
      return sid->time;
    }

    case PERF_RECORD_SWITCH_CPU_WIDE: {
      return ((switch_cpu_wide_record_t *)header)->sid.time;
    }

    case PERF_RECORD_EXIT: {
      return ((process_exit_record_t *)header)->sid.time;
    }

    default: {
      return 0;
    }
  }
}

/// @brief Processes a record for the perf record buffer
/// @param session Active CPU session
/// @param header Perf header for the record to process
static inline void process_software_buffer_record(struct perf_event_header *header,
                                                  cpu_session_t *session) {
  switch (header->type) {
    case PERF_RECORD_MMAP2: {
      struct mmap2_record *mmap2_rec = (struct mmap2_record *)header;

      // logic to update fname_map
      insert_finode_entry(mmap2_rec);  // add mapping to inodes for decoding later
      insert_fname_entry(mmap2_rec);
      break;
    }

    case PERF_RECORD_EXIT: {
      struct process_exit_record *exit = (struct process_exit_record *)header;

      // logic to remove pid from fname_map
      remove_fname_entry(exit->pid);
      break;
    }

    case PERF_RECORD_SWITCH_CPU_WIDE: {
      struct switch_cpu_wide_record *switch_rec = (struct switch_cpu_wide_record *)header;
      session->active_pid = switch_rec->header.misc & PERF_RECORD_MISC_SWITCH_OUT
                                ? (pid_t)switch_rec->next_prev_pid
                                : session->active_pid;
      break;
    }

    case PERF_RECORD_MMAP:
    case PERF_RECORD_SAMPLE:
    case PERF_RECORD_AUX:
    case PERF_RECORD_ITRACE_START:
    case PERF_RECORD_LOST_SAMPLES:
    case PERF_RECORD_LOST:
    case PERF_RECORD_THROTTLE:
    case PERF_RECORD_UNTHROTTLE:
    case PERF_RECORD_READ:
    case PERF_RECORD_COMM:
    case PERF_RECORD_FORK:
    case PERF_RECORD_SWITCH: {
      break;
    }

    case PERF_RECORD_NAMESPACES:
    case PERF_RECORD_KSYMBOL:
    case PERF_RECORD_BPF_EVENT:
    case PERF_RECORD_CGROUP:
    case PERF_RECORD_TEXT_POKE: {
      ASSERT(false, "Unexpected buffer entry.");
    }
  }
}

/// @brief Process a record for the AUX buffer
/// @param session Active CPU session
/// @param record Record to process
void process_aux_buffer_record(cpu_session_t *session, spe_record_raw_t *record) {
  uint64_t pc;
  memcpy(&pc, &record->pc, 7);
  pc = pc & 0x00FFFFFFFFFFFFFF;  // zero out the top byte because
                                 // SPE PC is 7 bytes

  uint64_t offset;
  finode_t finode;
  bool res = va_to_file_offset(pc, session->active_pid, &finode, &offset);

  if (res == false) {
    return;  // unable to map pc back to file/file offset
  }

  switch (record->type) {
    case AUX_PACKET_TYPE_LAT:
      parse_and_insert_lat_entry(record, &finode, offset);
      break;

    case AUX_PACKET_TYPE_BRANCH:
      parse_and_insert_bmiss_entry(record, &finode, offset);
      break;
  }
}

/// @brief Process all the entries in the record buffer up to the `target_ts`
/// @param session Active CPU session
/// @param target_ts Timestamp to go up until
void process_software_buffer_up_to_ts(cpu_session_t *session, uint64_t target_ts) {
  char *data_page = session->perf_software_buffer;
  uint64_t data_head = session->meta_page->data_head;
  uint64_t data_tail = session->last_record_tail;  // use session's last position
  // "On SMP-capable platforms, after reading the data_head value, user space
  // should issue an rmb()."
  // https://man7.org/linux/man-pages/man2/perf_event_open.2.html
#ifdef __aarch64__
  asm volatile("dmb ishld" ::: "memory");  // memory barrier for reading
#endif
  uint64_t data_size = session->meta_page->data_size;

  // use the last recorded timestamp to continue from
  uint64_t last_ts = session->last_record_ts;

  while (data_tail + sizeof(struct perf_event_header) < data_head) {
    struct perf_event_header *header =
        (struct perf_event_header *)(data_page + (data_tail % data_size));

    if (data_tail + header->size > data_head) {
      break;
    }

    uint64_t record_ts = get_perf_event_timestamp(header);

    if (record_ts > target_ts) {
      break;  // don't process this record. Note, `record_ts = 0` records (i.e.
              // those that don't have a timestamp), are skippped.
    }

    last_ts = record_ts;  // update the last processed timestamp

    process_software_buffer_record(header, session);

    data_tail += header->size;
  }

  session->last_record_ts = last_ts;
  session->last_record_tail = data_tail;

  session->meta_page->data_tail = data_tail;
}

/// @brief Processes all the aux buffer entries for a CPU session
/// @param session Active CPU session
void process_aux_buffer(cpu_session_t *session) {
  void *aux = session->perf_aux_buffer;
  uint64_t aux_size = session->meta_page->aux_size;
  uint64_t aux_head = session->meta_page->aux_head;

  // "On SMP-capable platforms, after reading the data_head value, user space
  // should issue an rmb()." The same must be done for the `aux_head`, according
  // to the docs. https://man7.org/linux/man-pages/man2/perf_event_open.2.html
#ifdef __aarch64__
  asm volatile("dmb ishld" ::: "memory");  // memory barrier for reading
#endif
  uint64_t aux_tail = session->last_aux_tail;

  uint64_t last_processed_ts = 0;

  // we use 2 * sizeof(spe_record_raw_t) to avoid a possible edge case where SPE
  // writes a sample before a SWITCH record is emitted
  while (aux_tail + 2 * sizeof(spe_record_raw_t) <= aux_head) {
    spe_record_raw_t *record = (spe_record_raw_t *)(aux + (aux_tail % aux_size));

    uint64_t timestamp = record->timestamp;

    uint64_t perf_ts = tsc_to_perf_time(timestamp, session);

    if (perf_ts >= last_processed_ts) {
      process_software_buffer_up_to_ts(session, perf_ts);
      // at this point, we should have the current active PID, and all the
      // mappings should be updated

      process_aux_buffer_record(session, record);
      last_processed_ts = perf_ts;
    }

    aux_tail += sizeof(spe_record_raw_t);
    session->last_aux_tail = aux_tail;
    session->meta_page->aux_tail = aux_tail;
  }
}

void serialize_bmiss_map() {
  char path[4096];
  FILE *branch_fp = NULL;
  struct btree_iter *iter = NULL;
  bool ok;

  // open branch file
  int res = snprintf(path, sizeof(path), "%s/%s", profile_configuration.data_dir,
                     profile_configuration.bmiss_map_filename);
  ASSERT(res >= 0, "Failed to create branch path.");
  branch_fp = fopen(path, "w");

  // write header
  res = fprintf(branch_fp,
                "filename,offset,count,not_taken_branches,"
                "mispredicted,total_latency,issue_latency,branch_type\n");
  ASSERT(res >= 0, "Failed to write branch header.");

  // write branch map entries
  iter = btree_iter_new(bmiss_map);
  ok = btree_iter_seek(iter, &(bmiss_map_entry_t){});

  while (ok) {
    const bmiss_map_entry_t *entry = btree_iter_item(iter);
    finode_map_entry_t key = {0};
    key.finode = entry->finode;
    const finode_map_entry_t *finode_entry = btree_get(finode_map, &key);

    ASSERT(finode_entry != NULL, "Failed to recover filename.");

    int write_result =
        fprintf(branch_fp, "%s,0x%lx,%lu,%lu,%x\n", finode_entry->filename, entry->offset,
                entry->count, entry->mispredicted, entry->branch_type);
    ASSERT(write_result >= 0, "Failed to write branch entry");
    ok = btree_iter_next(iter);
  }

  fclose(branch_fp);
}

void serialize_lat_map() {
  char path[4096];
  FILE *lat_fp = NULL;
  struct btree_iter *iter = NULL;
  bool ok;

  // open lat file
  int res = snprintf(path, sizeof(path), "%s/%s", profile_configuration.data_dir,
                     profile_configuration.lat_map_filename);
  ASSERT(res >= 0, "Failed to create load path.");
  lat_fp = fopen(path, "w");

  ASSERT(lat_fp != NULL, "Failed to open output files");

  // write header
  res = fprintf(lat_fp,
                "filename,offset,count,total_latency,issue_latency,"
                "translation_latency,"
                "l1_bin1,l1_bin2,l1_bin3,l1_bin4,"
                "l2_bin1,l2_bin2,l2_bin3,l2_bin4,"
                "l3_bin1,l3_bin2,l3_bin3,l3_bin4,"
                "dram_bin1,dram_bin2,dram_bin3,dram_bin4,saturated\n");
  ASSERT(res >= 0, "Failed to write lat map header.");

  // write lat map entries
  iter = btree_iter_new(lat_map);
  ok = btree_iter_seek(iter, &(lat_map_entry_t){});

  while (ok) {
    const lat_map_entry_t *entry = btree_iter_item(iter);
    finode_map_entry_t key = {0};
    key.finode = entry->finode;
    const finode_map_entry_t *finode_entry = btree_get(finode_map, &key);

    ASSERT(finode_entry != NULL, "Failed to recover filename.");

    int write_result =
        fprintf(lat_fp,
                "%s,0x%lx,%lu,%lu,%lu,%lu,"
                "%lu,%lu,%lu,%lu,"        // l1 bins
                "%lu,%lu,%lu,%lu,"        // l2 bins
                "%lu,%lu,%lu,%lu,"        // l3 bins
                "%lu,%lu,%lu,%lu,%lu\n",  // dram bins
                finode_entry->filename, entry->offset, entry->count, entry->total_latency,
                entry->issue_latency, entry->translation_latency, entry->l1.l1_bound_bin,
                entry->l1.l2_bound_bin, entry->l1.l3_bound_bin, entry->l1.dram_bound_bin,
                entry->l2.l1_bound_bin, entry->l2.l2_bound_bin, entry->l2.l3_bound_bin,
                entry->l2.dram_bound_bin, entry->l3.l1_bound_bin, entry->l3.l2_bound_bin,
                entry->l3.l3_bound_bin, entry->l3.dram_bound_bin, entry->dram.l1_bound_bin,
                entry->dram.l2_bound_bin, entry->dram.l3_bound_bin, entry->dram.dram_bound_bin,
                entry->saturated);
    ok = btree_iter_next(iter);
    ASSERT(write_result >= 0, "Failed to write lat entry");
  }

  fclose(lat_fp);
}

/// @brief Serializes the lat_map and bmiss_map into files
void serialize_maps() {
  serialize_bmiss_map();
  serialize_lat_map();
}

void handle_signal(int signum __attribute__((unused))) { terminate_flag = true; }

uint64_t clock_gettime_monotonic() {
  struct timespec ts;
  clock_gettime(CLOCK_MONOTONIC, &ts);
  return (uint64_t)ts.tv_sec * 1000000000ULL + ts.tv_nsec;
}

/// @brief Exposed wrapper function that APerf will call
/// @param argc Standard C like argc
/// @param argv Standard C like argv
void hotline(int argc, char *argv[]) {
  init_sys_info();
  parse_arguments(argc, argv);

  init_sessions();
  init_finode_map();
  init_fname_map();
  init_lat_map();
  init_bmiss_map();

  // configure signal handling
  struct sigaction sa = {.sa_handler = handle_signal, .sa_flags = 0};
  sigemptyset(&sa.sa_mask);

  ASSERT(sigaction(SIGTERM, &sa, NULL) != -1, "Sigaction failed.");

  uint64_t start_time = clock_gettime_monotonic();
  uint64_t timeout_ns = (uint64_t)profile_configuration.timeout * 1000000000ULL;

  uint64_t end_time = start_time + timeout_ns;

  enable_perf_profiling();

  while (clock_gettime_monotonic() < end_time && !terminate_flag) {
    sleep(profile_configuration.wakeup_period);
    for (uint64_t c = 0; c < cpu_system_config.num_cpus; c++) {
      process_aux_buffer(&sessions[c]);
    }
  }

  serialize_maps();
}