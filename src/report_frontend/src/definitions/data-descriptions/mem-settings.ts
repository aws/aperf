import type { DataDescription } from "../data-descriptions";
import { MEMORY_COMPACTION_INVESTIGATION, TLB_MISS_OPTIMIZATION } from "./optimization-guides";

/**
 * The huge page sizes, in kB, that /sys/kernel/mm can expose a directory for, used to
 * craete description entries for the per=size metrics and settings.
 */
export const HUGE_PAGE_SIZES_KB = [
  8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144, 524288, 1048576, 2097152,
  4194304, 8388608, 16777216,
];

function perSizeFieldDescriptions(): DataDescription["fieldDescriptions"] {
  const fieldDescriptions: DataDescription["fieldDescriptions"] = {};
  for (const sizeKb of HUGE_PAGE_SIZES_KB) {
    fieldDescriptions[`transparent_hugepage/hugepages-${sizeKb}kB/enabled`] = {
      readableName: `${sizeKb}kB Transparent Huge Pages`,
      description: `Whether the kernel may back anonymous memory with ${sizeKb}kB transparent huge pages. It is one of always, madvise, never, or inherit to follow the top-level enabled setting. A size smaller than hpage_pmd_size needs less contiguous memory, so the kernel can satisfy it more often.`,
      optimization: [TLB_MISS_OPTIMIZATION],
    };
    fieldDescriptions[`transparent_hugepage/hugepages-${sizeKb}kB/shmem_enabled`] = {
      readableName: `${sizeKb}kB Shared Memory Huge Pages`,
      description: `Whether the kernel may back shared memory, meaning tmpfs and shmem, with ${sizeKb}kB huge pages. This is a separate policy from the enabled setting of the same size, which covers anonymous memory only, and it takes always, within_size to use huge pages only up to the size the file was given, advise for ranges that asked through madvise, never, deny, force, or inherit to follow the top-level shmem_enabled setting.`,
      optimization: [TLB_MISS_OPTIMIZATION],
    };
    fieldDescriptions[`hugepages/hugepages-${sizeKb}kB/nr_hugepages`] = {
      readableName: `${sizeKb}kB HugeTLB Pool Size`,
      description: `The number of ${sizeKb}kB pages reserved in the HugeTLB pool of that size. HugeTLB pages are set aside up front and are not transparent huge pages: only applications that ask for them explicitly, through hugetlbfs, mmap(MAP_HUGETLB) or shmget(SHM_HUGETLB), can use them, and the memory is unavailable to everything else.`,
    };
    fieldDescriptions[`hugepages/hugepages-${sizeKb}kB/nr_overcommit_hugepages`] = {
      readableName: `${sizeKb}kB HugeTLB Overcommit Limit`,
      description: `How many ${sizeKb}kB pages the kernel may allocate on demand once the ${sizeKb}kB HugeTLB pool is exhausted. These allocations succeed only when enough contiguous memory happens to be free at the time, and are returned to the system when freed.`,
    };
  }
  return fieldDescriptions;
}

