use axbind::*;
use gfunc::fnav::{rsearch_dir, MetaType};
use gfunc::run::RunInfo;
use std::path::{Path, PathBuf};
use std::process::exit;
//parse::<toml::Table>
fn main() {
    let program_options = args::read_runinfo(RunInfo::get_from_env());
    eprintln!(" >> PROGRAM OPTIONS :: {:#?}", program_options);

    let (configs, config_file) = match args::priority_parse(&program_options.config_paths) {
        Some(conf) => conf,
        None => {
            return {
                eprintln!("No valid config files found out of:");
                eprintln!("{:#?}", program_options.config_paths);
                eprintln!("(check for invalid toml syntax)");
            }
        }
    };
    println!(" >> CONFIG FILE :: {:?}", config_file);

    let tagdir_paths = match rsearch_dir(
        &program_options.root_dir,
        &program_options.tagdir_path,
        MetaType::Directory,
    ) {
        Ok(paths) => paths,
        Err(e) => {
            eprintln!(
                "!. Unable to read specified root dir: {:?}",
                &program_options.root_dir
            );
            eprintln!("{:?}", e);
            exit(1);
        }
    };
    eprintln!(" >> TAGDIRS :: {:#?}", tagdir_paths);

    eprintln!(" >> OK <<");
}
