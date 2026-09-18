export const CPU_UTILIZATION_OPTIMIZATION = `
### CPU utilization investigations
#### Higher-than-expected CPU utilization
To find out which part in code is consuming more CPU time, you can perform [on-cpu profiling](https://aws.github.io/graviton/perfrunbook/debug_code_perf.html#on-cpu-profiling), which produces flamegraphs that indicate the CPU utilization of every stack trace. The profiling data is available in APerf if you used the \`--profile\` option during recording. To make sure the flamegraphs are correctly collected:
* Kernel addresses are only visible when \`kernel.kptr_restrict\` is 0 (in Sysctl Config). If it is not, set it with \`sudo sysctl -w kernel.kptr_restrict=0\` and record again.
* For native code, verify that it is built with \`-g -fno-omit-frame-pointer\`.
* For Java code, we recommend installing [async-profiler](https://github.com/async-profiler/async-profiler) and profile through APerf's \`--profile-java\` option, which provides richer data; Otherwise, ensure that the JVM is run with \`-XX:+PreserveFramePointer -agentpath:/usr/lib64/libperf-jvmti.so\`.
* For NodeJS code, verify that it is started with \`--perf-basic-prof\`. 
#### Lower-than-expected CPU utilization
Multiple factors, including lock contention, IO Bottlenecks, and OS scheduler issues, can lead to low CPU utilization. To find call stacks that are putting threads to sleep via the OS, you can perform [off-cpu profiling](https://aws.github.io/graviton/perfrunbook/debug_code_perf.html#off-cpu-profiling).
`;

export const IOWAIT_TIME_OPTIMIZATION = `
### Optimizations for high iowait time
High iowait time indicates a bottleneck in disk operations. If the host uses EBS, provision volumes with more IOPs ([optimization guide](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ebs-optimized.html)), or consider migrating to instance types with local storage (e.g. the "d." instances).
`;

export const NETWORK_USAGE_INVESTIGATION = `
### Investigate network usage
If the network throughput is not as expected, below are some steps to investigate:
* The aggregate TCP/IP counters are in the TCP/IP Stats data. APerf does not record per-connection state, so to find a single dominating connection, which can saturate one core and bottleneck the rest of the system, run \`watch netstat -t\` on the host.
* For EC2 instances, check the ENA Stats and see if ENA throttle is being hit:
    * \`bw_in_allowance_exceeded\`
    * \`bw_out_allowance_exceeded\`
    * \`conntrack_allowance_exceeded\`
    * \`linklocal_allowance_exceeded\`
    * \`pps_allowance_exceeded\`
    
    If hitting ENA throttles, provision a larger instance to get more bandwidth if possible. IO bottlenecks tend to mask any CPU performance gains.
`;

export const MEMORY_USAGE_INVESTIGATION = `
### Investigate memory usage
If the memory usage is not as expected, it is useful to exam the memory allocation profiles.

Currently, APerf only supports memory allocation profiling for Java applications (enabled through \`--profile-java\`). 
`;

export const INSTRUCTION_FOOTPRINT_OPTIMIZATION = `
### Optimizations for large instruction footprint:
* For C/C++ applications, use compiler options \`-flto\` and \`-Os\`, or look into [Feedback Directed Optimization](https://gcc.gnu.org/wiki/AutoFDO/Tutorial).
* For Java applications, JVM flags can be used to reduce instruction footprint caused by the JIT compiler
    1. Experiment with setting \`-XX:+TieredCompilation\` for faster start-up time and better code optimization.
    1. Start with setting \`-XX:ReservedCodeCacheSize=64M -XX:InitialCodeCacheSize=64M\` and then tune the sizes. Messages like \`code cache full\` indicate that the cache size needs to be increased.
`;

export const DATA_FOOTPRINT_OPTIMIZATION = `
### Optimizations for large data footprint:
The common practices of reducing data footprint include improving the temporal and spatial locality of the code, such as (if they apply)
* reuse the same data as much as possible;
* store related data in continuous memory (e.g. using array list instead of linked list);
* access memory sequentially (e.g. iterating through 2-d arrays row by row);
* break large loops into smaller one.

You can also use APerf's hotline feature (only works for native code and on metal Graviton instances) to detect hotspots in code and then insert prefetch instructions. 
`;

