# APerf

> [!NOTE]
> Leave us a feedback at https://github.com/aws/aperf/discussions/329

## What is APerf?
A CLI tool to gather many pieces of performance data in one go. APerf includes a recorder, reporter and custom-pmu sub tools. The recorder gathers performance metrics, stores them in a set of local files that can then be analyzed via the reporter sub tool. The custom-pmu sub-tool can be used to create custom PMU configs which can configure the events an aperf recorder collects.

## Why does APerf exist?
Performance issues in applications are investigated by recreating them locally and collecting data/metrics using monitoring tools like sysstat, perf, sysctl, ebpf, etc... or by running these tools remotely. Installing and executing various performance monitoring tools is a manual process and prone to errors. Even with the [Graviton Performance Runbook](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/README.md), understanding the output of these tools requires deep domain specific knowledge.

The aim of APerf is to enable anyone to collect performance data in their environment while providing tools to analyze and visualize application performance. APerf will hopefully enable faster troubleshooting by analyzing and highlighting deviations in performance between two application environments automatically.

> [!TIP]
> Check out the APerf guide and demo video at https://www.youtube.com/watch?v=mSrDZuxWFtw

## What data does APerf collect?
APerf collects the following metadata:
- `systeminfo`: System information and EC2 metadata if running on EC2 instances
- `kernel_config`: Kernel Configuration (/boot/config)
- `sysctl`: Sysctl variable configuration settings

