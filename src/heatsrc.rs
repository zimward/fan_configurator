use dialoguer::{Confirm, Input};
use glob::glob;
use serde::Serialize;
use std::{fs::read_to_string, io::Error, path::PathBuf};

pub fn search_paths<F, R>(
    possible_paths: &[&str],
    question_fn: F,
    param: Option<&Vec<String>>,
) -> Vec<R>
where
    F: Fn(&PathBuf, Option<&Vec<String>>) -> Result<Option<R>, Error>,
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
            if let Ok(val) = question_fn(&path, param) {
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

#[derive(Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub struct Pid {
    #[serde(rename = "set_point")]
    pub set_point: f32,
    pub p: f32,
    pub i: f32,
    pub d: f32,
}

#[derive(Serialize)]
pub struct HeatSrc {
    pub name: String,
    pub wildcard_path: String,
    pub pid_params: Pid,
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

fn ask_heat_src(path: &PathBuf, _: Option<&Vec<String>>) -> Result<Option<HeatSrc>, Error> {
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
        let set_point: f32 = Input::new()
            .with_prompt("Enter setpoint (°C)")
            .interact_text()?;
        Ok(Some(HeatSrc::new(name, full_path, set_point)))
    } else {
        Ok(None)
    }
}

pub fn search_heat_srcs() -> Vec<HeatSrc> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/temp*_input"];
    search_paths(&possible_paths, ask_heat_src, None)
}
