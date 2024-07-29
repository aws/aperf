use anyhow::Result;
use aperf_lib::record::{record, Record};
use aperf_lib::report::{report, Report};
use flate2::read::GzDecoder;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::{fs, panic};
use tar::Archive;
use tempfile::TempDir;

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
fn test_record() {
    run_test(|tempdir, aperf_tmp| {
        let run_name = tempdir
            .join("test_record")
            .into_os_string()
            .into_string()
            .unwrap();
        let rec = Record {
            run_name: Some(run_name.clone()),
            interval: 1,
            period: 2,
            profile: false,
            profile_java: None,
        };

        record(&rec, &aperf_tmp).unwrap();
        assert!(Path::new(&run_name).exists());
        assert!(Path::new(&(run_name.clone() + ".tar.gz")).exists());

        fs::remove_dir_all(&run_name).unwrap();
        fs::remove_file(run_name + ".tar.gz").unwrap();
        Ok(())
    })
}

#[test]
#[serial]
fn test_report() {
    run_test(|tempdir, aperf_tmp| {
        let run_name = tempdir
            .join("record_data")
            .into_os_string()
            .into_string()
            .unwrap();
        let rec = Record {
            run_name: Some(run_name.clone()),
            interval: 1,
            period: 2,
            profile: false,
            profile_java: None,
        };

        record(&rec, &aperf_tmp).unwrap();

        let test_report_path = PathBuf::from("test_report");
        let report_loc = tempdir
            .join("test_report")
            .into_os_string()
            .into_string()
            .unwrap();
        let report_path = tempdir.join("test_report");
        let rep = Report {
            run: [run_name.clone()].to_vec(),
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

        fs::remove_dir_all(&run_name).unwrap();
        fs::remove_dir_all(&report_loc).unwrap();
        fs::remove_file(run_name.clone() + ".tar.gz").unwrap();
        fs::remove_file(report_loc.clone() + ".tar.gz").unwrap();
        Ok(())
    })
}
