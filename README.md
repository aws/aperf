# APerf
## What is APerf?
A CLI tool to gather many pieces of performance data in one go. APerf includes a recorder, reporter and custom-pmu sub tools. The recorder gathers performance metrics, stores them in a set of local files that can then be analyzed via the reporter sub tool. The custom-pmu sub-tool can be used to create custom PMU configs which can configure the events an aperf recorder collects.

## Why does APerf exist?
Performance issues in applications are investigated by recreating them locally and collecting data/metrics using monitoring tools like sysstat, perf, sysctl, ebpf, etc... or by running these tools remotely. Installing and executing various performance monitoring tools is a manual process and prone to errors. Even with the [Graviton Performance Runbook](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/README.md), understanding the output of these tools requires deep domain specific knowledge.

The aim of APerf is to enable anyone to collect performance data in their environment while providing tools to analyze and visualize application performance. APerf will hopefully enable faster troubleshooting by analyzing and highlighting deviations in performance between two application environments automatically. 

## What data does APerf collect?
APerf collects the following metadata:
- System Info
- When run on EC2 instances this includes basic EC2 metadata
- Kernel Configuration (/boot/config)
- Sysctl variable configuration settings

APerf collects the following performance data:
- CPU Utilization, both per CPU and aggregate CPU utilization
- Virtual Memory Utilization
- Disk Utilization per Disk
- Interrupt Data per Interrupt Line per CPU
- CPU Performance Counters
- Network stats
- Meminfo
- Profile data (if enabled with `--profile` and `perf` binary present)
- JVM profile data with [async-profiler](https://github.com/async-profiler/async-profiler/tree/master) binary

## Requirements
* [Rust toolchain (v1.61.0+)](https://www.rust-lang.org/tools/install)
* [Node.js (v16.16.0+)](https://nodejs.org/en/download/)

## Installation
Download the binary from the [Releases](https://github.com/aws/APerf/releases) page.

`aperf` only supports running on Linux.

### Building from source
1. Download the source code from the [Releases](https://github.com/aws/APerf/releases) page.
2. Run the following commands:

```
cargo build
cargo test
```

## Usage
`aperf record` records performance data and stores them in a series of files. A report is then generated with `aperf report` and can be viewed in any system with a web browser. `aperf custom-pmu` can be used to generate a PMU config file which customizes which events are collected by aperf. The generated PMU config can be used with the `--pmu-config` flag with `aperf record`.

**KNOWN LIMITATION**

The default configuration of 10ms for `perf_event_mux_interval_ms` is known to cause serious performance overhead for systems with large core counts. We recommend setting this value to 100ms by doing the following:

```
echo 100 | sudo tee /sys/devices/*/perf_event_mux_interval_ms
```

**aperf record**
1. Download the `aperf` binary.
2. Start `aperf record`:
```
./aperf record -r <RUN_NAME> -i <INTERVAL_NUMBER> -p <COLLECTION_PERIOD>
```

**aperf report**
1. Download the `aperf` binary.
2. Download the directory created by `aperf record`.
3. Start `aperf report`:
```
./aperf report -r <COLLECTOR_DIRECTORY> -n <REPORT_NAME>
```

To compare the results of two different performance runs, use the following command:
```
./aperf report -r <COLLECTOR_DIRECTORY_1> -r <COLLECTOR_DIRECTORY_2> -n <REPORT_NAME>
```

**aperf custom-pmu**
1. Download the `aperf` binary.
2. Start `aperf custom-pmu`:
```
./aperf custom-pmu
```
### Example
To see a step-by-step example, please see our example [here](./EXAMPLE.md)

### Configuration

`aperf record` has the following flags available for use:

**Recorder Flags:**

`-V, --version` version of APerf

`-i, --interval` interval collection rate (default 1)

`-p, --period` period (how long you want the data collection to run, default is 10s)

`-r, --run-name` run name (name of the run for organization purposes, creates directory of the same name, default of aperf_[timestamp])

`--pmu-config` Custom PMU config file to use

`-v, --verbose` verbose messages

`-vv, --verbose --verbose` more verbose messages

`--profile` gather profiling data using the 'perf' binary

`--profile-java` profile JVMs by PID or name using async-profiler (default profiles all JVMs)

`./aperf record -h`

**Reporter Flags:**

`-V, --version` version of APerf visualizer

`-r, --run` run data to be visualized. Can be a directory or a tarball.

`-n, --name` report name (name of the report for origanization purposes, creates directory of the same name, default of aperf_report_<run>

`-v, --verbose` verbose messages

`-vv, --verbose --verbose` more verbose messages

`./aperf report -h`

**Custom-PMU Flags:**

`-V, --version` version of Aperf

`-p, --pmu-file` Name of the file for the custom PMU configuration

`--verify` Verify the supplied PMU file

`./aperf custom-pmu -h`

## APerf Issues?
Below are some prerequisites for profiling with APerf:
1. Select the [appropriate instance size](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/debug_hw_perf.md) if you need PMU stats.
2. For collecting PMU counter metrics w/o `root` or `sudo` permissions, set the `perf_event_paranoid` to `0`.
3. To collect PMU counter metrics, APerf needs to open up to 50 file descriptors per vCPU. So, increase `ulimit` settings accordingly.
4. APerf needs access to `/proc/kallsyms`, so we need to relax `kptr_restrict` by setting it to `0` (on Ubuntu OS).
5. To enable function-level profiling, install the `perf` binary on your instances.
6. Download to the instance the right [APerf binary](https://github.com/aws/aperf/releases), based on the instance type (x86/Intel/AMD or aarch64/Graviton).

## Logging
* `env_logger` is used to log information about the tool run to stdout.
* To see it, use `./aperf <command> -v`.
* To see more detail, use `./aperf <command> -vv`.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License. See [LICENSE](LICENSE) for more information.

