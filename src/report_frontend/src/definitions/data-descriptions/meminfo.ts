import type { DataDescription } from "../data-descriptions";
import {
  DIRTY_WRITEBACK_INVESTIGATION,
  MEMORY_USAGE_INVESTIGATION,
  PAGE_TABLE_OVERHEAD_INVESTIGATION,
} from "./optimization-guides";
import { HUGE_PAGE_SIZES_KB } from "./mem-settings";

function perSizeHugetlbPoolFieldDescriptions(): DataDescription["fieldDescriptions"] {
  const fieldDescriptions: DataDescription["fieldDescriptions"] = {};
  for (const sizeKb of HUGE_PAGE_SIZES_KB) {
    fieldDescriptions[`HugePages_Total_${sizeKb}kB`] = {
      readableName: `Total ${sizeKb}kB Huge Pages`,
      description: `Number of ${sizeKb}kB pages in the HugeTLB pool of that size. This is the per-size twin of HugePages_Total, which covers only the default huge page size.`,
      desired: "depends",
      unit: "Pages",
    };
    fieldDescriptions[`HugePages_Free_${sizeKb}kB`] = {
      readableName: `Free ${sizeKb}kB Huge Pages`,
      description: `Number of ${sizeKb}kB HugeTLB pool pages currently available for allocation. This is the per-size twin of HugePages_Free.`,
      desired: "higher",
      unit: "Pages",
    };
    fieldDescriptions[`HugePages_Rsvd_${sizeKb}kB`] = {
      readableName: `Reserved ${sizeKb}kB Huge Pages`,
      description: `Number of ${sizeKb}kB HugeTLB pool pages reserved but not yet allocated to applications. This is the per-size twin of HugePages_Rsvd.`,
      desired: "lower",
      unit: "Pages",
    };
    fieldDescriptions[`HugePages_Surp_${sizeKb}kB`] = {
      readableName: `Surplus ${sizeKb}kB Huge Pages`,
      description: `Number of ${sizeKb}kB huge pages allocated beyond that pool's configured size. This is the per-size twin of HugePages_Surp.`,
      desired: "lower",
      unit: "Pages",
    };
  }
  return fieldDescriptions;
}

