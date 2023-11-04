use axbind::*;
use optwrite::OptWrite;
use gfunc::fnav::{rsearch_dir, MetaType};
use gfunc::run::RunInfo;
use std::path::{Path, PathBuf};
use std::process::exit;
use toml_context::{extract_value, TableHandle, Context};
//parse::<toml::Table>

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
    for tag_root in &tag_roots {
        match &tag_root.groups {
            None => {
                /*
                let main = tagfile::TagGroup::from_table(&tag_root.main.handle())?;
                let options = master_config.options.clone().overriden_by(main.options);
                let scheme = scheme_registry.get(&main.scheme_spec.scheme)?
                    .ok_or(MainError::SchemeExpected(main.scheme_spec.scheme.clone(), tag_root.main.context.clone()))?;
                for file in main.files {
                    let axbind_file = escaped_manip(file.as_str(), options.escape_char.unwrap(), |s| 
                        s.replace(master_config.meta_options.wildcard_char.unwrap(), file));
                    }
                */
                evaluate_tagroot(tag_root, &master_config.options, &scheme_registry, &master_config.meta_options)?;
            },
            Some(groups) => todo!(),
        }

    }
    eprintln!(" >> OK <<");
    Ok(())
}
//cringe
fn evaluate_tagroot<'a>(tag_root: &'a tagfile::TagRoot, opt_basis: &configs::Options, registry: &'a configs::SchemeRegistry<'a>, meta_opts: &configs::MetaOptions) -> Result<(), MainError> {
    let tag_group = tagfile::TagGroup::from_table(&tag_root.main.handle())?;
    let options = opt_basis.clone().overriden_by(tag_group.options);
    //cringe
    let bindings = get_bindings(&registry, &tag_group.scheme_spec, meta_opts, tag_root.main.context.clone())?;
    for file in tag_group.files {
        let axbind_file = escaped_manip(file.as_str(), options.escape_char.unwrap(), |s| 
            s.replace(meta_opts.wildcard_char.unwrap(), file));
        let axbind_contents = std::fs::read_to_string(&axbind_file)
            .map_err(|e| MainError::Generic(Box::new(e)))?;
        }
    todo!();
}
fn main() {
    if let Err(e) = program() {
        eprint!("[FATAL!] :: {}", e);
        exit(1);
    }
}
