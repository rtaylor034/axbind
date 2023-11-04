use axbind::*;
use optwrite::OptWrite;
use gfunc::fnav::{rsearch_dir, MetaType};
use gfunc::run::RunInfo;
use std::path::{Path, PathBuf};
use std::process::exit;
use toml_context::{extract_value, TableHandle, Context};
//parse::<toml::Table>

///deserves to be rewritten tbh
fn program() -> Result<(), MainError> {
    let program_options = args::read_runinfo(RunInfo::get_from_env());
    eprintln!(" >> PROGRAM OPTIONS :: {:#?}", program_options);
    let config_root = gfunc::for_until(&program_options.config_paths, |p| {
        toml_context::TableRoot::from_file_path(p).ok()
    })
    .ok_or(MainError::NoConfigFileFound(program_options.config_paths))?;
    eprintln!(" >> CONFIG FILE :: {}", config_root.context);
    let master_config = configs::MasterConfig::from_table(&config_root.handle())?;
    eprintln!(" >> CONFIGS :: {:#?}", master_config);
    //TODO: fix scheme_dir not being relative to configuration directory.
    //perhaps add a gfunc function for easy relative/absolute parsing
    let scheme_path = PathBuf::from(String::clone(&config_root.context.branch)).with_file_name(master_config.scheme_dir);
    eprintln!(" >> FULL SCHEME DIR :: {:?}", scheme_path);
    let scheme_registry = configs::SchemeRegistry::load_dir(scheme_path.as_path())
        .map_err(|e| MainError::Generic(Box::new(e)))?;
    let tagdir_paths = rsearch_dir(
        &program_options.root_dir,
        &program_options.tagdir_path,
        MetaType::Directory,
    )
    .map_err(|e| MainError::InvalidRootDir(program_options.root_dir, e))?;
    eprintln!(" >> SCHEME REGISTRY :: {:#?}", scheme_registry);
    eprintln!(" >> TAGDIRS :: {:#?}", tagdir_paths);
    let tag_roots = tagdir_paths.into_iter().map(|path| tagfile::TagRoot::generate_from_dir(path)).collect::<Result<Vec<tagfile::TagRoot>, tagfile::GenerateErr>>()
        .map_err(|e| MainError::Generic(Box::new(e)))?;
    for tag_root in &tag_roots {
        match &tag_root.groups {
            None => evaluate_taggroup(&tag_root.main.handle(), &master_config.options, &scheme_registry, &master_config.meta_options)?,
            Some(groups) => 
                for group in groups {
                    evaluate_taggroup(&group.handle(), &master_config.options, &scheme_registry, &master_config.meta_options)?;
                }
        }

    }
    eprintln!(" >> OK <<");
    Ok(())
}
//cannot be bothered with this function signature, might as well be a macro.
fn evaluate_taggroup<'a>(tag_group_handle: &TableHandle<'a>, opt_basis: &configs::Options, registry: &'a configs::SchemeRegistry<'a>, meta_opts: &configs::MetaOptions) -> Result<(), MainError> {
    eprintln!(">> -- EVALUATING TAGGROUP :: {}", tag_group_handle.context);
    let tag_group = tagfile::TagGroup::from_table(tag_group_handle)?;
    let options = opt_basis.clone().overriden_by(tag_group.options);
    eprintln!(">> OPTIONS :: {:#?}", options);
    //cringe
    let bindings = get_bindings(&registry, &tag_group.scheme_spec, meta_opts, tag_group_handle.context.clone())?;
    eprintln!(">> BINDINGS :: {:#?}", bindings);
    for file in tag_group.files {
        eprintln!(">> AFFECTING FILE :: {}", file);
        let axbind_file = escaped_manip(file.as_str(), options.escape_char.unwrap(), |s| 
            s.replace(meta_opts.wildcard_char.unwrap(), file));
            eprintln!(">> AXBIND FILE :: {}", axbind_file);
        let axbind_contents = std::fs::read_to_string(&axbind_file)
            .map_err(|e| MainError::Generic(Box::new(e)))?;
            eprintln!(">> CONTENTS :: {}", axbind_contents);
        if let Err(e) = std::fs::write(file, axbind_replace(axbind_contents.as_str(), &bindings, &options).map_err(|e| MainError::ReplaceError(e))?.as_str()) {
            eprintln!("[Warn] Error writing to file '{}' (file skipped)", file);
            eprintln!(" - {}", e);
        }
    }
    Ok(())
}
fn main() {
    if let Err(e) = program() {
        eprint!("[FATAL!] :: {}", e);
        exit(1);
    }
}
