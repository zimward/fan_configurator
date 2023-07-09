use std::{
    fs::{read_to_string, write},
    io::Error,
    path::PathBuf,
    process::ExitCode,
    thread,
    time::Duration,
};

use deps::check_dependencies;
use dialoguer::{Confirm, Input, Select};
use glob::glob;

mod deps;

fn search_paths<F, R>(possible_paths: &[&str], question_fn: F) -> Vec<R>
where
    F: Fn(&PathBuf) -> Result<Option<R>, Error>,
{
    let mut srcs: Vec<R> = Vec::default();
    for path in possible_paths.iter() {
        //find matches
        let paths = glob(path);
        if paths.is_err() {
            continue;
        }
        //unwrap if value was ok
        let paths = paths.unwrap();
        //remove err variants
        let paths = paths.filter_map(|p| {
            if let Ok(path) = p {
                Some(path.clone())
            } else {
                None
            }
        });
        for path in paths {
            //if parameterization fails at one step fan/heatsrc is ignored
            if let Ok(val) = question_fn(&path) {
                if let Some(toadd) = val {
                    //resolve hwmon path
                    //failing should be impossible at this point
                    srcs.push(toadd);
                }
            }
        }
    }
    srcs
}
#[allow(dead_code)]
struct Pid {
    set_point: f32,
    p: f32,
    i: f32,
    d: f32,
}

#[allow(dead_code)]
struct HeatSrc {
    name: String,
    wildcard_path: String,
    pid_params: Pid,
}
impl HeatSrc {
    fn new(name: String, wildcard_path: String, set_point: f32) -> Self {
        Self {
            name,
            wildcard_path,
            pid_params: Pid {
                set_point,
                p: 0.0,
                i: 0.0,
                d: 0.0,
            },
        }
    }
}

fn ask_heat_src(path: &PathBuf) -> Result<Option<HeatSrc>, Error> {
    let mut value = read_to_string(path)?;
    value.pop();
    let mut value: f32 = value.parse().unwrap_or(0.0);
    value /= 1000.0;
    let label = path.to_str().unwrap_or("");
    let label = label.replace("input", "label");
    let name = read_to_string(label)?;
    let full_path = path.canonicalize()?.to_str().unwrap_or("").to_string();
    println!("Found source {full_path}");
    println!("Current value:{value}°C. Label:{name}");
    if Confirm::new()
        .with_prompt("Do you want to add this heat source to config?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        //configure heatsrc
        let name: String = Input::new()
            .with_prompt("Enter heat pressure source name")
            .interact_text()?;
        let set_point: String = Input::new()
            .with_prompt("Enter setpoint (°C)")
            .validate_with(|input: &String| {
                for c in input.chars() {
                    if c.is_alphabetic() {
                        return Err("You may only enter a number.");
                    }
                }
                Ok(())
            })
            .interact_text()?;
        Ok(Some(HeatSrc::new(
            name,
            full_path,
            set_point.parse().unwrap_or(60.0),
        )))
    } else {
        Ok(None)
    }
}

fn search_heat_srcs() -> Vec<HeatSrc> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/temp*_input"];
    search_paths(&possible_paths, ask_heat_src)
}

#[allow(dead_code)]
struct Fan<'a> {
    name: String,
    wildcard_path: String,
    min_pwm: u8,
    max_pwm: u8,
    cutoff: bool,
    heat_pressure_srcs: &'a [&'a str],
}

impl Default for Fan<'static> {
    fn default() -> Self {
        Self {
            name: "none".to_owned(),
            wildcard_path: "/dev/null".to_owned(),
            min_pwm: 0,
            max_pwm: 255,
            cutoff: false,
            heat_pressure_srcs: &[],
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

fn ask_fan(path: &PathBuf) -> Result<Option<Fan<'static>>, Error> {
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
    thread::sleep(Duration::from_secs(1));
    //check rpm
    let rpm = read_rpm(path)?;
    if rpm == 0 {
        println!("Fan rpm reads 0. skipping...");
        return Ok(None);
    }
    println!("Fan reached rpm of {rpm}");
    if Confirm::new()
        .with_prompt("Do you want to add this fan to config?")
        .default(true)
        .interact()
        .unwrap_or(false)
    {
        Ok(Some(Fan::default()))
    } else {
        Ok(None)
    }
}

fn search_fans() -> Vec<Fan<'static>> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/pwm*[0-9]"];
    search_paths(&possible_paths, ask_fan)
}

fn main() -> ExitCode {
    /* if let Err(exit) = check_dependencies() {
        return exit;
    }*/
    //search_heat_srcs();
    search_fans();
    ExitCode::SUCCESS
}
