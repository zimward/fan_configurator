use dialoguer::{Confirm, Input, MultiSelect};
use std::{
    fs::{read_to_string, write},
    io::Error,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use serde::Serialize;

use crate::heatsrc::search_paths;

fn default_max(val: &u8) -> bool {
    *val == 0xFF
}

fn default_cutoff(cutoff: &bool) -> bool {
    !*cutoff
}

#[derive(Serialize)]
pub struct Fan {
    name: String,
    wildcard_path: String,
    min_pwm: u8,
    #[serde(skip_serializing_if = "default_max")]
    max_pwm: u8,
    #[serde(skip_serializing_if = "default_cutoff")]
    cutoff: bool,
    heat_pressure_srcs: Vec<String>,
}

impl Fan {
    fn new(
        name: String,
        wildcard_path: String,
        min_pwm: u8,
        max_pwm: u8,
        cutoff: bool,
        heat_pressure_srcs: Vec<String>,
    ) -> Self {
        Self {
            name,
            wildcard_path,
            min_pwm,
            max_pwm,
            cutoff,
            heat_pressure_srcs,
        }
    }
}

fn enable_fan_pwm(path: &PathBuf, enable: bool) -> Result<(), Error> {
    //maybe terminate if this fails?
    if let Some(file) = path.file_name() {
        let mut pwm_enable = path.clone();
        //non-unicode chars should be impossible in file names
        let file = file.to_str().unwrap();
        pwm_enable.pop();
        pwm_enable.push(format!("{file}_enable"));
        match enable {
            true => write(pwm_enable.as_path(), "1")?,
            false => write(pwm_enable.as_path(), "0")?,
        }
    }
    Ok(())
}

fn read_rpm(path: &PathBuf) -> Result<u32, Error> {
    if let Some(fan_nr) = path.file_name() {
        if let Some(fan_nr) = fan_nr.to_string_lossy().strip_prefix("pwm") {
            let mut base = path.clone();
            base.pop();
            base.push(format!("fan{fan_nr}_input"));
            let mut rpm = read_to_string(base)?;
            rpm.pop(); //pop newline
            let rpm: u32 = rpm.parse().unwrap_or(0);
            Ok(rpm)
        } else {
            Ok(0)
        }
    } else {
        Ok(0)
    }
}

fn ask_fan(path: &PathBuf, heat_srcs: Option<&Vec<String>>) -> Result<Option<Fan>, Error> {
    let canonical_path = path.canonicalize()?;
    let heat_srcs = heat_srcs.unwrap();
    //enable manual pwm control
    enable_fan_pwm(path, true)?;
    //ramp fan up
    println!(
        "Ramping fan {} up...",
        canonical_path.to_str().unwrap_or("error")
    );
    write(path, "255")?;
    //wait for fan to reach rpm
    thread::sleep(Duration::from_secs(3));
    //check rpm
    let rpm = read_rpm(path)?;
    if rpm == 0 {
        println!("Fan rpm reads 0. skipping...");
        return Ok(None);
    }
    println!("Fan reached rpm of {rpm}");
    println!("Undulating Fan for easier identification.");
    let undulate = Arc::new(AtomicBool::new(true));
    {
        let copy = path.clone();
        let undulate = Arc::clone(&undulate);
        thread::spawn(move || {
            //ignore failed writes
            loop {
                let _ = write(&copy, "0");
                thread::sleep(Duration::from_secs(10));
                if !undulate.load(Ordering::Relaxed) {
                    break;
                }
                let _ = write(&copy, "255");
                thread::sleep(Duration::from_secs(10));
                if !undulate.load(Ordering::Relaxed) {
                    break;
                }
            }
        })
    };

    if Confirm::new()
        .with_prompt("Do you want to add this fan to config?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        undulate.store(false, Ordering::Relaxed);
        let name: String = Input::new().with_prompt("Enter fan name").interact_text()?;

        let mut heat_pressure_srcs: Vec<usize>;
        loop {
            heat_pressure_srcs = MultiSelect::new()
                .with_prompt("Select heat pressure sources")
                .items(heat_srcs)
                .interact()?;
            if heat_pressure_srcs.len() == 0 {
                println!("You must select at leat one heat pressure source!");
            } else {
                break;
            }
        }
        let heat_pressure_srcs = heat_pressure_srcs
            .iter()
            .filter_map(|index| heat_srcs.get(*index))
            .map(|a| a.clone())
            .collect();

        //min pwm
        println!("Searching minimum pwm value. This can take a while.");
        let mut min_pwm: u8 = 0;
        write(path, "0")?;
        println!("Waiting for fan to stop.");
        let mut rpm = 0;
        for _ in 0..30 {
            thread::sleep(Duration::from_secs(1));
            rpm = read_rpm(path)?;
            if rpm == 0 {
                break;
            }
        }
        if rpm == 0 {
            println!("Fan stopped.");
            println!("Searching the start pwm value of the fan.");
            loop {
                min_pwm = min_pwm.saturating_add(1);
                write(path, min_pwm.to_string())?;
                thread::sleep(Duration::from_millis(500));
                let rpm = read_rpm(path)?;
                if rpm != 0 {
                    println!("Fan starts spinning at {min_pwm}.");
                    break;
                }
            }
        } else {
            println!("Fan seems to be unable to stop. selecting 0 as min pwm.");
        }

        //max pwm
        let mut max_pwm: u8 = 255;
        println!("Setting fan pwm to {max_pwm}");
        loop {
            write(path, max_pwm.to_string())?;
            if Confirm::new()
                .with_prompt("Is this maximum fan speed quiet enough for you")
                .interact()?
            {
                break;
            }
            max_pwm = max_pwm.saturating_sub(5);
            if max_pwm <= min_pwm {
                max_pwm = min_pwm;
                println!("Selecting min pwm as max pwm value. Are you sure you want it to be this low? (Fan is going to constantly run at minimum speed)");
                break;
            }
        }
        let cutoff = Confirm::new()
            .with_prompt("Should the fan be stopped when minimum pwm is reached")
            .interact()?;

        enable_fan_pwm(path, false)?;
        Ok(Some(Fan::new(
            name,
            path.to_string_lossy().to_string(),
            min_pwm,
            max_pwm,
            cutoff,
            heat_pressure_srcs,
        )))
    } else {
        undulate.store(false, Ordering::Relaxed);
        enable_fan_pwm(path, false)?;
        Ok(None)
    }
}

pub fn search_fans(heat_srcs: Vec<String>) -> Vec<Fan> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/pwm*[0-9]"];
    search_paths(&possible_paths, ask_fan, Some(&heat_srcs))
}
