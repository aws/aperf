# Example
This guide will show how to run APerf to diagnose your application's performance.

## Download APerf
1. Download the binaries from the [Releases](https://github.com/aws/APerf/releases) page.
2. Copy the binaries to the host that is running your application.
3. Untar the directory, and place the binary you want it to reside.
```
tar -xvf ./aperf-v0.1.4-alpha-aarch64.tar.gz
```

## Running APerf
For the purpose of this example we will be collecting data on two systems. The first system will be an x86-based C6i system and the second one will be an AWS Graviton C7g instance. To collect performance data in 1 second time intervals for 10 seconds on the C6i instances, run the following command:

```
./aperf-v0.1.4-alpha-aarch64/aperf-collector -i 1 -p 10 -r c6i_performance_run_1
```

To collect performance data in 1 second time intervals for 10 seconds on the C7gf instances, run the following command (not the `run_name` parameter has changed. This allows us to easily differentiate between two performance runs.

```
./aperf-v0.1.4-alpha-aarch64/aperf-collector -i 1 -p 10 -r c7g_performance_run_1
```

## Visualizing The Results
To visualize the results you'll need access to a Linux desktop environment with a web browser installed. If you don't have access to a Linux desktop environment, [AWS Workspaces](https://aws.amazon.com/workspaces/) can be used to spin up a Linux desktop enviornment quickly and easily.

To get started you'll need the `aperf-visualizer` binary and the performance data on the same machine. To visualize the results of a single performance run use the following command:

```
./aperf-v0.1.4-alpha-aarch64/aperf-visualizer --run-directory c7g_performance_run_1
```

## Comparing Two Performance Run Results
To visualize and compare the results of two different performance runs, use the following command. APerf will automatically highlight variances between the two performance runs. This can be useful for comparing differences between systems.

```
./aperf-v0.1.4-alpha-aarch64/aperf-visualizer --run-directory c7g_performance_run_1 --run-directory c6i_performance_run_1
```
