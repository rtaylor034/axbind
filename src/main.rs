use axbind::*;
use gfunc::fnav::{rsearch_dir, MetaType};
use gfunc::run::RunInfo;
use std::path::{Path, PathBuf};
use std::process::exit;
use toml_context::{extract_value, TableHandle};
//parse::<toml::Table>

pub enum MainError {
    NoConfigFileFound(Vec<PathBuf>),
    InvalidRootDir(PathBuf, std::io::Error),
    ConfigError(configs::ConfigError),
}
impl From<configs::ConfigError> for MainError {
    fn from(value: configs::ConfigError) -> Self {
        Self::ConfigError(value)
    }
}
impl std::fmt::Display for MainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use MainError::*;
        match self {
            ConfigError(e) => e.fmt(f),
            NoConfigFileFound(paths) => {
                writeln!(f, "No valid fonfig files found out of:")?;
                writeln!(f, "{:#?}", paths)?;
                writeln!(f, "(check for invalid toml syntax)")
            }
            InvalidRootDir(path, ioe) => {
                writeln!(f, "Unable to read specified root dir: '{:?}'", path)?;
                writeln!(f, "{}", ioe)
            },
            _ => unreachable!(),
        }
    }
}
fn program() -> Result<(), MainError> {
    let program_options = args::read_runinfo(RunInfo::get_from_env());
    eprintln!(" >> PROGRAM OPTIONS :: {:#?}", program_options);
    let (config_table, config_path) = args::priority_parse(&program_options.config_paths)
        .ok_or(MainError::NoConfigFileFound(program_options.config_paths))?;
    eprintln!(" >> CONFIG FILE :: {:?}", config_path);
    let master_config = configs::MasterConfig::from_table(
        &TableHandle::new_root(&config_table, config_path.to_string_lossy().to_string()))?;
    eprintln!(" >> CONFIGS :: {:#?}", master_config);
    let tagdir_paths = rsearch_dir(
        &program_options.root_dir,
        &program_options.tagdir_path,
        MetaType::Directory,
    )
    .map_err(|e| MainError::InvalidRootDir(program_options.root_dir, e))?;
    eprintln!(" >> TAGDIRS :: {:#?}", tagdir_paths);
    eprintln!(" >> OK <<");
    Ok(())
}
fn main() {
    if let Err(e) = program() {
        eprint!("[FATAL!] :: {}", e);
        exit(1);
    }
}
