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
    Generic(Box<dyn std::fmt::Display>),
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
            }
            Generic(e) => e.fmt(f),
            _ => unreachable!(),
        }
    }
}
fn program() -> Result<(), MainError> {
    let program_options = args::read_runinfo(RunInfo::get_from_env());
    eprintln!(" >> PROGRAM OPTIONS :: {:#?}", program_options);
    let config_root = gfunc::for_until(&program_options.config_paths, |p| {
        toml_context::TableRoot::from_file_path(p).ok()
    })
    .ok_or(MainError::NoConfigFileFound(program_options.config_paths))?;
    eprintln!(" >> CONFIG FILE :: {:?}", config_root.context);
    let master_config = configs::MasterConfig::from_table(&config_root.handle())?;
    eprintln!(" >> CONFIGS :: {:#?}", master_config);
    let scheme_registry = configs::SchemeRegistry::load_dir(Path::new(master_config.scheme_dir))
        .map_err(|e| MainError::Generic(Box::new(e)))?;
    let tagdir_paths = rsearch_dir(
        &program_options.root_dir,
        &program_options.tagdir_path,
        MetaType::Directory,
    )
    .map_err(|e| MainError::InvalidRootDir(program_options.root_dir, e))?;
    eprintln!(" >> TAGDIRS :: {:#?}", tagdir_paths);
    let tag_roots = tagdir_paths.into_iter().map(|path| tagfile::TagRoot::generate_from_dir(path)).collect::<Result<Vec<tagfile::TagRoot>, tagfile::GenerateErr>>()
        .map_err(|e| MainError::Generic(Box::new(e)))?;
    for tag_root in tag_roots {
        match &tag_root.groups {
            None => {
                let group = tagfile::TagGroup::from_table(&tag_root.main.handle())?;
            },
            Some(groups) => todo!(),
        }

    }
    eprintln!(" >> OK <<");
    Ok(())
}
fn main() {
    if let Err(e) = program() {
        eprint!("[FATAL!] :: {}", e);
        exit(1);
    }
}