APerf collects the following performance data:
- `cpu_utilization`: CPU Utilization, both per CPU and aggregate CPU utilization
- `vmstat`: Virtual Memory Utilization
- `diskstats`: Disk Utilization per Disk
- `interrupts`: Interrupt Data per Interrupt Line per CPU
- `perf_stat`: CPU Performance Counters
- `processes`: CPU utilization of running processes
- `netstat`: Network stats
- `meminfo`: Memory usage information
- `perf_profile`: Performance profile data (enabled through the `--profile` option and the `perf` binary)
- `java_profile`: JVM profile data (enabled through the `--profile-java` option and the [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary)
- `hotline`: Memory and branch predictor hot spot detection (needs to be built with the Hotline feature and run on metal instance only)

Aperf collects the following data about its own process:
- `aperf_runlog`: the log messages
- `aperf_stats`: Execution time of each collection interval (including the total time and every data's collection time) 

## Requirements
* [Rust toolchain (v1.61.0+)](https://www.rust-lang.org/tools/install)
* [Node.js (v16.16.0+)](https://nodejs.org/en/download/)

## Installation
Download the binary from the [Releases](https://github.com/aws/APerf/releases) page. Only Linux platforms are supported.

### Building from source
1. Download the source code from the [Releases](https://github.com/aws/APerf/releases) page.

2. Install the Rust toolchain, node, and build tools
    - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - `curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash`
    - `source ~/.bashrc`  # or `source ~/.zshrc` if using zsh
    - `nvm install 16.16.0`
    - `nvm use 16.16.0`
    - `sudo apt install -y build-essential` for Ubuntu and `sudo yum install kernel-devel` for Amazon Linux

3. Run the following commands:
   ```
   cargo build
   cargo test
   ```

4. The memory and branch predictor hot spot analysis tool (hotline) is not enabled by default and requires having appropriate permissions set and necessary dependencies installed. The following demonstrates how to do it on Ubuntu and Amazon Linux.

   On Ubuntu 22.04:

   - `sudo apt-get update`
   - `sudo apt install -y build-essential`
   - `sudo apt install linux-modules-extra-$(uname -r)`
   - `sudo nano /etc/default/grub` and add/modify `GRUB_CMDLINE_LINUX_DEFAULT="kpti=off"`
   - `sudo update-grub`
   - `sudo reboot`
   - `sudo modprobe arm_spe_pmu`
   - `sudo sh -c 'echo 0 > /proc/sys/kernel/kptr_restrict'`
   - `sudo sh -c 'echo -1 > /proc/sys/kernel/perf_event_paranoid'`
   - `sudo chmod +r /proc/kallsyms`
   - `sudo apt-get install libdw-dev  libelf-dev  libcapstone-dev  zlib1g-dev  liblzma-dev  libbz2-dev  libzstd-dev`

   On Amazon Linux 2 / Amazon Linux 2023:

   - `sudo yum install kernel-devel`
   - `sudo nano /etc/default/grub` and add/modify `GRUB_CMDLINE_LINUX_DEFAULT="kpti=off"`
   - `sudo grub2-mkconfig -o /boot/grub2/grub.cfg`
   - `sudo reboot`
   - `sudo sysctl -w kernel.perf_event_paranoid=-1`
   - `sudo sysctl -w kernel.kptr_restrict=0`
   - `sudo sysctl --system`
   - `sudo chmod +r /proc/kallsyms`
   - `sudo yum groupinstall "Development Tools" -y && sudo yum install -y elfutils-devel elfutils-libelf-devel capstone-devel zlib-devel xz-devel bzip2-devel libzstd-devel`

   After completing the above steps, build the hotline-enabled binary:
   ```
   cargo build --release --features hotline
   cargo test --features hotline
   ```

## Usages
`aperf record` records performance data and stores them in a series of files. A report is then generated with `aperf report` and can be viewed in any system with a web browser. `aperf custom-pmu` can be used to generate a PMU config file which customizes which events are collected by aperf. The generated PMU config can be used with the `--pmu-config` flag with `aperf record`.

> [!WARNING]
> **KNOWN LIMITATION** :
> The default configuration of 10ms for `perf_event_mux_interval_ms` is known to cause serious performance overhead for systems with large core counts. We recommend setting this value to 100ms by doing the following: `echo 100 | sudo tee /sys/devices/*/perf_event_mux_interval_ms`

### Basic Usages

#### Record

Run the following command to start an Aperf record run. Aperf will run for `<PERIOD>` seconds. During the recording period, once every `<INTERVAL>` seconds, it collects performance data from the system and writes them to binary files. At the end of the record, all collected data will be available in the `<RUN_NAME>` directory and also archived in `<RUN_NAME>.tar.gz`.

```
aperf record -r <RUN_NAME> -i <INTERVAL> -p <PERIOD>
```

#### Report

Run the following command to generate an Aperf report for previously collected data. The data will be read from path `<RUN>`, which can be either the directory or archive produced by the previous `aperf record` command. The report will be generated in the `<REPORT_NAME>` directory and also archived in `<REPORT_NAME>.tar.gz`. To access the report, open the `index.html` file in browser.

```
aperf report -r <RUN> -n <REPORT_NAME>
```

You can compare the results of multiple performance record runs:
```
aperf report -r <RUN1> <RUN2> ... -n <REPORT_NAME>
```

#### Custom PMU

Run the following command to create a custom PMU configuration through command-line prompts. The generated configuration can then be used for `perf_stat` data collections through `aperf record --pmu-config`.

```
aperf custom-pmu
```

### Example
To see a step-by-step example, please see our example [here](./EXAMPLE.md).

### Available Options

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

Gather profiling data using the 'perf' binary.

`-F, --perf-frequency` [default: 99] 

Frequency for perf profiling in Hz.

`--profile-java [<PID/Name>,<PID/Name>,...,<PID/Name>]` [default: profiles all JVMs]

Profile JVMs using async-profiler.

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

-----

#### Custom PMU

`-p, --pmu-file <PMU_FILE>` 

Name of the file for an existing custom PMU configuration.

`--verify` 

Verify the supplied PMU file.

## APerf Issues?
#### PMU Counters:
* PMU counters are only available on [certain instance sizes](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md#how-to-collect-pmu-counters) and families. Select the appropriate instance size if you need PMU stats.
* For collecting PMU counter metrics w/o `root` or `sudo` permissions, set the `perf_event_paranoid` to `0`.
```
sudo sysctl -w kernel.perf_event_paranoid=0
```
* To collect PMU counter metrics, APerf needs to open up to 50 file descriptors per vCPU. So, increase `ulimit` settings accordingly.
* APerf preparation for PMU counter metrics may take significant time on larger instances, delaying the start of the recording period. Use `--dont-collect perf_stat` if startup time is a concern and/or PMU metrics are not necessary.
---
#### Other:
* APerf needs access to `/proc/kallsyms`, so we need to relax `kptr_restrict` by setting it to `0` (on Ubuntu OS).
```
sudo sysctl -w kernel.kptr_restrict=0
```
* To enable function-level profiling, install the `perf` binary on your instances.
* Download the right [APerf binary](https://github.com/aws/aperf/releases) based on the instance type (x86/Intel/AMD or aarch64/Graviton).
* For JVM profiling ensure the [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary is installed and the `jps` command is available (part of Java Development Kit).

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License. See [LICENSE](LICENSE) for more information.

