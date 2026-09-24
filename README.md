# APerf

> [!NOTE]
> Leave us feedback at https://github.com/aws/aperf/discussions/329

## What is APerf?
APerf is a CLI tool used for performance monitoring and debugging. It records a wide range of performance-related system metrics or data over a sampling period, such as CPU utilization, memory availability, and PMU counters, and writes them into an archive on disk. APerf's recording is low overhead, and aims to utilize <5% of one CPU. To view the data, APerf processes one or more collected archives, performs analysis, and generates an HTML report. In the report, users can refer to the analytical findings for potential performance issues, or they can browse through all collected data to get a holistic understanding of the systems under test.

## Why does APerf exist?
Traditionally, performance issues in applications are investigated by recreating them locally and collecting data/metrics using monitoring tools like sysstat, perf, sysctl, ebpf, etc... or by running these tools remotely. Installing and executing various performance monitoring tools is a manual process and prone to errors. Even after collecting data, understanding the output of these tools requires deep domain specific knowledge.

The goal of APerf is to accelerate the performance debugging process by enabling experienced users to deep dive into extensive data and less experienced users to identify issues without specialized knowledge. This is achieved by,

- Consolidating the collection of a wide array of system metrics into a single command.
- Efficiently visualizing data in an interactive report.
- Performing automated analysis to highlight potential performance issues.

> [!TIP]
> Check out the APerf guide and demo video at https://www.youtube.com/watch?v=mSrDZuxWFtw

## Download

#### Latest release: https://github.com/aws/aperf/releases/latest
#### Nightly builds: https://github.com/aws/aperf/releases/tag/nightly
#### Docker Images: https://gallery.ecr.aws/aperf/aperf

The release artifacts include
* ARM and X86 binaries
* Windows and Mac binaries, for report generation only
* ARM and x86 RPM packages
* ARM and x86 DEB packages
* Docker images
* The kubectl tool

