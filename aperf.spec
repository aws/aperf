Name:           aperf
Version:        %{_version}
Release:        1%{?dist}
Summary:        A CLI tool for performance monitoring and debugging

License:        Apache-2.0
URL:            https://github.com/aws/aperf
Source0:        aperf

%description
APerf is a CLI tool used for performance monitoring and debugging.
It records a wide range of performance-related system metrics or data over a sampling period, such as CPU utilization, memory availability, and PMU counters, and writes them into an archive on disk.
APerf's recording is low overhead, and aims to utilize <5% of one CPU. To view the data, APerf processes one or more collected archives, performs analysis, and generates an HTML report.
In the report, users can refer to the analytical findings for potential performance issues, or they can browse through all collected data to get a holistic understanding of the systems under test.

%prep

%build

%install
mkdir -p %{buildroot}%{_bindir}
install -m 0755 %{SOURCE0} %{buildroot}%{_bindir}/aperf

%post
# Install shell completions
/usr/bin/aperf setup-shell-completions --shell bash --install 2>/dev/null || true
/usr/bin/aperf setup-shell-completions --shell zsh --install 2>/dev/null || true

echo ""
echo "------------------------------------------------------------------------------"
echo "APerf installed successfully!"
echo "------------------------------------------------------------------------------"

perf_event_paranoid=$(cat /proc/sys/kernel/perf_event_paranoid 2>/dev/null || echo "2")

if [ "$perf_event_paranoid" != "-1" ]; then
    echo ""
    echo "For non-root users to collect PMU metrics, set kernel.perf_event_paranoid to -1 (current value: $perf_event_paranoid) by running:"
    echo "  sudo sysctl -w kernel.perf_event_paranoid=-1"
fi

kptr_restrict=$(cat /proc/sys/kernel/kptr_restrict 2>/dev/null || echo "1")

if [ "$kptr_restrict" != "0" ]; then
    echo ""
    echo "For non-root users to view kernel memory address in perf profile (--profile), set kernel.kptr_restrict to 0 (current value: $kptr_restrict) by running:"
    echo "  sudo sysctl -w kernel.kptr_restrict=0"
fi

missing_deps=""
command -v perf >/dev/null 2>&1 || missing_deps="${missing_deps}\n  - perf (for --profile option)"
command -v asprof >/dev/null 2>&1 || missing_deps="${missing_deps}\n  - async-profiler (for --profile-java option)"
command -v jps >/dev/null 2>&1 || missing_deps="${missing_deps}\n  - JDK with jps command (for --profile-java option)"

if [ -n "$missing_deps" ]; then
    echo ""
    echo "Optional dependencies not found:$missing_deps"
fi

echo ""
echo "Quick start:"
echo "  aperf record -r my_run -i 1 -p 60"
echo "  aperf report -r my_run -n my_report"
echo ""
echo "Documentation: https://github.com/aws/aperf"
echo "------------------------------------------------------------------------------"
echo ""

%files
%{_bindir}/aperf
