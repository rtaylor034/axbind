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
    MissingDefaultOptions(configs::ConfigError),
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
            MissingDefaultOptions(e) => {
                writeln!(f, "{}", e)?;
                if let configs::ConfigError::TableGet(te) = e {
                    if let toml_context::TableGetErr::NoKey = te.error {
                    return writeln!(f, "(All options MUST be specified in the master config file to be used as defaults)")
                    }
                }
                Ok(())
            }
            _ => unreachable!(),
        }
    }
}
fn program() -> Result<(), MainError> {
    let program_options = args::read_runinfo(RunInfo::get_from_env());
    eprintln!(" >> PROGRAM OPTIONS :: {:#?}", program_options);
    let (config_table, config_path) = args::priority_parse(&program_options.config_paths)
        .ok_or(MainError::NoConfigFileFound(program_options.config_paths))?;
    let config_handle =
        TableHandle::new_root(&config_table, config_path.to_string_lossy().to_string());
    eprintln!(" >> CONFIG FILE :: {:?}", config_path);
    let tagdir_paths = rsearch_dir(
        &program_options.root_dir,
        &program_options.tagdir_path,
        MetaType::Directory,
    )
    .map_err(|e| MainError::InvalidRootDir(program_options.root_dir, e))?;
    eprintln!(" >> TAGDIRS :: {:#?}", tagdir_paths);
    let (default_metaopts, default_opts) =
        get_default_options(&config_handle).map_err(|e| MainError::MissingDefaultOptions(e))?;
    eprintln!(" >> OK <<");
    Ok(())
}
pub fn get_default_options<'t>(
    config_handle: &TableHandle<'t>,
) -> Result<(configs::MetaOptions<'t>, configs::Options<'t>), configs::ConfigError> {
    use configs::{MetaOptions, Options};
    let metaopts_init =
        MetaOptions::from_table_forced(&extract_value!(Table, config_handle.get("metaoptions"))?)?;
    let opts_init =
        Options::from_table_forced(&extract_value!(Table, config_handle.get("options"))?)?;
    Ok((metaopts_init, opts_init))
}
fn main() {
    if let Err(e) = program() {
        eprint!("[FATAL!] :: {}", e);
        exit(1);
    }
}
