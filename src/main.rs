use std::{fs::read_to_string, path::PathBuf};

use dialoguer::Confirm;
use glob::glob;

fn get_modules() -> Vec<String> {
    let modules = read_to_string("/proc/modules").unwrap();
    let mut mods: Vec<String> = Vec::default();
    for line in modules.split_terminator('\n') {
        if let Some(m) = line.split_once(' ') {
            mods.push(m.0.to_string());
        }
    }
    mods
}

fn is_module_present(module: &str, modules: &Vec<String>) -> bool {
    for m in modules {
        if module == m {
            return true;
        }
    }
    false
}

//look for modules
fn check_dependencies() {
    let supported_modules = ["nct6775"];
    let modules = get_modules();
    let mut found = false;
    for m in supported_modules.iter() {
        let present = is_module_present(&m, &modules);
        if present {
            println!("Found module {m}!");
            found = true;
        }
    }
    if found {
        println!("No fan/sensor modules have been loaded!");
        if Confirm::new()
            .with_prompt("Do you want to try loading possible modules?")
            .default(true)
            .interact()
            .unwrap()
        {
            for m in supported_modules.iter() {
                println!("Trying to load {m}");
            }
        }
    }
}

fn search_fans() -> Vec<String> {
    Vec::default()
}

fn ask_heat_src(path: &PathBuf) -> bool {
    let mut value = read_to_string(path).unwrap();
    value.pop();
    let mut value: f32 = value.parse().unwrap();
    value /= 1000.0;
    let label = path.to_str().unwrap();
    let label = label.replace("input", "label");
    let name = read_to_string(label).unwrap();
    println!(
        "Found source {}",
        path.canonicalize().unwrap().to_str().unwrap()
    );
    println!("Current value:{value}°C. Label:{name}");
    Confirm::new()
        .with_prompt("Do you want to add this heat source to config?")
        .default(true)
        .interact()
        .unwrap_or(false)
}

fn search_heat_srcs() -> Vec<String> {
    let mut srcs: Vec<String> = Vec::default();
    let possible_paths = ["/sys/class/hwmon/hwmon*/temp*_input"];
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
            if ask_heat_src(&path) {
                //resolve hwmon path
                srcs.push(path.canonicalize().unwrap().to_str().unwrap().to_string());
            }
        }
    }
    for s in &srcs {
        println!("{s}");
    }
    srcs
}

fn main() {
    //check_dependencies();
    search_heat_srcs();
}
