import type { DataDescription } from "../data-descriptions";
import { DIRTY_WRITEBACK_INVESTIGATION } from "./optimization-guides";

export const SYSCTL_DATA_DESCRIPTION: DataDescription = {
  readableName: "Sysctl Config",
  summary: "Sysctl contains runtime kernel parameters.",
  defaultHelpfulLinks: ["https://docs.kernel.org/admin-guide/sysctl/kernel.html"],
  fieldDescriptions: {
    "vm.dirty_ratio": {
      readableName: "Dirty Page Limit (Ratio)",
      description:
        "The amount of dirty pages, as a percentage of the memory available for holding dirty data, at which a process that writes is made to stop and flush data to disk itself. This is 0 when vm.dirty_bytes is used instead.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    "vm.dirty_bytes": {
      readableName: "Dirty Page Limit (Bytes)",
      description:
        "The same limit as vm.dirty_ratio, expressed as a number of bytes rather than a percentage. Setting this to a non-zero value sets vm.dirty_ratio to 0, and setting vm.dirty_ratio sets this to 0.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    "vm.dirty_background_ratio": {
      readableName: "Background Writeback Threshold (Ratio)",
      description:
        "The amount of dirty pages, as a percentage of the memory available for holding dirty data, at which the kernel starts writing them back in the background. Unlike vm.dirty_ratio, reaching this point does not stop the application from writing. This is 0 when vm.dirty_background_bytes is used instead.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    "vm.dirty_background_bytes": {
      readableName: "Background Writeback Threshold (Bytes)",
      description:
        "The same threshold as vm.dirty_background_ratio, expressed as a number of bytes rather than a percentage. Setting this to a non-zero value sets vm.dirty_background_ratio to 0, and setting vm.dirty_background_ratio sets this to 0.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    "vm.dirty_writeback_centisecs": {
      readableName: "Writeback Interval",
      description:
        "How often the kernel flusher threads wake up to write dirty data back to disk, in hundredths of a second. Setting it to 0 disables periodic writeback.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    "vm.dirty_expire_centisecs": {
      readableName: "Dirty Data Expiry",
      description:
        "How long dirty data can stay in memory, in hundredths of a second, before the kernel flusher threads write it out.",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
  },
};
