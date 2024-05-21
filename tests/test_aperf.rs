use anyhow::Result;
use aperf_lib::record::{record, Record};
use aperf_lib::report::{report, Report};
use flate2::read::GzDecoder;
use std::fs;
use std::path::{Path, PathBuf};
use tar::Archive;

#[test]
fn test_record() {
    let rec = Record {
        run_name: Some("test_record".to_string()),
        interval: 1,
        period: 2,
        profile: false,
    };

    record(&rec).unwrap();
    assert!(Path::new("test_record").exists());
    assert!(Path::new("test_record.tar.gz").exists());

    fs::remove_dir_all("test_record").unwrap();
    fs::remove_file("test_record.tar.gz").unwrap();
}

#[test]
fn test_report() {
    let rec = Record {
        run_name: Some("record_data".to_string()),
        interval: 1,
        period: 2,
        profile: false,
    };

    record(&rec).unwrap();
    let rep = Report {
        run: ["record_data".to_string()].to_vec(),
        name: Some("test_report".to_string()),
    };
    report(&rep).unwrap();

    // Check if the directory has the proper structure
    assert!(Path::new("test_report").exists());
    assert!(Path::new("test_report/index.html").exists());
    assert!(Path::new("test_report/index.css").exists());
    assert!(Path::new("test_report/index.js").exists());
    assert!(Path::new("test_report/data/archive").exists());
    assert!(Path::new("test_report.tar.gz").exists());

    let tar_gz = fs::File::open("test_report.tar.gz").unwrap();
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
    assert!(paths.contains(&PathBuf::from("test_report/index.html")));
    assert!(paths.contains(&PathBuf::from("test_report/index.css")));
    assert!(paths.contains(&PathBuf::from("test_report/index.js")));
    assert!(paths.contains(&PathBuf::from("test_report/data/archive")));

    fs::remove_dir_all("record_data").unwrap();
    fs::remove_dir_all("test_report").unwrap();
    fs::remove_file("record_data.tar.gz").unwrap();
    fs::remove_file("test_report.tar.gz").unwrap();
}