Alternatively, you can [build APerf from source](#building-from-source).

## Quick Start

```bash
echo "Download the latest APerf binary"
VERSION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
  https://github.com/aws/aperf/releases/latest | sed 's#.*/tag/##')
ARCH=$(uname -m)
curl -fsSL "https://github.com/aws/aperf/releases/download/$VERSION/aperf-$VERSION-$ARCH.tar.gz" | tar -xzf -
cd aperf-$VERSION-$ARCH
./aperf --version

echo "Override perf_event_mux_interval_ms to reduce CPU overhead for PMU data collection"
echo 100 | sudo tee /sys/bus/event_source/devices/*/perf_event_mux_interval_ms

echo "Run APerf data collection for 10 seconds"
sudo ./aperf record -r my_run

echo "Generate APerf report"
sudo ./aperf report -r my_run -n my_report
```

Then open my_report/index.html in browser to access the report.

## What data does APerf collect?

The table below follows the report's navigation panel, which shows each data type under its "Report Page" name.

| Data Type                 | Report Page          | Description                                                                                                                                                  |
|---------------------------|----------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `systeminfo`              | Report Home          | System information and EC2 metadata if running on EC2 instances                                                                                              |
| **Performance Data**      |                      |                                                                                                                                                              |
| `cpu_utilization`         | CPU Utilization      | CPU Utilization percentage                                                                                                                                   |
| `perf_stat`               | PMU Events           | [PMU data](/docs/PMU.md)                                                                                                                                     |
| `interrupts`              | Interrupts           | Per-CPU Interrupt count                                                                                                                                      |
| `diskstats`               | Disk Stats           | Disk Utilization per device                                                                                                                                  |
| `processes`               | Processes            | Running processes resource usages                                                                                                                            |
| `meminfo`                 | Memory Usage         | Physical memory usage                                                                                                                                        |
| `memalloc`                | Memory Allocation    | Memory allocation data including buddyinfo, pagetypeinfo, and slabinfo (some data requires root privileges)                                                  |
| `vmstat`                  | Virtual Memory Stats | Virtual Memory stats                                                                                                                                         |
| `numastat`                | NUMA Stats           | Per-NUMA-node memory stats (/sys/devices/system/node/node\*/numastat)                                                                                        |
| `netstat`                 | TCP/IP Stats         | TCP/IP stats                                                                                                                                                 |
| `ena_stat`                | ENA Stats            | ENA (ethtool) stats                                                                                                                                          |
| `efa_stat`                | EFA Stats            | [EFA](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/efa.html) stats                                                                                    |
| **System Configurations** |                      |                                                                                                                                                              |
| `kernel_config`           | Kernel Config        | Kernel Configuration (/boot/config)                                                                                                                          |
| `sysctl`                  | Sysctl Config        | Sysctl variable configuration settings                                                                                                                       |
| `mem_settings`            | Memory Settings      | Static memory-related configurations                                                                                                                         |
| **Profiling**             |                      |                                                                                                                                                              |
| `perf_profile`            | Perf Profiling       | Performance profile data (enabled through the `--profile` option and the `perf` binary)                                                                      |
| `java_profile`            | Java Profiling       | JVM profile data (enabled through the `--profile-java` option and the [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary) |
| `hotline`                 | Hotline              | Memory and branch predictor hot spot detection (needs to be [built with the Hotline feature](#building-with-hotline) and run on metal instance only)                                   |
| **APerf Execution**       |                      |                                                                                                                                                              |
| `aperf_stats`             | APerf Stats          | Wallclock time of each data's collection and child processes' resource usages                                                                                |
| `aperf_runlog`            | APerf Logs           | The log messages of APerf's execution                                                                                                                        |

## Detailed Usages

For every subcommand below, you can supply these options:

`-h, --help`

Print help menu.

`-V, --version`

Print version.

`-v, --verbose`

Show debug messages. Use `-vv` for more verbose messages.

`-t, --tmp-dir <TMP_DIR>` [default: /tmp]

Temporary directory for intermediate files.

-----

### Record

`aperf record` collects system configurations and performance data periodically and writes them to disk. It produces a directory and tar ball containing all collected data.

`-r, --run-name <RUN_NAME>` [default: aperf_\<timestamp\>]

Name of the run, which defines the directory and archive name of the recorded data.

`-i, --interval <INTERVAL>` [default: 1]

Interval (in seconds) at which performance data is to be collected.

`-p, --period <PERIOD>` [default: 10]

Time (in seconds) for which the performance data is to be collected.

> [!TIP]
> If applicable, collect for at least 10 minutes to more accurately record the application's behavior.

`--dont-collect <Data Name>,<Data Name>...`

The list of performance data to skip collection. Cannot be used with `--collect_only`.

`--collect-only <Data Name>,<Data Name>...`

The list of performance data to be collected - the others will not be collected. Cannot be used with `--dont_collect`. Please note that we recommend to always collect as much data as possible for performance debugging, unless you are sure some data can be excluded.

`--profile` 

Gather profiling data using the 'perf' binary. See [perf documentation](./docs/PROFILINGS.md#perf) for more details and usage.

`-F, --perf-frequency <FREQUENCY>` [default: 99] 

Frequency for perf profiling in Hz.

`--profile-java [<PID/Name>,<PID/Name>,...,<PID/Name>]` [default: profiles all JVMs]

Profile JVMs using async-profiler. See [async-profiler documentation](./docs/PROFILINGS.md#async-profiler) for more details and usage.

`--pmu-config <PMU_CONFIG>` 

Custom PMU config file to use.

`--ungroup-pmu-events`

Avoid creating a PMU counter group for each metric defined in the PMU config. For details, please read the [PMU data document](/docs/PMU.md).

`--pmu-cpus <CPU>,<CPU RANGE>,...,<CPU>` [default: all online CPUs]

Collect PMU counters only on the given CPUs, given as a comma separated list of single CPUs and inclusive ranges, e.g. `12,5,41-55,16-19,1`.

`--hotline-sample-frequency <FREQUENCY>` (For Hotline-enabled binary) [default: 1000]

Hotline sampling period in Hz.

`--num-to-report <NUM_TO_REPORT>` [default: 5000]

Maximum number of report entries to process for Hotline tables.

-----

### Report

`aperf report` processes one or more directories or tar balls produced by `aperf record`, runs analytics, and generates a static HTML report. The report contains all visualized data and findings about potential performance bottlenecks. Check [EXAMPLE.md](/docs/EXAMPLE.md) for more report usages. 

`-r, --run <RUN> <RUN> ...` 

The paths to the directories or archives of the recorded data to be included in the report.

> [!TIP]
> If multiple runs are included in the report, the first run will be used as the base run. The data in every other run will be compared against the base run to generate statistical and analytical findings.

`-n, --name <NAME>` [default: aperf_report_<run>] 

The directory and archive name of the report.

`--time-range RUN_NAME=FROM_TIME:TO_TIME`

The time range to apply to a run in the report, including its time-series metrics, statistics, and analytical findings.
Specify the option multiple times to apply a time range for multiple runs, or omit the `RUN_NAME=` part to apply it to all runs. Either bound can be omitted or negative.

-----

### Setup Shell Completions
`aperf setup-shell-completions` generates a completion script for the specified shell and prints it to stdout. Aperf can also install the script to a specific location or attempt to detect the location for the shell using through `--install` option.

> [!TIP]
> Installing the RPM or DEB package installs the completion script automatically.

`--shell <SHELL>`

Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh].

`--install <Path including filename>`

Install the auto complete script using sudo, or specify a download path.

-----

### MCP Server (AI Assistant Integration)

APerf includes a built-in [MCP](https://modelcontextprotocol.io/) server that lets AI assistants (Kiro, Claude Desktop, etc.) record data, generate reports, and analyze performance metrics interactively.

```bash
# Start the MCP server (used by AI clients, not run manually)
aperf server --mcp
```

**Kiro setup** — add to `~/.kiro/settings/mcp.json`:

```json
{
  "mcpServers": {
    "aperf-mcp": {
      "command": "/path/to/aperf",
      "args": ["server", "--mcp"],
      "disabled": false
    }
  }
}
```

The server exposes 8 tools: `load_report`, `get_metrics`, `get_metric_values`, `get_analytical_findings`, `get_statistical_findings`, `get_flamegraph`, `record`, and `generate_report`. See [MCP Server docs](./docs/MCP-SERVER.md) for full details.

## Building from source
1. Download the source code from the [Releases](https://github.com/aws/APerf/releases) page.

2. Install requirements: [Rust toolchain (v1.61.0+)](https://www.rust-lang.org/tools/install), [Node.js (v16.16.0+)](https://nodejs.org/en/download/), and build tools

    ```bash
    # Install Rust
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    
    # Install Node.js via nvm
    curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
    source ~/.bashrc  # or source ~/.zshrc if using zsh
    nvm install 16.16.0
    nvm use 16.16.0
    
    # Install build tools
    # On Ubuntu:
    sudo apt install -y build-essential # or on Amazon Linux: sudo yum install kernel-devel
    ```

3. Build APerf:
   ```
   cargo build --release
   ```
   The binary will be located at `target/release/aperf`.

### Building with Hotline

Hotline is APerf's in-memory latency and branch hotspot analyzer. It uses [ARM Statistical Profiling Extension (SPE)](https://developer.arm.com/community/arm-community-blogs/b/architectures-and-processors-blog/posts/statistical-profile-extension) to sample micro-architectural events directly from the CPU pipeline, giving you precise, low-overhead visibility into where your workloads spend time and why.

Hotline produces two categories of analysis:

- **Memory latency hotspots** — identifies instructions with the highest memory access latency, broken down by execution, issue, and translation latency. A completion node view shows the cache hierarchy distribution (L1/L2/L3/DRAM) for each hot instruction, so you can see whether a load is bottlenecked on a specific cache level.
- **Branch misprediction hotspots** — identifies instructions with the most branch samples and their misprediction rates.

**Hotline can only be built and run on a baremetal Graviton instance.** For now, it is not released as a binary. To use the feature, please build from source following the below instructions:

   **On Ubuntu 22.04:**

   ```bash
   sudo apt-get update
   sudo apt install -y build-essential
   sudo apt install linux-modules-extra-$(uname -r)
   # KPTI isolates kernel and user space page tables, but interferes with performance monitoring. Disable:
   sudo nano /etc/default/grub  # Add/modify: GRUB_CMDLINE_LINUX_DEFAULT="kpti=off"
   sudo update-grub
   sudo reboot
   sudo modprobe arm_spe_pmu
   sudo sh -c 'echo 0 > /proc/sys/kernel/kptr_restrict'
   sudo sh -c 'echo -1 > /proc/sys/kernel/perf_event_paranoid'
   sudo chmod +r /proc/kallsyms
   sudo apt-get install libdw-dev libelf-dev libcapstone-dev zlib1g-dev liblzma-dev libbz2-dev libzstd-dev
   ```

   **On Amazon Linux 2 / Amazon Linux 2023:**

   ```bash
   sudo yum install kernel-devel
   # KPTI isolates kernel and user space page tables, but interferes with performance monitoring. Disable:
   sudo nano /etc/default/grub  # Add/modify: GRUB_CMDLINE_LINUX_DEFAULT="kpti=off"
   sudo grub2-mkconfig -o /boot/grub2/grub.cfg
   sudo reboot
   sudo sysctl -w kernel.perf_event_paranoid=-1
   sudo sysctl -w kernel.kptr_restrict=0
   sudo sysctl --system
   sudo chmod +r /proc/kallsyms
   sudo yum groupinstall "Development Tools" -y && sudo yum install -y elfutils-devel elfutils-libelf-devel capstone-devel zlib-devel xz-devel bzip2-devel libzstd-devel
   ```

   **After completing the above steps, build the hotline-enabled binary:**
   ```bash
   cargo build --release --features hotline
   cargo test --features hotline
   ```

## Known APerf Issues

* Since APerf report is a static HTML, a huge report with hour-long records on machines with numerous cores could produce processed data files larger than 1GB. These files could prevent Chrome from loading the report. Firefox works better with huge reports, but the loading latency will be high. Consider using the `--time-range` option to trim the report.
* Root permissions are required for APerf to collect pagetypeinfo and slabinfo in Memory Allocation data.

### PMU data collection
* To collect PMU data without `sudo` permissions, run `sudo sysctl -w kernel.perf_event_paranoid=-1` first.
* When using APerf's default PMU configuration, the default system value of 10ms for `perf_event_mux_interval_ms` could cause large CPU overheads. We recommend setting it to 100ms to reduce the overheads ([details](/docs/PMU.md)).
* The preparation time for PMU data can be significant on machines running kernel v5.x with a large number of cores. Use `--dont-collect perf_stat` if startup time is a concern and/or PMU metrics are not necessary.
* To learn more about the usage, limiations, and concerns of PMU data collection, refer to the [PMU data document](./docs/PMU.md).

### Profiling options
* Perf profiling (`--profile`) requires the Linux Perf tool to be installed first.
* To collect Perf profile without `sudo` permissions, run `sudo sysctl -w kernel.perf_event_paranoid=-1` and `sudo sysctl -w kernel.kptr_restrict=0` first.
* Java profiling (`--profile-java`) requires the [async-profiler](https://github.com/async-profiler/async-profiler/releases/latest) to be installed first.
* To learn more about the usage, limiations, and concerns of the profiling options, refer to the [profiling document](./docs/PROFILINGS.md).

## Documentation

- [Contributing](./CONTRIBUTING.md)
- [Development Guide](./docs/DEVELOPMENT.md)
- [Example Usage](./docs/EXAMPLE.md#aperf-example)
- [PMU Data Collection](./docs/PMU.md)
- [Profiling Options](./docs/PROFILINGS.md)
- [MCP Server (AI Assistant Integration)](./docs/MCP-SERVER.md)
- [Running on EKS](./docs/README-EKS.md#running-aperf-on-ekskubernetes)

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License. See [LICENSE](LICENSE) for more information.

