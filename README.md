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

## Quick Start
Get started with APerf in under 2 minutes:
```bash
# 1. Download and extract latest release
arch=$(uname -m); curl -sL $(curl -s https://api.github.com/repos/aws/aperf/releases/latest | grep "browser_download_url.*$arch.*\.tar\.gz" | cut -d'"' -f4) | tar -xz && echo "✓ aperf available at ./aperf-<version>-$arch/aperf"

# 2. Set kernel permissions (non-root users)
sudo sysctl -w kernel.perf_event_paranoid=-1
sudo sysctl -w kernel.kptr_restrict=0
ulimit -n 65535

# 3. Record performance data every 1 second for 60 seconds
./aperf record -r my_run -i 1 -p 60

# 4. Generate report
./aperf report -r my_run -n my_report

# 5. Open my_report/index.html in your browser
```

## What data does APerf collect?

| Data Type | Description |
|-----------|-------------|
| **Metadata** | |
| `systeminfo` | System information and EC2 metadata if running on EC2 instances |
| `kernel_config` | Kernel Configuration (/boot/config) |
| `sysctl` | Sysctl variable configuration settings |
| **Performance Data** | |
| `cpu_utilization` | CPU Utilization, both per CPU and aggregate CPU utilization |
| `vmstat` | Virtual Memory Utilization |
| `diskstats` | Disk Utilization per Disk |
| `interrupts` | Interrupt Data per Interrupt Line per CPU |
| `perf_stat` | CPU Performance Counters |
| `processes` | CPU utilization of running processes |
| `netstat` | Network stats |
| `meminfo` | Memory usage information |
| `perf_profile` | Performance profile data (enabled through the `--profile` option and the `perf` binary) |
| `java_profile` | JVM profile data (enabled through the `--profile-java` option and the [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary) |
| `hotline` | Memory and branch predictor hot spot detection (needs to be built with the Hotline feature and run on metal instance only) |
| **APerf Execution Data** | |
| `aperf_runlog` | The log messages |
| `aperf_stats` | Execution time of each collection interval (including the total time and every data's collection time) | 

## Installation
Download the binary from the [Releases](https://github.com/aws/APerf/releases) page. APerf record and report are fully supported on Linux. Only report generation (`aperf report`) is supported on macOS and Windows.

### Building from source
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

### Optional - Building with Hotline

4. The memory and branch predictor hot spot analysis tool (hotline) is not enabled by default and requires having appropriate permissions set and necessary dependencies installed. The following demonstrates how to do it on Ubuntu and Amazon Linux.

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

## Usage
`aperf record` records performance data and stores them in a series of files. A report is then generated with `aperf report` and can be viewed in any system with a web browser.

### Basic Usage

#### Record

Run the following command to start an Aperf record run. Aperf will run for `<PERIOD>` seconds. During the recording period, once every `<INTERVAL>` seconds, it collects performance data from the system and writes them to binary files. At the end of the record, all collected data will be available in the `<RUN_NAME>` directory and also archived in `<RUN_NAME>.tar.gz`.

```
sudo aperf record -r <RUN_NAME> -i <INTERVAL> -p <PERIOD>
```
To run without sudo, refer to [kernel permissions](#kernel-permissions).

#### Report

Run the following command to generate an Aperf report for previously collected data. The data will be read from path `<RUN>`, which can be either the directory or archive produced by the previous `aperf record` command. The report will be generated in the `<REPORT_NAME>` directory and also archived in `<REPORT_NAME>.tar.gz`. To access the report, open the `index.html` file in browser.

```
aperf report -r <RUN> -n <REPORT_NAME>
```

You can compare the results of multiple performance record runs:
```
aperf report -r <RUN1> <RUN2> ... -n <REPORT_NAME>
```

> [!TIP]
> If multiple runs are included in the report, the first run will be used as the base run. The data in every other run will be compared against the base run to generate all statistical findings and some analytical findings.

### Advanced Usage

<a name="kernel-permissions"></a>**Kernel Permissions for non-root users**

Aperf PMU collection, profiling options `--profile`, and hotline require relaxed kernel permissions:

```bash
sudo sysctl -w kernel.perf_event_paranoid=-1   # Provides profiling access for perf record, PMU counters, hotline
sudo sysctl -w kernel.kptr_restrict=0          # Provides perf access to /proc/kallsyms
ulimit -n 65535                                # Increases file descriptor limit for PMU collection
```

These settings can be skipped when running APerf with root privileges.

## Available Options

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

#### Record

`-r, --run-name <RUN_NAME>` [default: aperf_\<timestamp\>]

Name of the run, which defines the directory and archive name of the recorded data.

`-i, --interval <INTERVAL>` [default: 1]

Interval (in seconds) at which performance data is to be collected.

`-p, --period <PERIOD>` [default: 10]

Time (in seconds) for which the performance data is to be collected.

`--dont-collect <Data Name>,<Data Name>...`

The list of performance data to skip collection. Cannot be used with `--collect_only`.

`--collect-only <Data Name>,<Data Name>...`

The list of performance data to be collected - the others will not be collected. Cannot be used with `--dont_collect`. Please note that we recommend to always collect as much data as possible for performance debugging, unless you are sure some data can be excluded.

`--profile` 

Gather profiling data using the 'perf' binary. See [perf documentation](./docs/DEPENDENCIES.md#perf) for more details and usage.

`-F, --perf-frequency <FREQUENCY>` [default: 99] 

Frequency for perf profiling in Hz.

`--profile-java [<PID/Name>,<PID/Name>,...,<PID/Name>]` [default: profiles all JVMs]

Profile JVMs using async-profiler. See [async-profiler documentation](./docs/DEPENDENCIES.md#async-profiler) for more details and usage.

`--pmu-config <PMU_CONFIG>` 

Custom PMU config file to use.

`--hotline-sample-frequency <FREQUENCY>` (For Hotline-enabled binary) [default: 1000]

Hotline sampling period in Hz.

`--num-to-report <NUM_TO_REPORT>` [default: 5000]

Maximum number of report entries to process for Hotline tables.

-----

#### Report

`-r, --run <RUN> <RUN> ...` 

The paths to the directories or archives of the recorded data to be included in the report.

`-n, --name <NAME>` [default: aperf_report_<run>] 

The directory and archive name of the report.

`-t, --tmp-dir <TMP_DIR> ` [default: /tmp]

Temporary directory for intermediate files 

-----

#### Custom PMU
Use this command to create a custom PMU configuration interactively through command-line prompts. This allows users to specify the set of counters for their specific machine and use case. The generated configuration can then be used for `perf_stat` data collections through `aperf record --pmu-config <PMU_CONFIG>`.

`-p, --pmu-file <PMU_FILE>` 

Name of the file for an existing custom PMU configuration.

`--verify` 

Verify the supplied PMU file.

-----

#### Setup Shell Completions
This command generates a completion script for the specified shell, which will be printed to stdout. Aperf can also install the script to a specific location or automatically attempt to detect the proper location for the shell using the `--install` option.

`--shell <SHELL>`

Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]

`--install <Path including filename>`

Install the auto complete script using sudo, or specify a download path

## APerf Issues?

> [!WARNING]
> **KNOWN LIMITATIONS** :
> - The default configuration of 10ms for `perf_event_mux_interval_ms` is known to cause serious performance overhead for systems with large core counts. We recommend setting this value to 100ms by doing the following: `echo 100 | sudo tee /sys/devices/*/perf_event_mux_interval_ms`
> - APerf report is currently not able to efficiently process and display long record data. Multiple hour long records on machines with many CPUs (>128 core count) may be too large to run report on or open in a web browser.

#### PMU Counters:
* PMU counters are only available on [certain instance sizes](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#how-to-collect-pmu-counters) and families. Select the appropriate instance size if you need PMU stats.
* For collecting PMU counter metrics w/o `root` or `sudo` permissions, set the `perf_event_paranoid` to `-1`.
```
sudo sysctl -w kernel.perf_event_paranoid=-1
```
* To collect PMU counter metrics, APerf needs to open up to 50 file descriptors per vCPU. So, increase `ulimit` settings accordingly.
* APerf preparation for PMU counter metrics may take significant time on larger instances, delaying the start of the recording period. Use `--dont-collect perf_stat` if startup time is a concern and/or PMU metrics are not necessary.
#### Other:
* APerf needs access to `/proc/kallsyms`, so we need to relax `kptr_restrict` by setting it to `0` (on Ubuntu OS).
```
sudo sysctl -w kernel.kptr_restrict=0
```
* To enable function-level profiling, install the `perf` binary on your instances.
* Download the right [APerf binary](https://github.com/aws/aperf/releases) based on the instance type (x86/Intel/AMD or aarch64/Graviton).
* For JVM profiling ensure the [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary is installed and the `jps` command is available (part of Java Development Kit).

## Documentation

- [Contributing](./CONTRIBUTING.md)
- [Dependencies Reference](./docs/DEPENDENCIES.md#aperf-dependencies)
- [Development Guide](./docs/DEVELOPMENT.md)
- [Example Usage](./docs/EXAMPLE.md#aperf-example)
- [Running on EKS](./docs/README-EKS.md#running-aperf-on-ekskubernetes)

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License. See [LICENSE](LICENSE) for more information.

