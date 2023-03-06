# APerf
## What is APerf?
A CLI tool to gather many pieces of performance data in one go. APerf includes a collector and a visualizer sub tool. The collector gathers performance metrics, stores them in a set of local files that can then be analyzed via the visualizer sub tool.

## Why does APerf exist?
Performance issues in applications are investigated by recreating them locally and collecting data/metrics using monitoring tools like sysstat, perf, sysctl, ebpf, etc... or by running these tools remotely. Installing and executing various performance monitoring tools is a manual process and prone to errors. Even with the [Graviton Performance Runbook](https://github.com/aws/aws-graviton-getting-started/blob/main/perfrunbook/graviton_perfrunbook.md), understanding the output of these tools requires deep domain specific knowledge.

The aim of APerf is to enable anyone to collect performance data in their enviornment while providing tools to analyze and visualize application performance. APerf will hopefully enable faster troubleshooting by analyzing and highlighting deviations in performance between two application environments automatically. 

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

## Requirements
* [Rust toolchain (v1.61.0+)](https://www.rust-lang.org/tools/install)
* [Node.js (v16.16.0+)](https://nodejs.org/en/download/)

## Installation
Download the binaries from the [Releases](https://github.com/aws/APerf/releases) page.

`aperf-collector`  only supports running on Linux.

`aperf-visualizer` only supports running on Linux.


### Building from source
1. Download the source code from the [Releases](https://github.com/aws/APerf/releases) page.
2. Run the following commands:

```
cargo build
cargo test
```

## Usage
`aperf-collector` collects performance data and stores them in a series of files. These files are then viewed using `aperf-visualizer` either on the same machine the performance data was collected on or a remote machine running `aperf-visualizer`. 

To visualize the data using `aperf-visualizer` download the directory created by `aperf-collector` and load the data with `aperf-visualizer`.

**KNOWN LIMITATION**

The default configuration of 10ms for `/sys/devices/cpu/perf_event_mux_interval_ms` is known to cause serious performance overhead for systems with large core counts. We recommend setting this value to 100ms by doing the following:

```
echo 100 | sudo tee /sys/devices/cpu/perf_event_mux_interval_ms 
```

**aperf-collector**
1. Download the `aperf-collector` binary.
2. Start `aperf-collector`:
```
./aperf-collector -r <RUN_NAME> -i <INTERVAL_NUMBER> -p <COLLECTION_PERIOD>
```

**aperf-visualizer**
1. Download the `aperf-visualizer` binary.
2. Download the directory created by `aperf-collector` to the machine where `aperf-visualizer` will be running.
3. Start `aperf-visualizer`:
```
./aperf-visualizer -r <COLLECTOR_DIRECTORY> -p <PORT_NUMBER>
```
### Example
To see a step-by-step example, please see our example [here](./EXAMPLE.md)

### Configuration

`aperf-collector` has the following flags available for use:

**Collector Flags:**

`-v, --version` version of APerf

`-i, --interval` interval collection rate (default 1)

`-p, --period` period (how long you want the data collection to run, default is 10s)

`-r, --run-name` run name (name of the run for organization purposes, creates directory of the same name, default of aperf_[timestamp])


`./aperf-visualizer -h`

**Visualizer Flags:**

`-v, --version` version of APerf visualizer

`-r, --run-directory` directory that contains collected data 

`-p, --port` port number (default localhost:8080)

## Logging
* `env_logger` is used to log information about the tool run to stdout.
* To see it, use `export APERF_LOG_LEVEL=info`.
* To see more detail, use `export APERF_LOG_LEVEL=debug`.

## Security

See [CONTRIBUTING](CONTRIBUTING.md#security-issue-notifications) for more information.

## License

This project is licensed under the Apache-2.0 License. See [LICENSE](LICENSE) for more information.

