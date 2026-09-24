# Aperf Dependencies

## Perf

**External Documentation:** [perf-record(1) man page](https://man7.org/linux/man-pages/man1/perf-record.1.html)

### Profiling using Perf

**Prerequisites:**
Ensure perf binary is installed:

```bash
# For Amazon Linux or other Red Hat-based
sudo dnf install perf
```
```bash
# For Ubuntu / Debian
sudo apt install linux-tools-common linux-tools-generic linux-tools-$(uname -r)
```
Ensure Perf is available on PATH:
```bash
perf -v
```

For non-root APerf runs, configure the permissions to allow profiling collection and kernel symbol resolution:

```bash
sudo sysctl -w kernel.perf_event_paranoid=-1
sudo sysctl -w kernel.kptr_restrict=0
```

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

APerf spawns a `perf record` process at the start of the recording period that runs for the collection duration. After the recording completes, the flamegraphs are generated from the `perf record` output by running `perf inject` to add the compiled code symbols and converting to SVG.

## Async-profiler

**External Documentation:** [async-profiler GitHub repository](https://github.com/async-profiler/async-profiler)

### Profiling Java using Async-Profiler

**Prerequisites:**
Install [async-profiler](https://github.com/async-profiler/async-profiler):
```bash
VERSION=$(curl -fsSLI -o /dev/null -w '%{url_effective}' \
  https://github.com/async-profiler/async-profiler/releases/latest | sed 's#.*/tag/v##')
ARCH=$(uname -m | sed -e 's/x86_64/x64/' -e 's/aarch64/arm64/')
curl -fsSL "https://github.com/async-profiler/async-profiler/releases/download/v$VERSION/async-profiler-$VERSION-linux-$ARCH.tar.gz" \
  | sudo tar -xzf - -C /opt
sudo ln -sf /opt/async-profiler-$VERSION-linux-$ARCH/bin/{asprof,jfrconv} /usr/local/bin/
```
The `jps`, `jcmd`, and `jfr` commands provided by JDK are also required. To install JDK (24 is recommended but other 16+ versions should also work):
```bash
# For Amazon Linux or other Red Hat-based
sudo dnf install java-24-amazon-corretto-devel
```
```bash
# For Ubuntu / Debian, add the Corretto apt repository first
curl -fsSL https://apt.corretto.aws/corretto.key \
  | sudo gpg --dearmor -o /usr/share/keyrings/corretto-keyring.gpg
echo "deb [signed-by=/usr/share/keyrings/corretto-keyring.gpg] https://apt.corretto.aws stable main" \
  | sudo tee /etc/apt/sources.list.d/corretto.list
sudo apt update && sudo apt install java-24-amazon-corretto-jdk
sudo ln -sf /usr/lib/jvm/java-24-amazon-corretto/bin/jfr /usr/local/bin/jfr
```
Ensure that the below commands all work:
```bash
asprof --version
jfrconv --version
jps -h
jcmd -h
jfr --version
```

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

