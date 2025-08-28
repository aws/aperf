use anyhow::Result;
use aperf::data::DEFAULT_DATA_NAMES;
use aperf::record::{record, Record};
use aperf::report::{report, Report};
use aperf::APERF_RUNLOG;
use chrono::Utc;
use flate2::read::GzDecoder;
#[cfg(feature = "hotline")]
use libc::c_int;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::{fs, panic};
use tar::Archive;
use tempfile::TempDir;

#[cfg(feature = "hotline")]
extern "C" {
    fn test_all() -> c_int;
}

fn run_test<T>(test_func: T)
where
    T: FnOnce(PathBuf, PathBuf) -> Result<()> + panic::UnwindSafe,
{
    let tempdir = TempDir::with_prefix("aperf").unwrap();
    let aperf_tmp = TempDir::with_prefix("tmp_aperf").unwrap();

    let result = panic::catch_unwind(|| {
        test_func(tempdir.path().to_path_buf(), aperf_tmp.path().to_path_buf())
    });
    tempdir.close().unwrap();
    aperf_tmp.close().unwrap();
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

#[test]
#[serial]
fn test_record() {
    run_test(|tempdir, aperf_tmp| {
        let run_name =
            record_with_name("test_record".to_string(), &tempdir, &aperf_tmp, None, None).unwrap();

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

#[test]
#[serial]
fn test_record_dont_collect_some_data() {
    run_test(|tempdir, aperf_tmp| {
        let dont_collect_data_names = vec![
            String::from("netstat"),
            String::from("processes"),
            String::from("cpu_utilization"),
            String::from("meminfo"),
        ];

        let run_name = record_with_name(
            "test_record_skip_some_data".to_string(),
            &tempdir,
            &aperf_tmp,
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

#[test]
#[serial]
fn test_record_collect_only_some_data() {
    run_test(|tempdir, aperf_tmp| {
        let collect_only_data_names = vec![
            String::from("netstat"),
            String::from("vmstat"),
            String::from("systeminfo"),
            String::from("sysctl"),
            String::from("kernel_config"),
        ];

        let run_name = record_with_name(
            "test_record_skip_some_data".to_string(),
            &tempdir,
            &aperf_tmp,
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

#[test]
#[serial]
fn test_report() {
    run_test(|tempdir, aperf_tmp| {
        let run_dir =
            record_with_name("record_data".to_string(), &tempdir, &aperf_tmp, None, None)?;
        report_with_name(run_dir, tempdir, aperf_tmp)
    })
}

#[test]
#[serial]
fn test_report_dot_in_run_name() {
    run_test(|tempdir, aperf_tmp| {
        let run_dir =
            record_with_name("record.data".to_string(), &tempdir, &aperf_tmp, None, None)?;
        report_with_name(run_dir, tempdir, aperf_tmp)
    })
}

#[test]
#[serial]
fn test_report_with_empty_data_bin() {
    run_test(|tempdir, aperf_tmp| {
        let run_dir_name = "empty_data_bin";
        let run_dir = tempdir.join(run_dir_name);
        fs::create_dir(&run_dir).unwrap();
        fs::File::create(tempdir.join(format!("{run_dir_name}.tar.gz"))).unwrap();

        let time_str = Utc::now().format("%Y-%m-%d_%H_%M_%S").to_string();
        for data_name in DEFAULT_DATA_NAMES.iter() {
            let binary_file_path = run_dir.join(format!("{data_name}_{time_str}.bin"));
            fs::File::create(&binary_file_path).unwrap();
        }
        fs::File::create(run_dir.join("aperf_runlog")).unwrap();
        fs::File::create(run_dir.join("aperf_stats.bin")).unwrap();
        fs::File::create(run_dir.join("meta_data.bin")).unwrap();

        report_with_name(
            run_dir.into_os_string().into_string().unwrap(),
            tempdir,
            aperf_tmp,
        )
    })
}

fn record_with_name(
    run: String,
    tempdir: &Path,
    aperf_tmp: &Path,
    dont_collect: Option<Vec<String>>,
    collect_only: Option<Vec<String>>,
) -> Result<String> {
    let run_name = tempdir.join(run).into_os_string().into_string().unwrap();

    #[cfg(feature = "hotline")]
    let rec = Record {
        run_name: Some(run_name.clone()),
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
        run_name: Some(run_name.clone()),
        interval: 1,
        period: 2,
        dont_collect,
        collect_only,
        profile: false,
        perf_frequency: 99,
        profile_java: None,
        pmu_config: None,
    };

    let runlog = tempdir.join(*APERF_RUNLOG);
    fs::File::create(&runlog).unwrap();

    record(&rec, aperf_tmp, &runlog).unwrap();

    Ok(run_name)
}

fn report_with_name(run_dir: String, tempdir: PathBuf, aperf_tmp: PathBuf) -> Result<()> {
    let test_report_path = PathBuf::from("test_report");
    let report_loc = tempdir
        .join("test_report")
        .into_os_string()
        .into_string()
        .unwrap();
    let report_path = tempdir.join("test_report");
    let rep = Report {
        run: [run_dir.clone()].to_vec(),
        name: Some(report_loc.clone()),
    };
    report(&rep, &aperf_tmp).unwrap();

    // Check if the directory has the proper structure
    assert!(report_path.exists());
    assert!(report_path.join("index.css").exists());
    assert!(report_path.join("index.js").exists());
    assert!(report_path.join("data/archive").exists());
    assert!(Path::new(&(report_loc.clone() + ".tar.gz")).exists());

    let tar_gz = fs::File::open(report_loc.clone() + ".tar.gz").unwrap();
    let mut archive = Archive::new(GzDecoder::new(tar_gz));

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
    assert!(paths.contains(&test_report_path.join("index.html")));
    assert!(paths.contains(&test_report_path.join("index.css")));
    assert!(paths.contains(&test_report_path.join("index.js")));
    assert!(paths.contains(&test_report_path.join("data/archive")));

    fs::remove_dir_all(&run_dir).unwrap();
    fs::remove_dir_all(&report_loc).unwrap();
    fs::remove_file(run_dir.clone() + ".tar.gz").unwrap();
    fs::remove_file(report_loc.clone() + ".tar.gz").unwrap();
    Ok(())
}
