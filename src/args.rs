use gfunc::run::*;
use gfunc::{for_until, simple_envpath};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use toml;

#[derive(Debug)]
pub struct ProgramOptions {
    pub config_paths: Vec<PathBuf>,
    pub root_dir: PathBuf,
    pub tagdir_path: PathBuf,
}
pub fn read_runinfo(runinfo: RunInfo) -> ProgramOptions {
    let valid_singlet_opts: [(&'static str, Option<char>); 0] = [];
    let valid_valued_opts: [&'static str; 2] = ["tagdir", "config"];
    let valued_opts = runinfo.values.validate(valid_valued_opts).auto_exit();
    let _singlet_opts = runinfo.options.validate(valid_singlet_opts).auto_exit();
    let args = runinfo
        .arguements
        .validate_exact([|_: &_| true])
        .auto_exit();
    let root_dir = PathBuf::from(
        args.get(0)
            .expect("No arg[0] found, but passed validation?"),
    );
    let tagdir_path = match valued_opts.get("tagdir") {
        Some(tagdir) => PathBuf::from(tagdir),
        None => PathBuf::from(".axbind"),
    };
    let config_paths = match valued_opts.get("config") {
        Some(cfgpath) => vec![PathBuf::from(cfgpath)],
        None => vec![
            "$XDG_CONFIG_HOME/axbind/config.toml",
            "$HOME/.config/axbind/config.toml",
            "/etc/axbind/config.toml",
        ]
        .iter()
        .filter_map(|path| simple_envpath(path).ok())
        .collect(),
    };
    let o = ProgramOptions {
        root_dir,
        tagdir_path,
        config_paths,
    };
    o
}
pub fn priority_parse<I, T>(paths: I) -> Option<(toml::Table, PathBuf)>
where
    T: AsRef<Path>,
    I: IntoIterator<Item = T>,
{
    for_until(paths, |p| {
        let path = p.as_ref();
        fs::read_to_string(path)
            .ok()
            .and_then(|pathstr| pathstr.parse().ok())
            .map(|table| (table, path.to_owned()))
    })
}
