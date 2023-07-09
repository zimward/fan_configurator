use std::{fs::read_to_string, process::ExitCode};

use dialoguer::Confirm;
use nix::unistd::Uid;

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

fn is_root() -> Result<(), ExitCode> {
    if !Uid::effective().is_root() {
        eprintln!("This program has to be run with root permissions!");
        Err(ExitCode::FAILURE)
    } else {
        Ok(())
    }
}

//look for modules
pub fn check_dependencies() -> Result<(), ExitCode> {
    is_root()?;
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
    if !found {
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
    Ok(())
}
