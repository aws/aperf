use anyhow::Result;
use aperf::report::{report, Report};
use chrono::Utc;
use flate2::read::GzDecoder;
#[cfg(feature = "hotline")]
use libc::c_int;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::{fs, panic};
use tar::Archive;
use tempfile::TempDir;
#[cfg(target_os = "linux")]
use {
    aperf::data::DEFAULT_DATA_NAMES,
    aperf::record::{record, Record},
    aperf::APERF_RUNLOG,
};

#[cfg(feature = "hotline")]
extern "C" {
    fn test_all() -> c_int;
}

fn run_test<T>(test_func: T)
where
    T: FnOnce(PathBuf, PathBuf) -> Result<()> + panic::UnwindSafe,
{
    let work_dir = TempDir::with_prefix("aperf").unwrap();
    let tmp_dir = TempDir::with_prefix("tmp_aperf").unwrap();

    let result = panic::catch_unwind(|| {
        test_func(work_dir.path().to_path_buf(), tmp_dir.path().to_path_buf())
    });
    work_dir.close().unwrap();
    tmp_dir.close().unwrap();
    if let Err(e) = result {
        panic::resume_unwind(e);
    }
}

#[test]
#[serial]
#[cfg(feature = "hotline")]
fn test_hotline() {
    unsafe {
        test_all();
    }
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_record() {
    run_test(|work_dir, tmp_dir| {
        let run_name =
            record_with_name("test_record".to_string(), &work_dir, &tmp_dir, None, None).unwrap();

        assert!(Path::new(&run_name).exists());
        assert!(Path::new(&(run_name.clone() + ".tar.gz")).exists());

        let all_data_files = fs::read_dir(&run_name).unwrap();
        assert_eq!(
            all_data_files.count(),
            // Aside from the default data, Aperf still produces 4 more files:
            // aperf runlog, aperf stats, metadata, and pmu_config (since we are collecting perf_stat)
            DEFAULT_DATA_NAMES.len() + 4,
            "The data files should only include those not skipped, plus aperf runlog, aperf stats, metadata, and pmu_config."
        );

        fs::remove_dir_all(&run_name).unwrap();
        fs::remove_file(run_name + ".tar.gz").unwrap();
        Ok(())
    })
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_record_dont_collect_some_data() {
    run_test(|work_dir, tmp_dir| {
        let dont_collect_data_names = vec![
            String::from("netstat"),
            String::from("processes"),
            String::from("cpu_utilization"),
            String::from("meminfo"),
        ];

        let run_name = record_with_name(
            "test_record_skip_some_data".to_string(),
            &work_dir,
            &tmp_dir,
            Some(dont_collect_data_names.clone()),
            None,
        )
        .unwrap();

        assert!(Path::new(&run_name).exists());
        assert!(Path::new(&(run_name.clone() + ".tar.gz")).exists());

        let all_data_files = fs::read_dir(&run_name).unwrap();
        let mut num_data_files: usize = 0;
        for data_file in all_data_files {
            num_data_files += 1;
            let file_name = data_file.unwrap().file_name().into_string().unwrap();
            for dont_collect_data_name in &dont_collect_data_names {
                assert!(
                    !file_name.starts_with(dont_collect_data_name),
                    "{file_name} should not exist since it is included in dont_collect"
                );
            }
        }
        assert_eq!(
            num_data_files,
            // Aside from the data not included in dont_collect, Aperf still produces 4 more files:
            // aperf runlog, aperf stats, metadata, and pmu_config (since we are collecting perf_stat)
            DEFAULT_DATA_NAMES.len() - dont_collect_data_names.len() + 4,
            "The data files should only include those not skipped, plus aperf runlog, aperf stats, metadata, and pmu_config."
        );

        fs::remove_dir_all(&run_name).unwrap();
        fs::remove_file(run_name + ".tar.gz").unwrap();
        Ok(())
    })
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_record_collect_only_some_data() {
    run_test(|work_dir, tmp_dir| {
        let collect_only_data_names = vec![
            String::from("netstat"),
            String::from("vmstat"),
            String::from("systeminfo"),
            String::from("sysctl"),
            String::from("kernel_config"),
        ];

        let run_name = record_with_name(
            "test_record_skip_some_data".to_string(),
            &work_dir,
            &tmp_dir,
            None,
            Some(collect_only_data_names.clone()),
        )
        .unwrap();

        assert!(Path::new(&run_name).exists());
        assert!(Path::new(&(run_name.clone() + ".tar.gz")).exists());

        let all_data_files = fs::read_dir(&run_name).unwrap();
        let mut num_data_files: usize = 0;
        for data_file in all_data_files {
            num_data_files += 1;
            let file_name = data_file.unwrap().file_name().into_string().unwrap();
            for &data_name in DEFAULT_DATA_NAMES.iter() {
                if collect_only_data_names
                    .iter()
                    .any(|collect_only_data_name| collect_only_data_name == data_name)
                {
                    continue;
                }
                assert!(
                    !file_name.starts_with(data_name),
                    "{file_name} should not exist since it is not included in collect_only"
                );
            }
        }
        assert_eq!(
            num_data_files,
            // Aperf should only collect the data specified in the collect_only flag and produce
            // the corresponding binary files, plus 3 more:
            // aperf runlog, aperf stats, and metadata
            collect_only_data_names.len() + 3,
            "The data files should only include those not skipped, plus aperf runlog, aperf stats, and metadata."
        );

        fs::remove_dir_all(&run_name).unwrap();
        fs::remove_file(run_name + ".tar.gz").unwrap();
        Ok(())
    })
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_record_and_report() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("test_record_run");
        let run_dir = record_with_name(run_name.clone(), &work_dir, &tmp_dir, None, None)?;

        let report_name = String::from("test_report");
        let rep = Report {
            run: vec![run_dir],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name.clone()]);

        clean_dir_and_archive(&work_dir, &report_name);
        clean_dir_and_archive(&work_dir, &run_name);

        Ok(())
    })
}

