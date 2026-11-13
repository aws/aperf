/* SPDX-License-Identifier: GPL-2.0-only OR MIT */
#include "kernel_types.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_core_read.h>
#include <bpf/bpf_tracing.h>
#include "off_cpu.h"

char LICENSE[] SEC("license") = "Dual MIT/GPL";

/* Load-time tunables overridden from userspace: optional tgid filter, minimum
 * blocked time to record, capture deadline, and the aggregation bucket width and
 * epoch offset in ns. */
const volatile __u32 target_tgid = 0;
const volatile __u64 min_block_ns = 0, stop_at_ns = 0;
const volatile __u64 bucket_ns = 0, epoch_offset_ns = 0;

// KSTACK_TRACES / USTACK_TRACES: stack id -> instruction pointers. Split so user FP-less
// stacks can't occupy the slots kernel stacks hash to, and vice versa.
struct {
	__uint(type, BPF_MAP_TYPE_STACK_TRACE);
	__uint(max_entries, 16384);
	__uint(key_size, sizeof(__u32));
	__uint(value_size, PERF_MAX_STACK_DEPTH * sizeof(__u64));
} KSTACK_TRACES SEC(".maps");

struct {
	__uint(type, BPF_MAP_TYPE_STACK_TRACE);
	__uint(max_entries, 262144);
	__uint(key_size, sizeof(__u32));
	__uint(value_size, PERF_MAX_STACK_DEPTH * sizeof(__u64));
} USTACK_TRACES SEC(".maps");

// START: in-flight off-CPU starts keyed by tid
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 65536);
	__type(key, __u32);
	__type(value, struct off_cpu_start);
} START SEC(".maps");

// COUNTS: aggregated blocked ns (head/mid/tail) and first switch-in time, keyed by off_cpu_key
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__uint(max_entries, 524288);
	__type(key, struct off_cpu_key);
	__type(value, struct off_cpu_value);
} COUNTS SEC(".maps");

// OFF_CPU_STATS: totals, duration bands, and data-loss counters
struct {
	__uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
	__uint(max_entries, OFF_CPU_STAT_NR);
	__type(key, __u32);
	__type(value, __u64);
} OFF_CPU_STATS SEC(".maps");

static __always_inline void stat_inc(__u32 idx)
{
	__u64 *v = bpf_map_lookup_elem(&OFF_CPU_STATS, &idx);

	if (v)
		(*v)++;
}

static __always_inline void stat_add(__u32 idx, __u64 delta)
{
	__u64 *v = bpf_map_lookup_elem(&OFF_CPU_STATS, &idx);

	if (v)
		(*v) += delta;
}

static __always_inline __u32 duration_band(__u64 delta)
{
	if (delta < 10000)
		return OFF_CPU_STAT_BLOCK_LT_10US;
	if (delta < 100000)
		return OFF_CPU_STAT_BLOCK_LT_100US;
	if (delta < 1000000)
		return OFF_CPU_STAT_BLOCK_LT_1MS;
	if (delta < 10000000)
		return OFF_CPU_STAT_BLOCK_LT_10MS;
	if (delta < 100000000)
		return OFF_CPU_STAT_BLOCK_LT_100MS;
	if (delta < 1000000000)
		return OFF_CPU_STAT_BLOCK_LT_1S;
	if (delta < 10000000000)
		return OFF_CPU_STAT_BLOCK_LT_10S;
	return OFF_CPU_STAT_BLOCK_GE_10S;
}

static __always_inline long task_state(struct task_struct *t)
{
	if (bpf_core_field_exists(t->__state))
		return BPF_CORE_READ(t, __state);
	return BPF_CORE_READ((struct task_struct___pre514 *)t, state);
}

/* On each context switch, handle both tasks: the one leaving the CPU (prev)
 * and the one returning (next). Dying and runnable tasks, and blocks shorter than
 * min_block_ns, are skipped, as is every switch past stop_at_ns. */
