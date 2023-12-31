use axbind::*;
use gfunc::fnav::{rsearch_dir, MetaType};
use gfunc::run::RunInfo;
use optwrite::OptWrite;
use std::path::{PathBuf};
use std::process::exit;
use toml_context::{TableHandle};
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
    let scheme_path = PathBuf::from(String::clone(&config_root.context.branch))
        .with_file_name(master_config.scheme_dir);
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
    //this is some bullshit
    macro_rules! warn_continue {
        ($result:expr, $msg:expr) => {
            {
                match $result {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("[Warn] {}", $msg);
                        eprintln!(" - {}", e);
                        continue;
                    }
                }
            }
        }
    }
    for tag_path in tagdir_paths {
        let tag_root = warn_continue!(tagfile::TagRoot::generate_from_dir(&tag_path),
            format!("Unable to interpret tag directory {:?}", tag_path));
        match &tag_root.groups {
            None => warn_continue!(evaluate_taggroup(
                &tag_root,
                &tag_root.main.handle(),
                &master_config.options,
                &scheme_registry,
                &master_config.meta_options,
            ),
            format!("Unable to apply group '{}'", tag_root.main.context)),
            Some(groups) => {
                for group in groups {
                    warn_continue!(evaluate_taggroup(
                        &tag_root,
                        &group.handle(),
                        &master_config.options,
                        &scheme_registry,
                        &master_config.meta_options,
                    ),
                    format!("Unable to apply group '{}'", group.context));
                }
            }
        }
    }
    eprintln!(" >> OK <<");
    Ok(())
}
//cannot be bothered with this function signature, might as well be a macro.
fn evaluate_taggroup<'a>(
    tag_root: &tagfile::TagRoot,
    tag_group_handle: &TableHandle,
    opt_basis: &configs::Options,
    registry: &'a configs::SchemeRegistry<'a>,
    meta_opts: &configs::MetaOptions,
) -> Result<(), MainError> {
    let mut affecting_dir = tag_root.path.clone();
    affecting_dir.pop();
    eprintln!(">> -- EVALUATING TAGGROUP :: {}", tag_group_handle.context);
    let tag_group = tagfile::TagGroup::from_table(tag_group_handle)?;
    let options = opt_basis.clone().overriden_by(tag_group.options);
    eprintln!(">> OPTIONS :: {:#?}", options);
    //cringe
    let bindings = get_bindings(
        &registry,
        &tag_group.scheme_spec,
        meta_opts,
        tag_group_handle.context.clone(),
    )?;
    eprintln!(">> BINDINGS :: {:#?}", bindings);
    for file in tag_group.files {
        let axbind_file = escaped_manip(
            options.axbind_file_format.unwrap().as_str(),
            options.escape_char.unwrap(),
            |s| s.replace(meta_opts.wildcard_char.unwrap(), file)
        );
        let axbind_file_path = affecting_dir.with_file_name(&axbind_file);
        let file_path = affecting_dir.with_file_name(file);
        eprintln!(">> AFFECTING FILE :: {:?}", file_path);
        eprintln!(">> AXBIND FILE :: {:?}", axbind_file_path);
        let axbind_contents = match std::fs::read_to_string(&axbind_file_path) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "[Warn] Error reading file {:?} (file skipped)",
                    axbind_file_path
                );
                eprintln!(" - {}", e);
                continue;
            }
        };
        //eprintln!(">> CONTENTS :: {}", axbind_contents);
        if let Err(e) = std::fs::write(
            file_path,
            axbind_replace(axbind_contents.as_str(), &bindings, &options, &meta_opts)
                .map_err(|e| MainError::ReplaceError(e))?
                .as_str(),
        ) {
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