#[cfg(target_os = "linux")]
#[test]
#[serial]
fn test_record_and_report_dot_in_run_name() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("test.record.data");
        let run_dir = record_with_name(run_name.clone(), &work_dir, &tmp_dir, None, None)?;

        let report_name = String::from("test_report");
        let rep = Report {
            run: vec![run_dir],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name.clone()]);

        clean_dir_and_archive(&work_dir, &report_name);
        clean_dir_and_archive(&work_dir, &run_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_report_with_empty_data_bin() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("empty_data_bin");
        let run_dir = work_dir.join(run_name.clone());
        fs::create_dir(&run_dir).unwrap();
        fs::File::create(work_dir.join(format!("{}.tar.gz", run_name))).unwrap();

        let time_str = Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string();
        let data_names = vec![
            "cpu_utilization",
            "diskstats",
            "flamegraphs",
            "interrupts",
            "sysctl",
            "processes",
        ];
        for data_name in data_names {
            let binary_file_path = run_dir.join(format!("{data_name}_{time_str}.bin"));
            fs::File::create(&binary_file_path).unwrap();
        }
        fs::File::create(run_dir.join("aperf_runlog")).unwrap();
        fs::File::create(run_dir.join("aperf_stats.bin")).unwrap();
        fs::File::create(run_dir.join("meta_data.bin")).unwrap();

        let report_name = String::from("empty_data_report");
        let rep = Report {
            run: vec![run_dir.into_os_string().into_string().unwrap()],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name.clone()]);

        clean_dir_and_archive(&work_dir, &report_name);
        clean_dir_and_archive(&work_dir, &run_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_report_single_run() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("test_run_1");
        let run_path = get_test_data_path(format!("{}.tar.gz", run_name));

        let report_name = String::from("single_run_report");
        let rep = Report {
            run: vec![run_path.into_os_string().into_string().unwrap()],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name]);

        clean_dir_and_archive(&work_dir, &report_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_report_multiple_runs() {
    run_test(|work_dir, tmp_dir| {
        let run_name_1 = String::from("test_run_1");
        let run_path_1 = get_test_data_path(format!("{}.tar.gz", run_name_1));
        let run_name_2 = String::from("test_run_2");
        let run_path_2 = get_test_data_path(format!("{}.tar.gz", run_name_2));

        let report_name = String::from("multi_run_report");
        let rep = Report {
            run: vec![
                run_path_1.into_os_string().into_string().unwrap(),
                run_path_2.into_os_string().into_string().unwrap(),
            ],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name_1, run_name_2]);

        clean_dir_and_archive(&work_dir, &report_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_report_from_report() {
    run_test(|work_dir, tmp_dir| {
        let input_report_path = get_test_data_path("test_report.tar.gz");
        let run_name = String::from("test_run_3");
        let run_path = get_test_data_path(format!("{}.tar.gz", run_name));

        let report_name = String::from("report_from_report");
        let rep = Report {
            run: vec![
                input_report_path.into_os_string().into_string().unwrap(),
                run_path.into_os_string().into_string().unwrap(),
            ],
            name: Some(
                work_dir
                    .join(&report_name)
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(
            &work_dir,
            &report_name,
            vec![
                String::from("test_run_1"),
                String::from("test_run_2"),
                run_name,
            ],
        );

        clean_dir_and_archive(&work_dir, &report_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_report_already_exists() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("test_run_3");
        let run_path = get_test_data_path(format!("{}.tar.gz", run_name));

        let report_name = String::from("a_report");
        let report_path_str = work_dir
            .join(&report_name)
            .into_os_string()
            .into_string()
            .unwrap();
        let rep = Report {
            run: vec![run_path.into_os_string().into_string().unwrap()],
            name: Some(report_path_str.clone()),
        };
        assert!(report(&rep, &tmp_dir).is_ok());

        verify_report_structure(&work_dir, &report_name, vec![run_name]);

        let another_run_path = get_test_data_path("test_run_1.tar.gz");
        let rep_with_same_name = Report {
            run: vec![another_run_path.into_os_string().into_string().unwrap()],
            name: Some(report_path_str.clone()),
        };
        let error = report(&rep_with_same_name, &tmp_dir).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!(
                "The report {} already exists in current directory.",
                report_path_str
            )
        );

        clean_dir_and_archive(&work_dir, &report_name);

        Ok(())
    })
}

#[test]
#[serial]
fn test_run_data_not_exists() {
    run_test(|work_dir, tmp_dir| {
        let run_name = String::from("fake_run");
        let run_path = get_test_data_path(format!("{}.tar.gz", run_name));

        let report_name = String::from("the_report_never_generated");
        let report_dir_path = work_dir.join(&report_name);
        let report_archive_path = work_dir.join(format!("{}.tar.gz", report_name));
        let rep = Report {
            run: vec![run_path.clone().into_os_string().into_string().unwrap()],
            name: Some(
                report_dir_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };
        let error = report(&rep, &tmp_dir).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("The run {:?} does not exist.", run_path)
        );

        assert!(!report_dir_path.exists());
        assert!(!report_archive_path.exists());

        Ok(())
    })
}

#[test]
#[serial]
fn test_duplicate_run_data() {
    run_test(|work_dir, tmp_dir| {
        let input_report_path = get_test_data_path("test_report.tar.gz");
        let duplicate_run_name = String::from("test_run_1");
        let duplicate_run_path = get_test_data_path(format!("{}.tar.gz", duplicate_run_name));

        let report_name = String::from("report_with_duplicate_data");
        let report_dir_path = work_dir.join(&report_name);
        let report_archive_path = work_dir.join(format!("{}.tar.gz", report_name));
        let rep = Report {
            run: vec![
                input_report_path.into_os_string().into_string().unwrap(),
                duplicate_run_path.into_os_string().into_string().unwrap(),
            ],
            name: Some(
                report_dir_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };

        let error = report(&rep, &tmp_dir).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("Multiple runs with the same name: {}", duplicate_run_name)
        );

        assert!(!report_dir_path.exists());
        assert!(!report_archive_path.exists());

        Ok(())
    })
}

#[test]
#[serial]
fn test_duplicate_run_data_quick_fail() {
    run_test(|work_dir, tmp_dir| {
        let duplicate_run_name = String::from("test_run_2");
        let duplicate_run_path = get_test_data_path(format!("{}.tar.gz", duplicate_run_name));

        let report_name = String::from("report_not_to_be_generated");
        let report_dir_path = work_dir.join(&report_name);
        let report_archive_path = work_dir.join(format!("{}.tar.gz", report_name));
        let rep = Report {
            run: vec![
                duplicate_run_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
                duplicate_run_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ],
            name: Some(
                report_dir_path
                    .clone()
                    .into_os_string()
                    .into_string()
                    .unwrap(),
            ),
        };

        let error = report(&rep, &tmp_dir).unwrap_err();
        assert_eq!(
            error.to_string(),
            format!("Multiple runs with the same name: {}", duplicate_run_name)
        );

        assert!(!report_dir_path.exists());
        assert!(!report_archive_path.exists());

        Ok(())
    })
}

#[cfg(target_os = "linux")]
fn record_with_name(
    run_name: String,
    work_dir: &Path,
    tmp_dir: &Path,
    dont_collect: Option<Vec<String>>,
    collect_only: Option<Vec<String>>,
) -> Result<String> {
    let run_path_str = work_dir
        .join(run_name)
        .into_os_string()
        .into_string()
        .unwrap();

    #[cfg(feature = "hotline")]
    let rec = Record {
        run_name: Some(run_path_str.clone()),
        interval: 1,
        period: 2,
        dont_collect,
        collect_only,
        profile: false,
        perf_frequency: 99,
        profile_java: None,
        pmu_config: None,
        hotline_frequency: 1000,
        num_to_report: 5000,
    };

    #[cfg(not(feature = "hotline"))]
    let rec = Record {
        run_name: Some(run_path_str.clone()),
        interval: 1,
        period: 2,
        dont_collect,
        collect_only,
        profile: false,
        perf_frequency: 99,
        profile_java: None,
        pmu_config: None,
    };

    let runlog = work_dir.join(*APERF_RUNLOG);
    fs::File::create(&runlog)?;

    record(&rec, tmp_dir, &runlog)?;

    Ok(run_path_str)
}

/// Verify that the report structure is as expected
fn verify_report_structure(
    report_root: &PathBuf,
    report_name: &String,
    expected_run_names: Vec<String>,
) {
    let report_path = report_root.join(report_name);
    let report_archive_path = report_root.join(format!("{}.tar.gz", report_name));

    // Check if the directory has the proper structure
    assert!(report_path.exists());
    assert!(report_path.join("main.css").exists());
    assert!(report_path.join("bundle.js").exists());
    assert!(report_path.join("data").join("js").join("runs.js").exists());
    let report_run_archives_path = report_path.join("data").join("archive");
    assert!(report_run_archives_path.exists());
    for run_name in &expected_run_names {
        assert!(report_run_archives_path
            .join(format!("{}.tar.gz", run_name))
            .exists());
    }
    assert!(report_archive_path.exists());

    let report_archive_file = fs::File::open(report_archive_path).unwrap();
    let mut archive = Archive::new(GzDecoder::new(report_archive_file));

    let paths: Vec<PathBuf> = archive
        .entries()
        .unwrap()
        .map(|entry| -> Result<PathBuf> {
            let binding = entry.unwrap();
            let path = binding.path().unwrap().into_owned();
            Ok(path.to_path_buf())
        })
        .filter_map(|e| e.ok())
        .collect();

    // Check if the tarball of the directory has the proper structure
    let report_name_path_buf = PathBuf::from(report_name);
    assert!(paths.contains(&report_name_path_buf.join("index.html")));
    assert!(paths.contains(&report_name_path_buf.join("main.css")));
    assert!(paths.contains(&report_name_path_buf.join("bundle.js")));
    assert!(paths.contains(&report_name_path_buf.join("data").join("js").join("runs.js")));
    let report_archive_run_archives_path = report_name_path_buf.join("data").join("archive");
    for run_name in &expected_run_names {
        assert!(
            paths.contains(&report_archive_run_archives_path.join(format!("{}.tar.gz", run_name)))
        );
    }
}

/// Remove the directory and archive with the same name in the root path
fn clean_dir_and_archive(root_path: &PathBuf, file_name: &String) {
    let dir_path = root_path.join(file_name);
    let archive_path = root_path.join(format!("{}.tar.gz", file_name));

    fs::remove_dir_all(dir_path).unwrap();
    fs::remove_file(archive_path).unwrap();
}

fn get_test_data_path<P: AsRef<Path>>(test_data_file_name: P) -> PathBuf {
    PathBuf::from("tests")
        .join("test_data")
        .join(test_data_file_name)
}
