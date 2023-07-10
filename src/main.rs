use std::{
    fs::{write, File},
    path::Path,
    process::ExitCode,
};

use deps::check_dependencies;
use fan::{search_fans, Fan};
use heatsrc::{search_heat_srcs, HeatSrc};
use serde::Serialize;

mod deps;
mod fan;
mod heatsrc;

fn write_config(heat_srcs: &Vec<HeatSrc>, fans: &Vec<Fan>, config_path: &Path) {
    #[derive(Serialize)]
    struct Cfg<'a> {
        heat_srcs: &'a Vec<HeatSrc>,
        fans: &'a Vec<Fan>,
    }
    let json =
        serde_json::to_string_pretty(&Cfg { heat_srcs, fans }).expect("Failed to serialize config");
    write(config_path, json).expect("Failed to write config.");
}

fn tune_pid(heat_srcs: &mut Vec<HeatSrc>, fans: &Vec<Fan>) {}

fn main() -> ExitCode {
    if let Err(exit) = check_dependencies() {
        return exit;
    }
    let heat_srcs = search_heat_srcs();
    let mut heat_src_names: Vec<String> = Vec::default();
    for src in heat_srcs.iter() {
        heat_src_names.push(src.name.clone());
    }
    let fans = search_fans(heat_src_names);
    write_config(&heat_srcs, &fans, &Path::new("/tmp/test-config.json"));
    ExitCode::SUCCESS
}