export const MEMINFO_DATA_DESCRIPTION: DataDescription = {
  readableName: "Memory Usage",
  summary:
    "Memory usage metrics measure the usage of the system's physical memory. The data were collected from the system pseudo-file /proc/meminfo.",
  defaultUnit: "Bytes",
  defaultHelpfulLinks: ["https://man7.org/linux/man-pages/man5/proc_meminfo.5.html"],
  fieldDescriptions: {
    VmallocUsed: {
      readableName: "Virtual Memory Allocated Used",
      description:
        "Amount of virtual memory currently allocated through the vmalloc interface for kernel data structures and device drivers.",
      desired: "moderate",
    },
    Cached: {
      readableName: "Page Cache Memory",
      description:
        "Memory used by the kernel to cache file system data and metadata to improve I/O performance by reducing disk access.",
      desired: "moderate",
    },
    MemFree: {
      readableName: "Free Memory",
      description:
        "Amount of memory sitting on the kernel's free lists, in use by nobody. Linux deliberately spends spare memory on caching files, so a low value is the normal healthy state and not a sign of a problem. MemAvailable is the metric that answers how much a new workload could get.",
      desired: "higher",
    },
    FilePmdMapped: {
      readableName: "File PMD Mapped",
      description:
        "File-backed memory pages mapped using Page Middle Directory entries for large page optimizations in memory management.",
      desired: "lower",
    },
    DirectMap2M: {
      readableName: "Direct Map 2M Pages",
      description: "Number of bytes of RAM linearly mapped by kernel in 2 MB pages.",
      desired: "higher",
    },
    Unevictable: {
      readableName: "Unevictable Memory",
      description:
        "Memory the kernel cannot reclaim or swap, which is the mlocked memory counted in Mlocked plus pages of ramfs and of shared memory segments locked with SHM_LOCK.",
      desired: "lower",
    },
    Percpu: {
      readableName: "Per-CPU Memory",
      description:
        "Memory allocated on a per-CPU basis for CPU-local data structures to avoid cache line contention in multi-processor systems.",
      desired: "moderate",
    },
    AnonHugePages: {
      readableName: "Anonymous Huge Pages",
      description:
        "Anonymous memory backed by PMD-sized transparent huge pages. This is the meminfo twin of the vmstat metric nr_anon_transparent_hugepages, reported as a size rather than a page count, and like it, it counts only the PMD size and not the smaller multi-size huge pages.",
      desired: "depends",
    },
    "Inactive(file)": {
      readableName: "Inactive File Pages",
      description:
        "File-backed memory pages in the inactive LRU list that are candidates for reclamation when memory pressure occurs.",
      desired: "moderate",
    },
    CmaTotal: {
      readableName: "CMA Total Memory",
      description:
        "Total Contiguous Memory Allocator memory reserved for devices requiring physically contiguous memory blocks.",
      desired: "higher",
    },
    SwapFree: {
      readableName: "Free Swap Space",
      description:
        "Available swap space that can be used when physical memory is exhausted for virtual memory management.",
      desired: "higher",
    },
    CmaFree: {
      readableName: "CMA Free Memory",
      description:
        "Free pages in the Contiguous Memory Allocator pool available for devices requiring physically contiguous memory blocks.",
      desired: "higher",
    },
    Committed_AS: {
      readableName: "Committed Memory",
      description:
        "Total amount of memory currently allocated or reserved by processes including virtual memory that may not be physically present.",
      desired: "moderate",
    },
    Inactive: {
      readableName: "Inactive Memory",
      description:
        "Total memory pages in the inactive LRU lists that are candidates for reclamation when memory pressure occurs.",
      desired: "moderate",
    },
    CommitLimit: {
      readableName: "Commit Limit",
      description:
        "The commit ceiling, computed as vm.overcommit_ratio percent of RAM plus swap. It is only enforced when vm.overcommit_memory is 2; under the default of 0 the kernel ignores it, so Committed_AS routinely exceeds it without anything being wrong.",
      desired: "higher",
    },
    SUnreclaim: {
      readableName: "Slab Unreclaimable",
      description:
        "Kernel slab memory holding objects that are in use and so cannot be reclaimed under memory pressure. It is freed when the kernel releases the objects themselves, which shrinking a cache can trigger.",
      desired: "lower",
    },
    MemTotal: {
      readableName: "Total Memory",
      description:
        "Total amount of physical memory available to the system including memory used by kernel and applications.",
      desired: "higher",
    },
    Slab: {
      readableName: "Slab Memory",
      description: "Total kernel slab memory used for caching frequently used kernel objects and data structures.",
      desired: "moderate",
    },
    DirectMap4k: {
      readableName: "Direct Map 4K Pages",
      description: "Number of bytes of RAM linearly mapped by kernel in 4 kB pages.",
      desired: "moderate",
    },
    SwapTotal: {
      readableName: "Total Swap Space",
      description:
        "Total amount of swap space available for virtual memory management when physical memory is exhausted.",
      desired: "higher",
    },
    Shmem: {
      readableName: "Shared Memory",
      description:
        "Memory used by shared memory segments, meaning tmpfs, /dev/shm and System V shared memory. The kernel cannot drop it the way it drops page cache, only swap it, and it does not appear in any process's resident set size, so a large value is easy to miss when accounting for memory per process.",
      desired: "moderate",
    },
    "Active(file)": {
      readableName: "Active File Pages",
      description:
        "File-backed memory pages in the active LRU list that are frequently accessed and less likely to be reclaimed.",
      desired: "moderate",
    },
    MemAvailable: {
      readableName: "Available Memory",
      description:
        "The kernel's estimate of how much memory a new workload could allocate without pushing the system into swapping. It is MemFree, less the watermarks the kernel keeps in reserve, plus the page cache and reclaimable slab that could be given up on demand, which is why it is normally larger than MemFree.",
      desired: "higher",
      optimization: [MEMORY_USAGE_INVESTIGATION],
    },
    SwapCached: {
      readableName: "Swap Cache",
      description:
        "Memory that was swapped out but is now back in RAM and still cached in case it needs to be swapped out again.",
      desired: "lower",
    },
    ShmemPmdMapped: {
      readableName: "Shared Memory PMD Mapped",
      description:
        "Shared memory pages mapped using Page Middle Directory entries for large page optimizations in shared memory segments.",
      desired: "lower",
    },
    HugePages_Total: {
      readableName: "Total Huge Pages",
      description:
        "Total number of huge pages configured in the system for applications requiring large contiguous memory blocks.",
      desired: "depends",
      unit: "Pages",
    },
    KernelStack: {
      readableName: "Kernel Stack Memory",
      description: "Memory used by kernel stacks for each thread and process in the system.",
      desired: "moderate",
    },
    HugePages_Rsvd: {
      readableName: "Reserved Huge Pages",
      description:
        "Number of huge pages an application has committed to using but has not faulted in yet. The kernel guarantees these will be available to it, and they are a subset of HugePages_Free rather than a separate quantity.",
      desired: "lower",
      unit: "Pages",
    },
    NFS_Unstable: {
      readableName: "NFS Unstable Pages",
      description: "Pages that have been written to NFS server but not yet committed to stable storage.",
      desired: "lower",
    },
    KReclaimable: {
      readableName: "Kernel Reclaimable",
      description: "Kernel memory that can be reclaimed when the system is under memory pressure.",
      desired: "moderate",
    },
    HugePages_Surp: {
      readableName: "Surplus Huge Pages",
      description: "Number of huge pages allocated beyond the configured pool size.",
      desired: "lower",
      unit: "Pages",
    },
    ShmemHugePages: {
      readableName: "Shared Memory Huge Pages",
      description: "Shared memory segments using huge pages for improved performance.",
      desired: "lower",
    },
    Hugetlb_Unused_Memory_Percent: {
      readableName: "Unused HugeTLB Share of Memory",
      description:
        "Memory reserved in the HugeTLB pools that no application allocated or committed to, as a percentage of MemTotal. Computed across every pool size as (HugePages_Free - HugePages_Rsvd) x page size. Reserved pages are unavailable to everything else, so this is the share of the machine the pools hold without anyone using it. Pages an application has already committed to faulting in are counted in HugePages_Rsvd and excluded, so lazily faulted allocations are not mistaken for waste.",
      desired: "lower",
      unit: "Percent",
    },
    Hugepagesize: {
      readableName: "Huge Page Size",
      description: "Size of each huge page in the system typically 2MB or 1GB.",
      desired: "fixed",
    },
    FileHugePages: {
      readableName: "File Huge Pages",
      description: "File-backed memory pages using huge page sizes for improved I/O performance.",
      desired: "lower",
    },
    DirectMap1G: {
      readableName: "Direct Map 1G Pages",
      description: "Number of bytes of physical memory directly mapped using 1GB pages for maximum TLB efficiency.",
      desired: "higher",
    },
    Bounce: {
      readableName: "Bounce Buffer Memory",
      description: "Memory used for bounce buffers when DMA cannot directly access certain memory regions.",
      desired: "lower",
    },
    DirectMap4M: {
      readableName: "Direct Map 4M Pages",
      description:
        "Number of bytes of physical memory directly mapped using 4MB pages for improved TLB efficiency on some architectures.",
      desired: "higher",
    },
    Mlocked: {
      readableName: "Memory Locked",
      description:
        "Memory an application has locked into RAM with mlock or mlockall. The kernel can neither swap nor reclaim it, so it stays consumed no matter how much memory pressure builds.",
      desired: "lower",
    },
    Writeback: {
      readableName: "Writeback Memory",
      description:
        "Amount of memory holding data that has been handed to the storage device and whose write has not yet completed. This is the same quantity as the vmstat metric nr_writeback, reported as a size rather than a page count.",
      desired: "lower",
    },
    SReclaimable: {
      readableName: "Slab Reclaimable",
      description: "Kernel slab memory that can be reclaimed when the system is under memory pressure.",
      desired: "moderate",
    },
    VmallocTotal: {
      readableName: "Virtual Memory Allocator Total",
      description: "Total virtual address space available for vmalloc allocations by the kernel.",
      desired: "higher",
    },
    HardwareCorrupted: {
      readableName: "Hardware Corrupted Memory",
      description: "Memory pages marked as corrupted due to hardware errors and excluded from use.",
      desired: "lower",
    },
    AnonPages: {
      readableName: "Anonymous Pages",
      description: "Anonymous memory pages not backed by files including process heap stack and anonymous mappings.",
      desired: "depends",
    },
    Active: {
      readableName: "Active Memory",
      description:
        "Total memory pages in active LRU lists that are frequently accessed and less likely to be reclaimed.",
      desired: "moderate",
    },
    "Active(anon)": {
      readableName: "Active Anonymous Memory",
      description: "Anonymous memory pages in the active LRU list that are frequently accessed.",
      desired: "moderate",
    },
    "Inactive(anon)": {
      readableName: "Inactive Anonymous Memory",
      description: "Anonymous memory pages in the inactive LRU list that are candidates for swapping.",
      desired: "moderate",
    },
    MmapCopy: {
      readableName: "Memory Map Copy",
      description: "Memory used for copy-on-write mappings during memory management operations.",
      desired: "lower",
    },
    WritebackTmp: {
      readableName: "Temporary Writeback Memory",
      description:
        "Memory held in temporary buffers while being written back, which is used by filesystems such as FUSE. This is the same quantity as the vmstat metric nr_writeback_temp, reported as a size rather than a page count.",
      desired: "lower",
    },
    Quicklists: {
      readableName: "Quicklist Memory",
      description: "Memory used by quicklists for fast allocation and deallocation of kernel objects.",
      desired: "lower",
    },
    Hugetlb: {
      readableName: "Huge TLB Memory",
      description:
        "Total memory held by the HugeTLB pools, summed over every configured page size. This memory is set aside for applications that ask for huge pages explicitly and is unavailable to everything else, whether or not any application is using it.",
      desired: "depends",
    },
    Buffers: {
      readableName: "Buffer Memory",
      description: "Memory used by the kernel for buffering block device I/O operations.",
      desired: "moderate",
    },
    Dirty: {
      readableName: "Dirty Memory",
      description:
        "Amount of memory holding data that an application has written but that the kernel has not yet sent to storage. This is the same quantity as the vmstat metric nr_dirty, reported as a size rather than a page count.",
      desired: "depends",
      optimization: [DIRTY_WRITEBACK_INVESTIGATION],
    },
    VmallocChunk: {
      readableName: "Virtual Memory Allocator Chunk",
      description: "Largest contiguous chunk of virtual address space available for vmalloc allocations.",
      desired: "higher",
    },
    PageTables: {
      readableName: "Page Table Memory",
      description:
        "Memory used by page tables for virtual to physical address translation. Every process pays for the memory it maps, so this grows with the number of processes multiplied by the size each one maps, and huge pages shrink it by covering more memory per entry.",
      desired: "lower",
      optimization: [PAGE_TABLE_OVERHEAD_INVESTIGATION],
    },
    PageTables_Memory_Percent: {
      readableName: "Page Table Share of Memory",
      description:
        "Memory used by page tables as a percentage of MemTotal. This is address translation overhead rather than memory the workload asked for, so a large share is worth reducing.",
      desired: "lower",
      unit: "Percent",
      optimization: [PAGE_TABLE_OVERHEAD_INVESTIGATION],
    },
    Mapped: {
      readableName: "Mapped Memory",
      description: "Memory pages mapped into process address spaces for files and shared libraries.",
      desired: "moderate",
    },
    HugePages_Free: {
      readableName: "Free Huge Pages",
      description:
        "Number of huge pages in the pool that are not yet allocated. This includes the pages counted in HugePages_Rsvd, which an application has already committed to using, so the pages genuinely spare are HugePages_Free minus HugePages_Rsvd.",
      desired: "higher",
      unit: "Pages",
    },
    ...perSizeHugetlbPoolFieldDescriptions(),
  },
};