export const TLB_MISS_OPTIMIZATION = `
### Optimizations for high TLB misses
Since TLB is a cache that holds virtual-to-physical address translation, reducing its miss rate can improve performance. The Memory Settings data reports how huge pages are configured on the recorded host, so start there before changing anything:
* Enable Transparent Huge Pages (THP) by running
    \`\`\`shell
    echo always > /sys/kernel/mm/transparent_hugepage/enabled
    \`\`\`
    to enable THP for all processes, or
    \`\`\`shell
    echo madvise > /sys/kernel/mm/transparent_hugepage/enabled
    \`\`\`
    to enable THP for applications that opted in through making the \`madvise\` system call.
* On Linux kernels >=6.9, THP is extended with [folios](https://lwn.net/Articles/937239/) that create 16KB and 64KB huge pages in addition to the 2MB ones, allowing the Linux kernel to use huge pages in more places. The folios sizes can be modified at
    * \`/sys/kernel/mm/transparent_hugepage/hugepages-16kB/enabled\`
    * \`/sys/kernel/mm/transparent_hugepage/hugepages-64kB/enabled\`
    * \`/sys/kernel/mm/transparent_hugepage/hugepages-2048kB/enabled\`
    
    Each of them can be set to \`never\`, \`always\`, and \`madvise\`. To inherit the top-level THP setting, set their values to \`inherit\`.
* If your application can use pinned huge pages because it uses \`mmap\` directly, try reserving the huge pages directly via the OS, by either:
    * running \`sysctl -w vm.nr_hugepages=X\` (run time),
    * or adding \`hugepagesz=2M hugepages=512\` to \`/etc/default/grub\` and reboot (boot time).
* For Java applications, consider adding the following JVM flags:
    * \`-XX:+UseTransparentHugePages\` if THP preference is at least \`madvise\`
    * \`-XX:+UseLargePages\` if you have reserved huge pages through the methods above.
`;

export const LOW_IPC_INVESTIGATION = `
### Investigating low IPC
If IPC on a system is lower than another when running the same application, try to identify whether the bottleneck comes from the frontend or backend by checking the \`stall_frontend_pkc\` and \`stall_backend_pkc\` metrics.
`;

export const CPU_FRONTEND_STALLS_INVESTIGATION = `
### Investigating CPU frontend stalls
Frontend stalls are commonly due to inefficient instruction fetching, caused by either wrong branch prediction or memory access (to fetch instruction or translate instruction addresses). Check the below metrics to further investigate the root cause:
* \`branch-mpki\`
* \`inst-l1-mpki\`
* \`inst-tlb-mpki\`
* \`inst-tlb-tw-pki\`
* \`code-sparsity\`
`;

export const CPU_BACKEND_STALLS_INVESTIGATION = `
### Investigating CPU backend stalls
Backend stalls are commonly due to slow executions of the instructions, which are usually caused by excessive memory access to fetch the data or translate their addresses. Check the below metrics to further investigate the root cause:
* \`data-l1-mpki\`
* \`l2-mpki\`
* \`l3-mpki\`
* \`data-tlb-mpki\`
* \`data-tlb-tw-pki\`
`;

export const LSE_OPTIMIZATION = `
### Enable Large-System Extensions (LSE)
For faster atomic operations, the compiler needs to generate LSE instructions instead of load/store exclusives (if the processor supports it). The below two GCC flags should be used:
* \`-march=armv8.2-a\` enables all instructions supported by the corresponding ARM processor. Find all possible values and more instructions for the \`-march\` flag [here](https://gcc.gnu.org/onlinedocs/gcc/AArch64-Options.html#index-march).
* \`-mno-outline-atomics\` enables calls to out-of-line helpers to implement atomic operations and uses the LSE instructions if they are available.
For natively-built Rust binary, can use \`export RUSTFLAGS="-Ctarget-features=+lse"\` for code that runs on all ARM platforms with LSE supports. 
`;

