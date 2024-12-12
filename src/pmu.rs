#[cfg(target_arch = "aarch64")]
use crate::data::perf_stat::arm64_perf_list;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use crate::data::perf_stat::{form_events_map, x86_perf_list};
use crate::data::perf_stat::{to_events, NamedCtr, NamedTypeCtr, PerfType};
use crate::PDError;

use anyhow::Result;
use clap::Args;
use inquire::{
    list_option::ListOption, required, validator::Validation, Confirm, MultiSelect, Select, Text,
};
use std::path::PathBuf;

#[derive(Args, Debug)]
pub struct CustomPMU {
    /// Name of the file for the custom PMU configuration.
    #[clap(short, long, value_parser)]
    pub pmu_file: Option<String>,

    /// Verify the supplied pmu file.
    #[clap(long, value_parser)]
    pub verify: bool,
}

pub fn get_ctrs(opt_str: &str) -> Result<Vec<NamedTypeCtr>> {
    println!("   \"{opt_str}\": [");
    let mut ret: Vec<NamedTypeCtr> = Vec::new();
    loop {
        println!("     {{");
        println!("       \"perf_type\": RAW");
        let perf_type = PerfType::RAW;
        let name = Text::new("     \"name\":")
            .with_validator(|s: &str| {
                if s.chars().all(char::is_alphanumeric) {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(
                        "Name must contain only alphanumeric characters.".into(),
                    ))
                }
            })
            .with_validator(required!())
            .prompt()?;
        let raw_config = Text::new("     \"config\":")
            .with_validator(|s: &str| {
                if s.starts_with("0x") {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(
                        "Config must be hexadecimal and start with 0x.".into(),
                    ))
                }
            })
            .with_validator(|s: &str| {
                let no_prefix = s.trim_start_matches("0x");
                match u64::from_str_radix(no_prefix, 16) {
                    Ok(_) => Ok(Validation::Valid),
                    Err(_) => Ok(Validation::Invalid("Invalid hexadecimal value.".into())),
                }
            })
            .prompt()?;
        let no_prefix = raw_config.trim_start_matches("0x");
        let config = u64::from_str_radix(no_prefix, 16)?;
        println!("     }}");
        ret.push(NamedTypeCtr {
            perf_type,
            name,
            config,
        });
        if !Confirm::new(format!("Add more {opt_str}:").as_str())
            .with_default(false)
            .prompt()?
        {
            println!("\n   ]");
            break;
        }
    }
    Ok(ret)
}

pub fn add_events(existing_events: Vec<NamedCtr>) -> Result<Vec<NamedCtr>> {
    let mut events: Vec<NamedCtr> = Vec::new();
    let mut event_names = Vec::new();
    for event in existing_events {
        event_names.push(event.name);
    }
    loop {
        println!("{{");
        let event_names_tmp = event_names.clone();
        let name = Text::new(" \"name\":")
            .with_validator(|s: &str| {
                if s.chars().all(char::is_alphanumeric) {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(
                        "Name must contain only alphanumeric characters.".into(),
                    ))
                }
            })
            .with_validator(move |s: &str| {
                if !event_names_tmp.contains(&s.to_string()) {
                    Ok(Validation::Valid)
                } else {
                    Ok(Validation::Invalid(
                        "Event with the same name exists.".into(),
                    ))
                }
            })
            .with_validator(required!())
            .prompt()?;
        event_names.push(name.clone());
        let nrs = get_ctrs("nrs")?;
        let drs = get_ctrs("drs")?;
        let scale_text = Text::new(" \"scale\":")
            .with_validator(required!())
            .with_validator(|s: &str| match s.parse::<u64>() {
                Ok(v) => {
                    if v == 0 {
                        Ok(Validation::Invalid("Scaling value cannot be 0.".into()))
                    } else {
                        Ok(Validation::Valid)
                    }
                }
                Err(_) => Ok(Validation::Invalid(
                    "Scaling value should be a valid number.".into(),
                )),
            })
            .prompt()?;
        let scale = scale_text.parse::<u64>()?;
        events.push(NamedCtr {
            name,
            nrs,
            drs,
            scale,
        });
        println!("  }}");
        if !Confirm::new("Add more events:")
            .with_default(false)
            .prompt()?
        {
            break;
        }
    }
    Ok(events)
}

