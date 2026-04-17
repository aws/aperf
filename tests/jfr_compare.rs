use aperf::profiling::jfr::format_jfr;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

const JFR_EVENTS: &str = "jdk.ExecutionSample,jdk.NativeMethodSample,profiler.WallClockSample,jdk.MethodTrace,jdk.ObjectAllocationInNewTLAB,jdk.ObjectAllocationOutsideTLAB,jdk.ObjectAllocationSample,profiler.LiveObject,jdk.JavaMonitorEnter,jdk.ThreadPark,profiler.Malloc,profiler.Free,jdk.CPUTimeSample,profiler.NativeLock";

fn parse_events(output: &str) -> HashMap<String, usize> {
    let text = if let Some(pos) = output.find("\n---\n") {
        &output[pos + 5..]
    } else {
        output
    };

    let mut events: HashMap<String, usize> = HashMap::new();
    let mut current = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() && !current.is_empty() {
            let event = current.join("\n").trim().to_string();
            if !event.is_empty() {
                *events.entry(event).or_default() += 1;
            }
            current.clear();
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        let event = current.join("\n").trim().to_string();
        if !event.is_empty() {
            *events.entry(event).or_default() += 1;
        }
    }
    events
}

fn run_jfr_print(jfr_path: &Path) -> String {
    let output = Command::new("jfr")
        .args(["print", "--stack-depth", "999", "--events", JFR_EVENTS])
        .arg(jfr_path)
        .output()
        .expect("failed to run jfr");
    assert!(
        output.status.success(),
        "jfr print failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("jfr output not utf8")
}

#[cfg(target_os = "linux")]
#[test]
fn compare_jfr_outputs() {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/jfr_fixtures");
    let mut tested = 0;

    for entry in std::fs::read_dir(&fixtures).expect("cannot read jfr_fixtures dir") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("jfr") {
            continue;
        }

        let name = path.file_name().unwrap().to_string_lossy();
        println!("Testing: {}", name);

        let gt_raw = run_jfr_print(&path);
        let ours_raw = format_jfr(&path).expect("format_jfr failed");

        let ground_truth = parse_events(&gt_raw);
        let ours = parse_events(&ours_raw);

        let gt_total: usize = ground_truth.values().sum();
        let our_total: usize = ours.values().sum();

        let mut only_gt = Vec::new();
        let mut only_ours = Vec::new();

        for (event, &count) in &ground_truth {
            let ours_count = ours.get(event).copied().unwrap_or(0);
            if count > ours_count {
                only_gt.push((count - ours_count, &event[..event.len().min(120)]));
            }
        }
        for (event, &count) in &ours {
            let gt_count = ground_truth.get(event).copied().unwrap_or(0);
            if count > gt_count {
                only_ours.push((count - gt_count, &event[..event.len().min(120)]));
            }
        }

        if !only_gt.is_empty() || !only_ours.is_empty() {
            let dump_dir = Path::new("/tmp/aperf_java_testing");
            fs::create_dir_all(dump_dir).ok();
            let stem = path.file_stem().unwrap().to_string_lossy();
            fs::write(dump_dir.join(format!("{}_ground_truth.out", stem)), &gt_raw).ok();
            fs::write(dump_dir.join(format!("{}_ours.out", stem)), &ours_raw).ok();

            let mut msg =
                format!("MISMATCH in {name} (ground_truth: {gt_total}, ours: {our_total})\n");
            msg.push_str(&format!("  Outputs saved to {}\n", dump_dir.display()));
            for (count, preview) in &only_gt {
                msg.push_str(&format!(
                    "  only in ground_truth [{count}x]: {}...\n",
                    preview.replace('\n', " | ")
                ));
            }
            for (count, preview) in &only_ours {
                msg.push_str(&format!(
                    "  only in ours [{count}x]: {}...\n",
                    preview.replace('\n', " | ")
                ));
            }
            panic!("{}", msg);
        }

        println!("  PASS ({} events)", gt_total);
        tested += 1;
    }

    assert!(tested > 0, "No .jfr files found in {}", fixtures.display());
}