export const EC2_NETWORK_BANDWIDTH_ALLOWANCE_RECOMMENDATIONS = `
### Working with EC2 network bandwidth allowances
Every EC2 instance has bandwidth maximum applied to inbound and outbound simultaneously. The bandwidth allowance largely depends on the instance type and size. 
If the \`bw_*_allowance_exceeded\` metric is constantly larger than zero, consider scaling up the instance. 
Or if the instance is behind a load balancer, consider adding more instances to distribute the load. 
Click [here](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ec2-instance-network-bandwidth.html) to learn more about EC2 network bandwidth.
`;

export const EC2_NETWORK_PPS_ALLOWANCE_RECOMMENDATIONS = `
### Working with EC2 network PPS allowances
The Packet-per-second (PPS) allowance is separate from the bandwidth allowances. 
If the \`pps_allowance_exceeded\` metric is the only one breaching, while the \`bw_*_allowance_exceeded\` metrics stay zero, 
this indicates that the network traffics are dominated by small packets, possibly caused by packet fragments. 
Refer to this [guide](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/ena-nitro-perf.html#ena-nitro-perf-maximize) for tuning PPS performance. 
Otherwise, also consider vertical or horizontal scaling.
`;

export const EC2_NETWORK_TRACKED_CONNECTIONS_ALLOWANCE_RECOMMENDATIONS = `
### Working with EC2 network tacked connections allowances
A security group only tracks a connection if the inbound rule and outbound rule are asymmetric, or if the connections are made through certain external components ([full list](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/security-group-connection-tracking.html#automatic-tracking)).
To optimize network performance under tracked connections, consider the following configurations, if they apply:
* Make inbound and outbound rules symmetric, so that the connections in scope will be untracked.
* Use network [ACLs](https://docs.aws.amazon.com/vpc/latest/userguide/vpc-network-acls.html), which are stateless, instead of security group rules to control access.
* If you have to use security groups, configure the shortest idle connection tracking timeout possible to allow unused connection tracking to be quickly available.
* For long-lived connections, configure TCP keep alives to be sent at intervals of less than 5 minutes to ensure connections stay open and maintain their tracked state, to avoid overhead of connection re-establishment.

For more details, check the [official guide](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/security-group-connection-tracking.html) for EC2 security group connection tracking.
`;

export const EC2_NETWORK_LINK_LOCAL_ALLOWANCE_RECOMMENDATIONS = `
### Working with EC2 network link-local allowances
The link-local allowance is fixed for all instance types. Therefore, for any non-zero values in the \`linklocal_allowance_exceeded\` metric, you need to identify the link-local service through tools, such as \`iftop\` or \`tcpdump\`, to root-cause and reduce the traffics.
The link-local address ranges are well-known, non-routable IP addresses, used by Amazon EC2 to provide services that are accessible only from an EC2 instance:
* IPV4: 169.254.0.0/16 (169.254.0.0 to 169.254.255.255)
* IPv6: fe80::/10

Common link-local services and their fixed IPs are:
* [Instance Metadata Service (IMDS)](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/instancedata-data-retrieval.html): \`169.254.169.254\`/\`fd00:ec2::254\`
* [Amazon Route 53 Resolver](https://docs.aws.amazon.com/vpc/latest/userguide/AmazonDNS-concepts.html): \`169.254.169.253\`/\`fd00:ec2::253\`/\`primary private IPV4 CIDR range provisioned to your VPC plus two\`
* [Amazon Time Sync Service](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/set-time.html): \`169.254.169.123\`/\`fd00:ec2::123\`
`;

