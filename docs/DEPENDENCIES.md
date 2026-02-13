# Aperf Dependencies

## Perf

**External Documentation:** [perf-record(1) man page](https://man7.org/linux/man-pages/man1/perf-record.1.html)

### Profiling using Perf

**Prerequisites:**
- Ensure perf binary is installed.
- [Kernel permissions](../README.md#advanced-usage) are set if not running with root permissions.

**What it collects:**  
System-wide CPU profiling data using `perf record` with call graphs. These are displayed as flamegraphs in the report.

**Usage examples:**
```bash
# Profile at default (99 Hz) sampling frequency
aperf record --profile

# Profile at 50 Hz
aperf record --profile --perf-frequency 50
```

### Implementation Details

APerf uses the perf binary when the `--profile` option is passed to the record subcommand. Under the hood, aperf spawns a perf process using this command:

```bash
perf record -a -q -g -k 1 -F <perf_frequency> -e cpu-clock:pppH -o <data_file_path> -- sleep <collection_time>
```

**Parameters (not directly set by user):**
- `perf_frequency`: Sampling frequency in Hz, set with `-F` or `--perf-frequency` option. Defaults to 99
- `data_file_path`: Path where perf data is written
- `collection_time`: Duration of the collection period in seconds

#### Behavior

APerf spawns a `perf record` process at the start of the recording period that runs for the collection duration. After the recording completes, APerf processes the collected data using `perf report --stdio --percent-limit 1` to generate a text report of the top functions (those consuming ≥1% of samples). This report is saved to the `top_functions` file in the data directory and displayed in the APerf HTML report. The flamegraphs are then generated from the `perf record` output by running `perf inject` to add the compiled code symbols and converting to SVG. All intermediate files are saved in the record archive for reference.

## Async-profiler

**External Documentation:** [async-profiler GitHub repository](https://github.com/async-profiler/async-profiler)

### Profiling Java using Async-Profiler

**Prerequisites:**
- Install [async-profiler](https://github.com/async-profiler/async-profiler) and add it to PATH.
- Ensure JDK is installed (APerf uses the `jps` command).

**What it collects:**  
CPU, allocation, and wall-clock samples from JVMs which are displayed as heatmaps in the report. If no JVMs are specified, APerf automatically detects and profiles all running JVMs.

**Usage examples:**

```bash
# Profile all running JVMs
aperf record --profile-java 
aperf record --profile-java jps

# Profile specific JVMs by name or PID
aperf record --profile-java program,program2,4936

# Profile a single JVM by PID with custom run name
aperf record --profile-java 3224 -r my_record
```

> [!TIP]
> See a step-by-step example in [EXAMPLE.md](./EXAMPLE.md).

### Implementation Details

APerf uses the async-profiler binary when the `--profile-java` option is passed into the record subcommand:

```bash
asprof -d <collection_time - elapsed_time> -o jfr -e cpu --alloc 2m --wall 100ms --cstack vm -F vtable -f <output_file_path> <jid>
```

**Parameters (not directly set by user):**
- `collection_time - elapsed_time`: Remaining time in the aperf record (asprof may be launched during the recording period)
- `output_file_path`: Temporary file path for asprof output
- `jid`: Java process ID to attach asprof to. This is set by APerf depending on the arguments passed to `--profile-java`

> [!TIP]
> async-profiler should be run by the same user that owns the target JVM process, or with elevated privileges (root/sudo).

#### Behavior

When run with arg `jps` or no args, `--profile-java` will check for new JVMs using `jps` at the beginning of record and at every sampling interval. APerf will launch an asprof process with the options specified above on any new JVMs detected. Otherwise, if a specific PID or JVM name is passed, then APerf will only attempt to launch asprof at the beginning of the record. After the record, APerf uses the `jfrconv` command to generate cpu, alloc, and wall profiling heatmaps for each JVM profiled.

For more detailed Java performance debugging, you can analyze the the generated JFR file further with [JDK Mission Control](https://www.oracle.com/java/technologies/jdk-mission-control.html).

