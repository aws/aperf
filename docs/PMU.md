# APerf PMU Data Collection

Modern CPUs are equipped with a Performance Monitoring Unit (PMU), which is a small piece of hardware that counts micro-architectural events, such as instructions retired, cycles, cache and TLB misses, branch mispredictions, pipeline stalls, and more. Metrics derived from these counter values provide significant insights into why a workload is slow, making PMU data one of the most valuable metrics that APerf collects.

The PMU data is collected through Linux's `perf_event_open` system call. For events to be collected, APerf opens counters on each CPU (all online CPUs by default unless otherwise specified by the `--pmu-cpus` flag), which are programmed by the kernel into one of the PMU's hardware counter registers, and APerf reads the accumulated counts at every collection interval to compute the metrics shown in the report. Because each CPU core only has a small number of counter registers (typically 2-8), when there are more events than available registers to collect, kernel is required to time-share the registers among the counters ([multiplexing](#multiplexing)).


## PMU Config

APerf reads JSON based config files to decide what PMU events to collect and what metrics to compute and show in the report.

Based on the platform and processor type, APerf is compiled with a list of [default configs](/src/pmu_configs/). To customize the PMU collection, users can create a custom config and pass it to APerf through `aperf record --pmu-config <file>`.

A PMU config is of the following format:

```json
{
  "events": {
    "Instructions": "armv8_pmuv3_0/event=0x8/",
    "Cycles":       "armv8_pmuv3_0/event=0x11/",
    "Branches":     "armv8_pmuv3_0/event=0x10/"
  },
  "metrics": {
    "ipc":         "Instructions / Cycles",
    "branch-mpki": "Branches / Instructions * 1000"
  }
}
```

- **`events`** is a map from an user-chosen event name to a the event's configuration string in the format of `pmu/field=value,.../`, where each field corresponds to an entry located at `/sys/bus/event_source/devices/<pmu>/format/`. For the configuration values, please refer to the [full PMU event lists](#full-pmu-event-lists).
- **`metrics`** is a map from an user-chosen metric name to a formula over the event names defined above. The formulas are arithmetic expressions (`+ - * /`, parentheses, constants).

When choosing the metric names, please note that APerf has a list of preferred names for which help information and optimization guides are configured in the report:
* `ipc`
* `stall-frontend-pkc`
* `stall-backend-pkc`
* `branch-mpki`
* `inst-l1-mpki`
* `data-l1-mpki`
* `l2-mpki`
* `l3-mpki`
* `inst-tlb-mpki`
* `inst-tlb-tw-pki`
* `data-tlb-mpki`
* `data-tlb-tw-pki`
* `data-rd-tlb-mpki`
* `data-rd-tlb-tw-pki`
* `data-st-tlb-mpki`
* `data-st-tlb-tw-pki`
* `code-sparsity`
* `strex-spec-pki`


## Collection Mode

APerf collects the PMU events in one of two possible modes, **grouped** and **ungrouped**.

In **grouped mode (default)**, APerf creates a PMU counter group for each defined metric, and a PMU counter for each event used by the metric. Since counters within a group are collected on PMU atomically, this mode guarantees that the computed metric value is accurate for the time the group is collected. However, this mode also means that multiple counters might be created for a single event, depending on the metric definitions, which increases the level of [multiplexing](#multiplexing) and reduces the actual time that a group is being collected by the PMU.

In **ungrouped mode (enabled through `aperf record --ungroup-pmu-events`)**, APerf creates a PMU counter for each defined event and schedules all counters on the PMU with no groups. This means that if there are more counters than the available PMU registers ([multiplexing](#multiplexing) is present), there is no gaurantee that the events used by a metric are collected at the same time. However, if the number of events fit in available PMU registers, this mode should be used to avoid [multiplexing](#multiplexing) and accuracy concerns that might otherwise incurred by the grouped mode.

To account for [multiplexing](#multiplexing), APerf always scale all metric values as if they were being collected the whole time. However, please choose the collection mode based on the config (the effective number of counters to schedule on the PMU) to minimize the inaccuracy. 

## Multiplexing

A CPU core typically has 2 to 8 general-purpose PMU counter registers (the exact number varies by processor and EC2 instance size), and they are shared among all processes. When the counters to collect exceed the available registers, the kernel rotates the counters through the registers in time slices, so each counter is only counting for a fraction of the collection interval.

Note that some events are counted on **dedicated counters** instead of the general-purpose registers, so effectively not consuming the budget — but only on **full-PMU (metal / dedicated-socket) instance sizes**:

- **Intel** provides fixed counters for instructions retired
  (`event=0xc0,umask=0x0`), core cycles (`event=0x3c,umask=0x0`), reference
  cycles, and topdown slots.
- **ARM** has a dedicated cycle counter for the `Cycles` event (`event=0x11`)
  on metal sizes.
- **AMD** has no fixed counters on any size.

The general-purpose register count itself also shrinks on smaller instance sizes, and differently per vendor (e.g. when measured on large instances: Intel exposes the full 8, AMD ~5 of 6, Graviton4 only 2 of 6). A config that avoids multiplexing on metal can therefore multiplex heavily on a small size of the same family.

Also note that on Intel, some events can only be counted on a **subset** of the general-purpose registers (e.g. `MEM_LOAD_RETIRED.*` events are restricted to
the first four registers; see the `Counter` field in the [intel/perfmon](https://github.com/intel/perfmon) event lists). A config can therefore multiplex even when the event count fits the total register budget, if too many events compete for the same restricted registers. AMD core events have no such restrictions.

To compensate for multiplexing, the kernel reports alongside each counter the time it was enabled and the time it actually ran. APerf uses them to scale every value as if the counter were collected for the whole interval, i.e. `value * time_enabled / time_running`. This estimate is accurate when the workload behaves steadily, but degrades on bursty workloads.

Multiplexing also consumes extra CPU during collection, since the kernel spends time rotating the context.

## PMU Support on EC2

Not all instance sizes support PMU event collection. Generally instance sizes which have an entire dedicated socket have full access to all PMU events, and smaller instance sizes of the newer generations have a reduced set of events suitable for most profiling needs. For older generations smaller instance sizes may not support any PMU event collection. Also, less PMU registers are available on smaller instance sizes. For details, please see the [Graviton Perf Runbook](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#how-to-collect-pmu-counters).

## Full PMU Event Lists

This section contains the processor type, core type, and full list of supported PMU events for each EC2 type.

### ARM

| Instance families | Processor | Core | Full PMU event list |
|---|---|---|---|
| a1 | AWS Graviton | Cortex-A72 | [cortex-a57-a72](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/arm64/arm/cortex-a57-a72) |
| m6g\*, c6g\*, r6g\*, t4g, x2gd, g5g, i4g, im4gn, is4gen | AWS Graviton2 | Neoverse N1 | [neoverse-n1.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_n1_r4p1_pmu.json) |
| m7g\*, c7g, c7gd, r7g\* | AWS Graviton3 | Neoverse V1 | [neoverse-v1.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_v1_r1p2_pmu.json) |
| c7gn, hpc7g | AWS Graviton3E | Neoverse V1 | [neoverse-v1.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_v1_r1p2_pmu.json) |
| m8g\*, c8g\*, r8g\*, x8g, i8g\* | AWS Graviton4 | Neoverse V2 | [neoverse-v2.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_v2_r0p0_pmu.json) |
| p6e-gb200 | NVIDIA Grace CPU | Neoverse V2 | [neoverse-v2.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_v2_r0p0_pmu.json) |
| m9g\*, c9g\* | AWS Graviton5 | Neoverse V3 | [neoverse-v3.json](https://gitlab.arm.com/telemetry-solution/telemetry-solution/-/blob/main/data/pmu/cpu/specifications/neoverse/neoverse_v3_r0p0_pmu.json) |

### Intel

| Instance families | Processor | Core | Full PMU event list |
|---|---|---|---|
| c3, m3, r3, i2 | Xeon E5-2670v2 / E5-2680v2 | Ivy Bridge-EP | [IVT](https://github.com/intel/perfmon/tree/main/IVT/events) |
| c4, d2, m4 (other sizes), x1\* | Xeon E5-2666v3 / E5-2676v3 / E7-8880v3 | Haswell-EP/EX | [HSX](https://github.com/intel/perfmon/tree/main/HSX/events) |
| m4.16xlarge, r4, i3, h1, g3, p3 | Xeon E5-2686 v4 | Broadwell-EP | [BDX](https://github.com/intel/perfmon/tree/main/BDX/events) |
| m5, m5d, r5, r5d, t3, i3en, z1d, p3dn | Xeon Platinum 8175 / 8151 (z1d) | Skylake-SP | [SKX](https://github.com/intel/perfmon/tree/main/SKX/events) |
| c5 (other sizes), c5d (other sizes), c5n | Xeon Platinum 8124M | Skylake-SP | [SKX](https://github.com/intel/perfmon/tree/main/SKX/events) |
| c5 (12xl/24xl/metal), c5d (12xl/24xl/metal) | 2nd Gen Xeon Platinum 8275CL | Cascade Lake | [CLX](https://github.com/intel/perfmon/tree/main/CLX/events) |
| m5n, m5dn, r5n, r5dn, r5b, m5zn, x2iezn, d3\* | Xeon Platinum 8259 / 8252 | Cascade Lake | [CLX](https://github.com/intel/perfmon/tree/main/CLX/events) |
| g4dn, inf1, vt1, dl1, dl2q, p4d\* | Xeon P-8259L / 8275CL | Cascade Lake | [CLX](https://github.com/intel/perfmon/tree/main/CLX/events) |
| u-3tb1, u-6tb1 | Xeon Platinum 8176M | Skylake-SP | [SKX](https://github.com/intel/perfmon/tree/main/SKX/events) |
| m6i\*, c6i\*, r6i\*, i4i, x2idn, x2iedn, hpc6id, trn1\* | Xeon Ice Lake (8375C) | Ice Lake-SP | [ICX](https://github.com/intel/perfmon/tree/main/ICX/events) |
| m7i\*, c7i\*, r7i\*, u7i\*, p5en, trn2\* | Xeon Sapphire Rapids | Sapphire Rapids | [SPR](https://github.com/intel/perfmon/tree/main/SPR/events) |
| i7i\*, g7e, p6-b200, p6-b300 | Xeon Emerald Rapids | Emerald Rapids | [EMR](https://github.com/intel/perfmon/tree/main/EMR/events) |
| m8i\*, c8i\*, r8i\*, x8i, g7 | Xeon 6 (Granite Rapids) | Granite Rapids | [GNR](https://github.com/intel/perfmon/tree/main/GNR/events) |

### AMD

| Instance families | Processor | Core | Full PMU event list |
|---|---|---|---|
| m5a\*, r5a\*, t3a | EPYC 7571 | Zen1 / Naples | [amdzen1](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen1) |
| c5a\*, g4ad, g5 | 2nd Gen EPYC 7R32 | Zen2 / Rome | [amdzen2](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen2) |
| m6a, c6a, r6a, hpc6a, f2, g6\*, gr6\*, inf2, p5, p5e | EPYC 7R13 | Zen3 / Milan | [amdzen3](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen3) |
| m7a, c7a, r7a, hpc7a | EPYC 9R14 | Zen4 / Genoa | [amdzen4](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen4) |
| m8a, c8a, r8a, hpc8a | EPYC 9R45 | Zen5 / Turin | [amdzen5](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen5) |
| m8azn, x8aedz | EPYC 9R05 | Zen5 / Turin | [amdzen5](https://github.com/torvalds/linux/tree/master/tools/perf/pmu-events/arch/x86/amdzen5) |
