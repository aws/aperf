import type { DataDescription } from "../data-descriptions";
import { TLB_MISS_OPTIMIZATION, DIRTY_WRITEBACK_INVESTIGATION } from "./optimization-guides";

export const KERNEL_CONFIG_DATA_DESCRIPTION: DataDescription = {
  readableName: "Kernel Config",
  summary:
    'Kernel configs contain configuration options used when the running kernel was compiled. The data were collected from /boot/config* file. Value "y" means the module is compiled directly in the kernel, "not set"/"n" means the module is not compiled in the kernel, and "m" means the module is compiled as a loadable module.',
  defaultHelpfulLinks: ["https://docs.kernel.org/admin-guide/bootconfig.html"],
  fieldDescriptions: {
    CONFIG_BLK_WBT: {
      readableName: "Block Layer Writeback Throttling",
      description:
        "Writeback throttling lets the block layer hold back background writeback when it would otherwise delay other I/O. To check whether it is active on a running system, read /sys/block/<device>/queue/wbt_lat_usec, which holds the target latency throttling tries to preserve for that device - a value of 0 means it is currently inactive.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    CONFIG_BLK_WBT_MQ: {
      readableName: "Block Layer Writeback Throttling (Multi-Queue)",
      description:
        "Enables writeback throttling for the multi-queue block layer (blk-mq), which is the I/O path used by devices such as NVMe and virtio storage.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    CONFIG_HZ: {
      readableName: "Kernel Timer Tick Frequency",
      description:
        "The frequency, in ticks per second, of the kernel's periodic timer interrupt, commonly 100, 250, 300 or 1000. A lower value reduces timer interrupt overhead, while a higher value gives the scheduler finer granularity. If the kernel is tickless (see CONFIG_NO_HZ_IDLE and CONFIG_NO_HZ_FULL), the tick is suppressed on idle CPUs, so this value is an upper bound on the tick rate rather than a constant interrupt rate.",
    },
    CONFIG_TRANSPARENT_HUGEPAGE: {
      readableName: "Transparent Hugepage Support",
      description:
        "Transparent Hugepages allows the kernel to use huge pages and huge tlb transparently to the applications whenever possible. This feature can improve computing performance to certain applications by speeding up page faults during memory allocation, by reducing the number of tlb misses and by speeding up the pagetable walking.",
      optimization: [TLB_MISS_OPTIMIZATION],
      helpfulLinks: [
        "https://aws.github.io/graviton/perfrunbook/optimization_recommendation.html#optimizing-for-high-tlb-miss-rates",
      ],
    },
  },
};
