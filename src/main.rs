use std::{fs::read_to_string, io::Error, path::PathBuf};

use deps::check_dependencies;
use dialoguer::Confirm;
use glob::glob;

mod deps;

fn search_fans() -> Vec<String> {
    Vec::default()
}

fn ask_heat_src(path: &PathBuf) -> Result<bool, Error> {
    let mut value = read_to_string(path)?;
    value.pop();
    let mut value: f32 = value.parse().unwrap_or(0.0);
    value /= 1000.0;
    let label = path.to_str().unwrap_or("");
    let label = label.replace("input", "label");
    let name = read_to_string(label)?;
    println!(
        "Found source {}",
        path.canonicalize()?.to_str().unwrap_or("")
    );
    println!("Current value:{value}°C. Label:{name}");
    Ok(Confirm::new()
        .with_prompt("Do you want to add this heat source to config?")
        .default(true)
        .interact()
        .unwrap_or(false))
}

fn search_paths<F>(possible_paths: &[&str], question_fn: F) -> Vec<String>
where
    F: Fn(&PathBuf) -> Result<bool, Error>,
{
    let mut srcs: Vec<String> = Vec::default();
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
            if let Ok(add) = question_fn(&path) {
                if add {
                    //resolve hwmon path
                    //failing should be impossible at this point
                    srcs.push(path.canonicalize().unwrap().to_str().unwrap().to_string());
                }
            }
        }
    }
    srcs
}

fn search_heat_srcs() -> Vec<String> {
    let possible_paths = ["/sys/class/hwmon/hwmon*/temp*_input"];
    search_paths(&possible_paths, ask_heat_src)
}

fn main() {
    check_dependencies();
    search_heat_srcs();
}
