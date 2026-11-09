/* SPDX-License-Identifier: GPL-2.0-only OR MIT */
/* Minimal kernel integer typedefs, constants, and the task_struct fields the
 * program reads, so off_cpu.bpf.c compiles with -nostdinc against only the libbpf
 * helper headers (no libc or full kernel headers). */
#ifndef APERF_KERNEL_TYPES_H
#define APERF_KERNEL_TYPES_H

typedef unsigned char __u8;
typedef short unsigned int __u16;
typedef int __s32;
typedef unsigned int __u32;
typedef long long int __s64;
typedef long long unsigned int __u64;

typedef __u16 __be16;
typedef __u32 __be32;
typedef __u32 __wsum;

typedef int pid_t;
typedef _Bool bool;

enum bpf_map_type {
	BPF_MAP_TYPE_HASH = 1,
	BPF_MAP_TYPE_PERCPU_ARRAY = 6,
	BPF_MAP_TYPE_STACK_TRACE = 7,
};

enum {
	BPF_ANY = 0,
	BPF_NOEXIST = 1,
};

enum {
	BPF_F_USER_STACK = (1ULL << 8),
};

#define PERF_MAX_STACK_DEPTH 127
#define EEXIST 17
#define EXIT_DEAD 0x0010
#define EXIT_ZOMBIE 0x0020
#define PF_IO_WORKER 0x00000010
#define PF_KTHREAD 0x00200000
#define TASK_RUNNING 0x0000

struct task_struct {
	unsigned int __state;
	unsigned int flags;
	int exit_state;
	pid_t pid;
	pid_t tgid;
} __attribute__((preserve_access_index));

/* task_struct before 5.14, where __state was "long state". CO-RE ignores the ___pre514
 * suffix when matching kernel BTF, so task_state() reads whichever field exists. */
struct task_struct___pre514 {
	long state;
} __attribute__((preserve_access_index));

#endif
