use anyhow::{Ok, Result};
use clap::Args;

#[derive(Args, Debug)]
pub struct CustomPMU {}

pub fn custom_pmu(_custom_pmu: &CustomPMU) -> Result<()> {
    print!("Please refer to src/pmu_configs to create a custom PMU config file.");
    Ok(())
}