export const MEMORY_COMPACTION_INVESTIGATION = `
### Investigate failing memory compaction
Compaction migrates in-use pages out of the way to build a physically contiguous block. Either the background \`kcompactd\` thread does it, which costs the workload nothing, or the thread that needs the memory does it itself and stays paused for the whole attempt. \`compact_stall\` in the Virtual Memory Stats data counts only the second kind. Transparent Huge Pages are the most common requester, but device and network drivers need contiguous memory too.

**Find out why it is failing**
Migrating pages needs somewhere to move them to, so compaction fails when there is no usable free memory:
* If \`MemAvailable\` in the Memory Usage data is low, there is nowhere to migrate pages to. Relieve the memory pressure; no compaction setting will help.
* Otherwise the free memory is pinned by kernel allocations, which cannot be migrated. Check the PageType and PageBlocks metrics in the Memory Allocation data for a large Unmovable share, which only freeing that kernel memory or a reboot resets.

**Find out which thread is paying for it**
For Transparent Huge Pages (compare \`thp_fault_alloc\` and \`thp_fault_fallback\` in the Virtual Memory Stats data), the \`transparent_hugepage/defrag\` setting in the Memory Settings data decides which of the two does the work:
* \`always\` compacts in the faulting thread, which is what produces these stalls.
* \`defer\` wakes \`kcompactd\` instead and falls back to regular pages for now, so the block is still built but off the application's critical path.
* \`never\` skips compaction altogether.

This setting only covers huge pages. A driver asking for a contiguous buffer compacts in its own thread regardless, so if the stalls are not coming from huge pages, changing it will not help.

**Related settings**
* \`vm.compaction_proactiveness\` in the Sysctl Config data - how aggressively \`kcompactd\` compacts in the background, before an application ever asks.
* \`vm.extfrag_threshold\` in the Sysctl Config data - how fragmented a zone must be before the kernel prefers compaction over reclaim.
* On kernels 6.9 and newer, the per-size \`transparent_hugepage/hugepages-*kB/enabled\` settings in the Memory Settings data ask for less contiguous memory each, so they succeed far more often than the largest size.
* A reserved HugeTLB pool, shown as \`hugepages/hugepages-*kB/nr_hugepages\` in the Memory Settings data, sets memory aside while it is still unfragmented so no search is needed later.
`;

export const PAGE_TABLE_OVERHEAD_INVESTIGATION = `
### Reduce page table memory
Every process needs its own page table entries for the memory it maps, so the cost is the number of processes multiplied by the size each one maps. With 4kB pages, a 64GB mapping costs about 128MB of entries per process, which is why hundreds of processes sharing one large region can spend as much memory on the entries as the region itself. Huge pages make each entry cover 2MB instead of 4kB.

**Back the mapped memory with huge pages**
\`ShmemHugePages\` and \`HugePages_Total\` in the Memory Usage data show whether either mechanism is in use. Both being zero means every entry is the system's default page size (e.g. 4kB).
* If the application can request huge pages itself, that is the most direct route. PostgreSQL takes \`huge_pages=on\` with a pool reserved through \`vm.nr_hugepages\`, reported as \`hugepages/hugepages-*kB/nr_hugepages\` in the Memory Settings data, large enough for \`shared_buffers\`.
* Otherwise allow transparent huge pages for shared memory. A shared anonymous mapping is shmem internally, so it follows \`transparent_hugepage/shmem_enabled\` in the Memory Settings data and not \`transparent_hugepage/enabled\`. If that setting reads \`never\`, \`within_size\` is the usual choice, since it only uses a huge page where one fits entirely inside the mapping: \`echo within_size > /sys/kernel/mm/transparent_hugepage/shmem_enabled\`.

**Or reduce the number of processes**
The entries are paid per process, so fewer processes mapping the region saves proportionally. For a database this usually means a connection pooler.
`;

export const DIRTY_WRITEBACK_INVESTIGATION = `
### Investigate dirty page writeback
The kernel holds data written by an application in the page cache as dirty pages until it flushes them to disk. How much the kernel buffers, and how eagerly it flushes, decides how much time the application spends waiting on the disk. 
If a performance regression is suspected to be related to dirty page writeback, check if any of the below configs are inconsistent across runs. 

**Related Settings**
The current value of each setting below is in the Sysctl Config data. To change one, run for example
\`\`\`shell
sysctl -w vm.dirty_bytes=1073741824
\`\`\`
and persist it under \`/etc/sysctl.d/\`.
* \`vm.dirty_bytes\` or \`vm.dirty_ratio\` - the limit at which a writing process is made to stop and flush data itself. Only one of them can be set to non-zero.
* \`vm.dirty_background_ratio\` or \`vm.dirty_background_bytes\` - the point where the kernel starts flushing in the background, without blocking the application.
* \`vm.dirty_writeback_centisecs\` - how often the kernel flusher threads wake up.
* \`vm.dirty_expire_centisecs\` - how old dirty data has to be before it is written out.

\`CONFIG_BLK_WBT\` and \`CONFIG_BLK_WBT_MQ\` are the two related kernel configs. They let the block layer hold back background writeback so that it does not delay the application's own I/O.
`;
