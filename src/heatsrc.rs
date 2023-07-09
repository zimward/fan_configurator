use dialoguer::{Confirm, Input};
use glob::glob;
use std::{fs::read_to_string, io::Error, path::PathBuf};

pub fn search_paths<F, R>(possible_paths: &[&str], question_fn: F) -> Vec<R>
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
pub struct Pid {
    set_point: f32,
    p: f32,
    i: f32,
    d: f32,
}

pub struct HeatSrc {
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

pub fn search_heat_srcs() -> Vec<HeatSrc> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/temp*_input"];
    search_paths(&possible_paths, ask_heat_src)
}