export const MEM_SETTINGS_DATA_DESCRIPTION: DataDescription = {
  readableName: "Memory Settings",
  summary:
    "Memory settings contain the static memory configuration read once from /sys/kernel/mm: the Transparent Huge Pages policy, the tunables of the khugepaged thread that collapses regular pages into huge pages in the background, and the pre-reserved HugeTLB page pools. Which page sizes appear depends on the architecture and the kernel's base page size - arm64 exposes more HugeTLB pool sizes than x86_64 - and a page size only lists the policies that apply to it, so a size usable for shared memory but not for anonymous memory has shmem_enabled without enabled. A setting that exists but could not be read is shown with an empty value.",
  defaultHelpfulLinks: [
    "https://docs.kernel.org/admin-guide/mm/transhuge.html",
    "https://docs.kernel.org/admin-guide/mm/hugetlbpage.html",
  ],
  fieldDescriptions: {
    "transparent_hugepage/enabled": {
      readableName: "Transparent Huge Pages",
      description:
        "Whether the kernel backs anonymous memory with PMD-sized transparent huge pages: always for every process, madvise only for the ranges a process asked for with madvise(MADV_HUGEPAGE), or never.",
      optimization: [TLB_MISS_OPTIMIZATION],
      helpfulLinks: [
        "https://aws.github.io/graviton/perfrunbook/optimization_recommendation.html#optimizing-for-high-tlb-miss-rates",
      ],
    },
    "transparent_hugepage/defrag": {
      readableName: "Huge Page Allocation Effort",
      description:
        "How hard the kernel works to obtain a huge page when none is immediately available: always for the faulting process to compact memory itself and stall until it finishes, defer to leave the work to the background kcompactd thread and fall back to regular pages, defer+madvise for defer except that madvised regions stall, madvise for only madvised regions to stall, or never.",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    "transparent_hugepage/shmem_enabled": {
      readableName: "Transparent Huge Pages for Shared Memory",
      description:
        "Whether the kernel backs shared memory and tmpfs files with huge pages: always, within_size to go no further than the file's size, advise only where madvise asked for it, never, deny for every mount, or force for every file regardless of size.",
    },
    "transparent_hugepage/use_zero_page": {
      readableName: "Huge Zero Page",
      description:
        "Whether a read fault on untouched anonymous memory is satisfied by one shared huge zero page (1) instead of the regular zero page (0). It saves faults on sparsely read memory at the cost of keeping a PMD-sized page allocated while it is in use.",
    },
    "transparent_hugepage/hpage_pmd_size": {
      readableName: "PMD Huge Page Size",
      description:
        "The size in bytes of a PMD-sized transparent huge page on this kernel - 2097152 on a 4kB-page kernel, and 536870912 on an arm64 64kB-page kernel. This is the size the top-level enabled and defrag settings apply to.",
    },
    "transparent_hugepage/shrink_underused": {
      readableName: "Shrink Underused Huge Pages",
      description:
        "Whether the kernel may split a transparent huge page whose pages are mostly unused and return the unused part to the system when memory is short (1), or keep it whole (0). Splitting recovers memory that a partially used huge page would otherwise hold, at the cost of losing the huge mapping.",
    },
    "transparent_hugepage/khugepaged/defrag": {
      readableName: "khugepaged Allocation Effort",
      description:
        "Whether khugepaged may compact memory to obtain a huge page while collapsing regular pages in the background (1), or must give up when no huge page is readily available (0). This work happens in khugepaged and not in the application, unlike the transparent_hugepage defrag setting.",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    "transparent_hugepage/khugepaged/max_ptes_none": {
      readableName: "khugepaged Max Untouched Pages",
      description:
        "How many of the pages in a region khugepaged is collapsing may be untouched. A higher value collapses more regions, at the cost of backing memory the application never faulted in.",
    },
    "transparent_hugepage/khugepaged/max_ptes_shared": {
      readableName: "khugepaged Max Shared Pages",
      description:
        "How many of the pages in a region khugepaged is collapsing may be shared with another process. Collapsing copies those pages and so breaks the sharing, which a lower value avoids.",
    },
    "transparent_hugepage/khugepaged/max_ptes_swap": {
      readableName: "khugepaged Max Swapped Pages",
      description:
        "How many of the pages in a region khugepaged is collapsing may be swapped out. Above this the region is left alone rather than paying to read the pages back in.",
    },
    "transparent_hugepage/khugepaged/pages_to_scan": {
      readableName: "khugepaged Pages Per Scan",
      description: "How many pages khugepaged inspects in one pass before it sleeps again.",
    },
    "transparent_hugepage/khugepaged/scan_sleep_millisecs": {
      readableName: "khugepaged Scan Interval",
      description: "How long khugepaged sleeps between scan passes, in milliseconds.",
    },
    "transparent_hugepage/khugepaged/alloc_sleep_millisecs": {
      readableName: "khugepaged Allocation Retry Interval",
      description:
        "How long khugepaged waits, in milliseconds, after it fails to allocate a huge page before it tries to collapse another region.",
      optimization: [MEMORY_COMPACTION_INVESTIGATION],
    },
    ...perSizeFieldDescriptions(),
  },
};
