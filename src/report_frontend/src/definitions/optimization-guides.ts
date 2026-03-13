export const CPU_UTILIZATION_OPTIMIZATION = `
### CPU utilization investigations
#### Higher-than-expected CPU utilization
To find out which part in code is consuming more CPU time, you can perform [on-cpu profiling](https://aws.github.io/graviton/perfrunbook/debug_code_perf.html#on-cpu-profiling), which produces flamegraphs that indicate the CPU utilization of every stack trace. The profiling data is available in APerf if you used the \`--profile\` option during recording. To make sure the flamegraphs are correctly collected:
* Before profiling, make sure \`/proc/sys/kernel/kptr_restrict\` is 0 for kernel address visibility. If not, run \`sudo sysctl -w kernel.kptr_restrict=0\`.
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
* Run \`watch netstat -t\` to look for heavily used connections. A dominating connection can saturate one core and bottleneck the rest of the system.
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
Since TLB is a cache that holds virtual-to-physical address translation, reducing its miss rate can improve performance:
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