SEC("tp_btf/sched_switch")
int BPF_PROG(off_cpu, bool preempt, struct task_struct *prev, struct task_struct *next)
{
	__u64 now = bpf_ktime_get_ns();

	if (now > stop_at_ns)
		return 0;
	__u32 next_pid = BPF_CORE_READ(next, pid);
	__u32 prev_pid = BPF_CORE_READ(prev, pid);
	__u32 prev_tgid = BPF_CORE_READ(prev, tgid);
	bool tracked = prev_pid != 0 && (target_tgid == 0 || prev_tgid == target_tgid);
	// Return-to-user preemption has preempt=false but leaves prev TASK_RUNNING.
	bool runnable = tracked && (preempt || task_state(prev) == TASK_RUNNING);

	if (runnable)
		stat_inc(OFF_CPU_STAT_RUNNABLE);
	// Leaving task (prev): stash its switch-out time, stack ids, and comm in START.
	if (tracked && !runnable &&
	    !(BPF_CORE_READ(prev, exit_state) & (EXIT_ZOMBIE | EXIT_DEAD))) {
		struct off_cpu_start start = {};
		bool kthread = BPF_CORE_READ(prev, flags) & (PF_KTHREAD | PF_IO_WORKER);
		long k = bpf_get_stackid(ctx, &KSTACK_TRACES, 0);
		long u = kthread ? -1 : bpf_get_stackid(ctx, &USTACK_TRACES, BPF_F_USER_STACK);

		stat_inc(OFF_CPU_STAT_SWITCHES_TOTAL);
		if (k == -EEXIST) {
			stat_inc(OFF_CPU_STAT_KSTACK_COLLISION);
			start.kstack_miss = OFF_CPU_STACK_COLLISION;
		} else if (k < 0) {
			stat_inc(OFF_CPU_STAT_KSTACK_FAILED);
			start.kstack_miss = OFF_CPU_STACK_FAILED;
		}
		if (kthread) {
			stat_inc(OFF_CPU_STAT_USTACK_KTHREAD);
			start.ustack_miss = OFF_CPU_STACK_NONE;
		} else if (u == -EEXIST) {
			stat_inc(OFF_CPU_STAT_USTACK_COLLISION);
			start.ustack_miss = OFF_CPU_STACK_COLLISION;
		} else if (u < 0) {
			stat_inc(OFF_CPU_STAT_USTACK_FAILED);
			start.ustack_miss = OFF_CPU_STACK_FAILED;
		}
		start.ts_ns = now;
		start.tgid = prev_tgid;
		start.kstack_id = k < 0 ? -1 : k;
		start.ustack_id = u < 0 ? -1 : u;
		bpf_get_current_comm(&start.comm, sizeof(start.comm));
		if (bpf_map_update_elem(&START, &prev_pid, &start, BPF_ANY))
			stat_inc(OFF_CPU_STAT_START_UPDATE_FAILED);
	}

	// Incoming task (next): pop its START entry and add the blocked time to COUNTS.
	struct off_cpu_start *sp = bpf_map_lookup_elem(&START, &next_pid);
	if (!sp)
		return 0;
	struct off_cpu_start s = *sp;
	bpf_map_delete_elem(&START, &next_pid);
	__u64 delta = now > s.ts_ns ? now - s.ts_ns : 0;
	stat_inc(duration_band(delta));
	if (delta < min_block_ns)
		return 0;
	stat_inc(OFF_CPU_STAT_EVENTS_TOTAL);
	stat_add(OFF_CPU_STAT_TOTAL_NS, delta);
	if (s.kstack_miss == OFF_CPU_STACK_COLLISION)
		stat_add(OFF_CPU_STAT_KSTACK_COLLISION_NS, delta);
	else if (s.kstack_miss == OFF_CPU_STACK_FAILED)
		stat_add(OFF_CPU_STAT_KSTACK_FAILED_NS, delta);
	if (s.ustack_miss == OFF_CPU_STACK_COLLISION)
		stat_add(OFF_CPU_STAT_USTACK_COLLISION_NS, delta);
	else if (s.ustack_miss == OFF_CPU_STACK_FAILED)
		stat_add(OFF_CPU_STAT_USTACK_FAILED_NS, delta);
	else if (s.ustack_miss == OFF_CPU_STACK_NONE)
		stat_add(OFF_CPU_STAT_USTACK_KTHREAD_NS, delta);
	struct off_cpu_key key = {};
	key.pid = s.tgid;
	__builtin_memcpy(key.comm, s.comm, sizeof(key.comm));
	key.kstack_id = s.kstack_id;
	key.ustack_id = s.ustack_id;
	__u64 width_ns = bucket_ns, wake_ns = now + epoch_offset_ns;
	__u64 bucket = width_ns ? wake_ns / width_ns : 0;
	__u64 in_bucket_ns = wake_ns - bucket * width_ns;
	__u64 head_ns = 0, mid_ns = 0, tail_ns = delta;

	key.bucket = bucket;
	// head: time in the first bucket if the stretch crossed a boundary; mid: in the fully
	// covered buckets between; tail: the rest, in the wake-up bucket. They sum to delta.
	if (delta > in_bucket_ns) {
		head_ns = delta - in_bucket_ns;
		tail_ns = in_bucket_ns;
		if (head_ns > width_ns) {
			__u64 mid_buckets = (head_ns - 1) / width_ns;

			key.mid_buckets = mid_buckets;
			mid_ns = mid_buckets * width_ns;
			head_ns -= mid_ns;
		}
	}
	// Several CPUs can update one key; an unused add result keeps 5.10's legacy XADD.
	struct off_cpu_value *v = bpf_map_lookup_elem(&COUNTS, &key);
	if (!v) {
		struct off_cpu_value init = {
			.switch_in_ns = now, .head_ns = head_ns,
			.mid_ns = mid_ns, .tail_ns = tail_ns,
		};

		if (!bpf_map_update_elem(&COUNTS, &key, &init, BPF_NOEXIST))
			return 0;
		// Before 6.4 the error is a zero-extended int, so re-look-up on any error.
		v = bpf_map_lookup_elem(&COUNTS, &key);
		if (!v) {
			stat_inc(OFF_CPU_STAT_COUNTS_UPDATE_FAILED);
			return 0;
		}
	}
	if (head_ns)
		__sync_fetch_and_add(&v->head_ns, head_ns);
	if (mid_ns)
		__sync_fetch_and_add(&v->mid_ns, mid_ns);
	__sync_fetch_and_add(&v->tail_ns, tail_ns);
	return 0;
}
