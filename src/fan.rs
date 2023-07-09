use dialoguer::{Confirm, Input};
use std::{
    fs::{read_to_string, write},
    io::Error,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};

use crate::heatsrc::search_paths;

pub struct Fan {
    name: String,
    wildcard_path: String,
    min_pwm: u8,
    max_pwm: u8,
    cutoff: bool,
    heat_pressure_srcs: Vec<String>,
}

impl Default for Fan {
    fn default() -> Self {
        Self {
            name: "none".to_owned(),
            wildcard_path: "/dev/null".to_owned(),
            min_pwm: 0,
            max_pwm: 255,
            cutoff: false,
            heat_pressure_srcs: Vec::default(),
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
        println!("enableing");
        match enable {
            true => write(pwm_enable.as_path(), "1")?,
            false => write(pwm_enable.as_path(), "0")?,
        }
        println!("finished");
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

fn ask_fan(path: &PathBuf) -> Result<Option<Fan>, Error> {
    let canonical_path = path.canonicalize()?;
    //enable manual pwm control
    //enable_fan_pwm(path, true)?;
    //ramp fan up
    println!(
        "Ramping fan {} up...",
        canonical_path.to_str().unwrap_or("error")
    );
    //write(path, "255")?;
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
    let handle = {
        let copy = path.clone();
        let undulate = Arc::clone(&undulate);
        thread::spawn(move || {
            //ignore failed writes
            while undulate.load(Ordering::Relaxed) {
                let _ = write(&copy, "0");
                thread::sleep(Duration::from_secs(10));
                let _ = write(&copy, "255");
                thread::sleep(Duration::from_secs(10));
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
        enable_fan_pwm(path, false)?;
        let _ = handle.join();
        let name: String = Input::new().with_prompt("Enter fan name").interact_text()?;

        Ok(Some(Fan::default()))
    } else {
        undulate.store(false, Ordering::Relaxed);
        enable_fan_pwm(path, false)?;
        let _ = handle.join();
        Ok(None)
    }
}

pub fn search_fans() -> Vec<Fan> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/pwm*[0-9]"];
    search_paths(&possible_paths, ask_fan)
}
