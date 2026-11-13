/* SPDX-License-Identifier: GPL-2.0-only OR MIT */
#ifndef APERF_OFF_CPU_H
#define APERF_OFF_CPU_H

/* Why a stack id was not captured, carried from switch-out to switch-in to attribute its time. */
enum {
	OFF_CPU_STACK_OK = 0,
	OFF_CPU_STACK_COLLISION = 1,
	OFF_CPU_STACK_FAILED = 2,
	OFF_CPU_STACK_NONE = 3,
};

/* Per-thread record stashed when a task blocks, keyed by its tid in START. */
struct off_cpu_start {
	__u64 ts_ns;
	__u32 tgid;
	__u8 kstack_miss;
	__u8 ustack_miss;
	__u16 _pad;
	__s64 kstack_id;
	__s64 ustack_id;
	__u8 comm[16];
};

/* COUNTS aggregation key: process, comm, middle buckets, both stack ids, and wake-up bucket.
 * Mirrored by the userspace Rust OffCpuKey. */
struct off_cpu_key {
	__u32 pid;
	__u8 comm[16];
	__u32 mid_buckets;
	__s64 kstack_id;
	__s64 ustack_id;
	__u64 bucket;
};

struct off_cpu_value {
	__u64 switch_in_ns;
	__u64 head_ns;
	__u64 mid_ns;
	__u64 tail_ns;
};

enum {
	OFF_CPU_STAT_START_UPDATE_FAILED = 0,
	OFF_CPU_STAT_COUNTS_UPDATE_FAILED = 1,
	OFF_CPU_STAT_KSTACK_FAILED = 2,
	OFF_CPU_STAT_KSTACK_COLLISION = 3,
	OFF_CPU_STAT_USTACK_FAILED = 4,
	OFF_CPU_STAT_USTACK_COLLISION = 5,
	OFF_CPU_STAT_SWITCHES_TOTAL = 6,
	OFF_CPU_STAT_EVENTS_TOTAL = 7,
	OFF_CPU_STAT_TOTAL_NS = 8,
	OFF_CPU_STAT_KSTACK_COLLISION_NS = 9,
	OFF_CPU_STAT_KSTACK_FAILED_NS = 10,
	OFF_CPU_STAT_USTACK_COLLISION_NS = 11,
	OFF_CPU_STAT_USTACK_FAILED_NS = 12,
	OFF_CPU_STAT_BLOCK_LT_100US = 13,
	OFF_CPU_STAT_BLOCK_LT_1MS = 14,
	OFF_CPU_STAT_BLOCK_LT_10MS = 15,
	OFF_CPU_STAT_BLOCK_LT_100MS = 16,
	OFF_CPU_STAT_BLOCK_LT_1S = 17,
	OFF_CPU_STAT_BLOCK_LT_10S = 18,
	OFF_CPU_STAT_BLOCK_GE_10S = 19,
	OFF_CPU_STAT_BLOCK_LT_10US = 20,
	OFF_CPU_STAT_USTACK_KTHREAD = 21,
	OFF_CPU_STAT_USTACK_KTHREAD_NS = 22,
	OFF_CPU_STAT_RUNNABLE = 23,
	OFF_CPU_STAT_NR = 24,
};

_Static_assert(sizeof(struct off_cpu_start) == 48, "off_cpu_start layout");
_Static_assert(sizeof(struct off_cpu_key) == 48, "off_cpu_key layout");
_Static_assert(__builtin_offsetof(struct off_cpu_key, kstack_id) == 24, "off_cpu_key layout");
_Static_assert(sizeof(struct off_cpu_value) == 32, "off_cpu_value layout");

#endif
