use anyhow::Result;
use crate::{data::Data, get_file, PDError};
use serde::Deserialize;
use std::{collections::HashMap, fs::File};

pub struct DataVisualizer {
    pub data: Data,
    pub file_handle: Option<File>,
    pub run_values: HashMap<String, Vec<Data>>,
    pub file_name: String,
    pub js_file_name: String,
    pub js: String,
    pub api_name: String,
}

impl DataVisualizer {
    pub fn new(data: Data, file_name: String, js_file_name: String, js: String, api_name: String) -> Self {
        DataVisualizer {
            data: data,
            file_handle: None,
            run_values: HashMap::new(),
            file_name: file_name,
            js_file_name: js_file_name,
            js: js,
            api_name: api_name,
        }
    }

    pub fn init_visualizer(&mut self, dir: String, name: String) -> Result<(), tide::Error> {
        let file = get_file(dir.clone(), self.file_name.clone())?;
        self.file_handle = Some(file);
        self.run_values.insert(name, Vec::new());
        Ok(())
    }

    pub fn unpack_data(&mut self, name: String) -> Result<()> {
        let mut data = Vec::new();
        for document in serde_yaml::Deserializer::from_reader(self.file_handle.as_ref().unwrap()) {
            let v = Data::deserialize(document);
            data.push(v?);
        }
        self.run_values.insert(name.clone(), data);
        Ok(())
    }

    pub fn get_data(&mut self, query: String) -> Result<String> {
        /* Get run name from Query */
        let param: Vec<(String, String)> = serde_urlencoded::from_str(&query)?;
        let (_, run) = param[0].clone();

        let values = self.run_values.get_mut(&run).ok_or(PDError::VisualizerRunValueGetError(run.to_string()))?;
        if values.len() == 0 {
            return Ok("No data collected".to_string());
        }
        self.data.get_data(values.clone(), query)
    }
}

pub trait GetData {
    fn get_data(&mut self, _values: Vec<Data>, _query: String) -> Result<String> {
        unimplemented!();
    }
}

#[cfg(test)]
mod tests {
    use crate::data::cpu_utilization::{CpuData, CpuUtilization};
    use crate::data::{Data, TimeEnum};
    use super::DataVisualizer;

    #[test]
    fn test_unpack_data() {
        let mut dv = DataVisualizer::new(
            Data::CpuUtilization(CpuUtilization::new()),
            "cpu_utilization".to_string(),
            String::new(),
            String::new(),
            "cpu_utilization".to_string(),
        );
        assert!(
            dv.init_visualizer("test/performance_data_2022-01-01_01_01_01/".to_string(), "test".to_string()).unwrap() == ()
        );
        assert!(dv.unpack_data("test".to_string()).unwrap() == ());
        let ret = dv.get_data("run=test&get=aggregate".to_string()).unwrap();
        let values: Vec<CpuData> = serde_json::from_str(&ret).unwrap();
        assert!(values[0].cpu == -1);
        match values[0].time {
            TimeEnum::TimeDiff(value) => assert!(value == 0),
            _ => assert!(false),
        }
        match values[1].time {
            TimeEnum::TimeDiff(value) => assert!(value == 1),
            _ => assert!(false),
        }
    }
}