pub fn delete_events(mut perf_list: Vec<NamedCtr>) -> Result<Vec<NamedCtr>> {
    loop {
        let mut ev_list = Vec::new();
        for event in &perf_list {
            ev_list.push(event.name.clone());
        }
        if ev_list.is_empty() {
            println!("Cannot delete any more events.");
            return Ok(Vec::new());
        }
        let delete_list = MultiSelect::new("Select event(s) to delete:", ev_list)
            .with_validator(|a: &[ListOption<&String>]| {
                if a.is_empty() {
                    Ok(Validation::Invalid("Must choose at least 1 option.".into()))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?;
        for name in delete_list {
            let index = perf_list.iter().position(|ev| ev.name == name).unwrap();
            let out = format!(
                "\n{}\nDelete?",
                serde_json::to_string_pretty(&perf_list[index])?
            );
            if Confirm::new(&out).with_default(true).prompt()? {
                perf_list.remove(index);
            }
        }
        if !Confirm::new("Remove more?").with_default(false).prompt()? {
            break;
        }
    }
    Ok(perf_list.to_vec())
}

pub fn create_pmu_config(cpmu: &CustomPMU) -> Result<()> {
    let mut pmu_file = PathBuf::from("aperf_custom_pmu.json");
    if let Some(f) = &cpmu.pmu_file {
        pmu_file = PathBuf::from(f);
    }
    #[cfg(target_arch = "aarch64")]
    let events: Vec<NamedCtr> = serde_json::from_slice(arm64_perf_list::GRV_EVENTS)?;
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let events: Vec<NamedCtr> = serde_json::from_slice(x86_perf_list::INTEL_EVENTS)?;
    #[cfg(target_arch = "aarch64")]
    let platform = "Graviton";
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let platform = "Intel";
    println!(
        "\nAperf PMU Event structure (Ex: {})\n{}",
        platform,
        serde_json::to_string_pretty(&events[0])?
    );
    println!(
        "\nPlease enter your custom PMU details. Only hex values (0x) for 'config' will work.\n"
    );
    let events = add_events(Vec::new())?;
    let f = std::fs::File::create(&pmu_file)?;
    serde_json::to_writer_pretty(f, &events)?;
    println!(
        "\nCustom PMU config generated at: {:?}. Use this with 'aperf record --pmu-file {:?}'.",
        pmu_file, pmu_file
    );
    Ok(())
}

pub fn get_config(choice: &str, cpmu: &CustomPMU) -> Result<Vec<NamedCtr>> {
    if choice == "User provided" {
        if let Some(f) = &cpmu.pmu_file {
            let file = std::fs::File::open(PathBuf::from(f))?;
            return Ok(serde_json::from_reader(&file)?);
        } else {
            println!("No custom config file provided.");
            return Err(PDError::PMUCustomFileNotFound.into());
        }
    };
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    match choice {
        "Intel" => to_events(x86_perf_list::INTEL_EVENTS),
        "Intel Sapphire Rapids" => {
            form_events_map(x86_perf_list::INTEL_EVENTS, x86_perf_list::SPR_CTRS)
        }
        "Intel Icelake" => form_events_map(x86_perf_list::INTEL_EVENTS, x86_perf_list::ICX_CTRS),
        "AMD" => to_events(x86_perf_list::AMD_EVENTS),
        "AMD Genoa" => form_events_map(x86_perf_list::AMD_EVENTS, x86_perf_list::GENOA_CTRS),
        "AMD Milan" => form_events_map(x86_perf_list::AMD_EVENTS, x86_perf_list::MILAN_CTRS),
        _ => Ok(Vec::new()),
    }
    #[cfg(target_arch = "aarch64")]
    match choice {
        "Graviton" => to_events(arm64_perf_list::GRV_EVENTS),
        _ => Ok(Vec::new()),
    }
}

pub fn modify_existing_config(cpmu: &CustomPMU) -> Result<()> {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    let config = Select::new(
        "Select a config:",
        vec![
            "User provided",
            "Intel",
            "Intel Sapphire Rapids",
            "Intel Icelake",
            "AMD",
            "AMD Genoa",
            "AMD Milan",
        ],
    )
    .prompt()?;
    #[cfg(target_arch = "aarch64")]
    let config = Select::new("Select a config:", vec!["User provided", "Graviton"]).prompt()?;
    let mut perf_list = get_config(config, cpmu)?;
    loop {
        let option = Select::new("Select action:", vec!["Add", "Delete", "Done"]).prompt()?;
        if option == "Add" {
            if perf_list.is_empty() {
                #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                let example: Vec<NamedCtr> = to_events(x86_perf_list::INTEL_EVENTS)?;
                #[cfg(target_arch = "aarch64")]
                let example: Vec<NamedCtr> = to_events(arm64_perf_list::GRV_EVENTS)?;
                println!(
                    "\nAperf PMU Event structure (Example)\n{}",
                    serde_json::to_string_pretty(&example[0])?
                );
            } else {
                println!(
                    "\nAperf PMU Event structure (Ex: {})\n{}",
                    config,
                    serde_json::to_string_pretty(&perf_list[0])?
                );
            }
            perf_list.append(&mut add_events(perf_list.clone())?);
        } else if option == "Delete" {
            perf_list = delete_events(perf_list.clone())?;
        } else if option == "Done" {
            let f = std::fs::File::create("aperf_existing_modified.json")?;
            serde_json::to_writer_pretty(f, &perf_list)?;
            println!(
                "\nCustom PMU config generated at: aperf_existing_modified.json. Use this with 'aperf record --pmu-file aperf_existing_modified.json'.",
            );
            break;
        }
    }
    Ok(())
}

pub fn custom_pmu(cpmu: &CustomPMU) -> Result<()> {
    if cpmu.verify {
        if let Some(custom_file) = &cpmu.pmu_file {
            let f = std::fs::File::open(custom_file)?;
            let check_format: Result<Vec<NamedCtr>, serde_json::Error> =
                serde_json::from_reader(&f);
            if check_format.is_ok() {
                println!("Custom PMU file is valid.");
                return Ok(());
            } else {
                return Err(PDError::PMUFileInvalid.into());
            }
        }
        return Err(PDError::PMUCustomFileNotFound.into());
    }
    let choice = Select::new(
        "Aperf Custom PMU config:",
        vec!["Create from scratch", "Modify existing config"],
    )
    .prompt()?;
    if choice == "Create from scratch" {
        create_pmu_config(cpmu)
    } else {
        modify_existing_config(cpmu)
    }
}
