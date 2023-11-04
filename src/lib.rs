use aho_corasick::{AhoCorasick, PatternID};
use configs::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use toml_context::*;
pub mod args;
pub mod configs;
pub mod tagfile;

pub enum MainError {
    NoConfigFileFound(Vec<PathBuf>),
    InvalidRootDir(PathBuf, std::io::Error),
    SchemeExpected(String, Context),
    FunctionError(Context, String, std::io::Error),
    ConfigError(configs::ConfigError),
    ReplaceError(Box<dyn std::error::Error>),
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
            },
            SchemeExpected(scheme, context) => {
                writeln!(f, "No scheme with name '{}' exists", scheme)?;
                writeln!(f, " > expected from '{}'", context)
            },
            FunctionError(context, key, error) => {
                writeln!(f, "Error while applying bind function '{}' on key '{}'", context, key)?;
                writeln!(f, " - {}", error)
            },
            ReplaceError(e) => e.fmt(f),
            Generic(e) => e.fmt(f),
            _ => unreachable!(),
        }
    }
}
pub type Mapping<T> = HashMap<String, T>;
pub type RefMapping<'t, T> = HashMap<&'t String, T>;

pub fn axbind_replace<S: AsRef<str>>(text: &str, bindings: &RefMapping<S>, options: &configs::Options) -> Result<String, Box<dyn std::error::Error>> {
    let pairs: Vec<(&str, &str)> = bindings.into_iter().map(|(k, v)| ((k).as_str(), v.as_ref())).collect();
    let searcher = AhoCorasick::new(pairs.iter().map(|(k, _)| *k))?;
    //weirdchamp collect then slice 
    Ok(searcher.replace_all(text, pairs.into_iter().map(|(_, v)| v).collect::<Vec<&str>>().as_slice()))
}
//this entire function may be a codesmell
pub fn get_bindings<'t>(registry: &'t SchemeRegistry<'t>, scheme_spec: &tagfile::SchemeSpec<'t>, meta_opts: &MetaOptions, spec_context: Context) -> Result<RefMapping<'t, String>, MainError> {
    let scheme = registry.get(scheme_spec.scheme)?.ok_or(ConfigError::SchemeExpected(spec_context.with("scheme".to_owned()), scheme_spec.scheme.to_owned()))?;
    macro_rules! gen_error {
        ($category:expr, $key:expr) => { ConfigError::TableRefExpect(spec_context.with($category.to_owned()).with(($key).to_owned()),
        TableGetError {
            error: TableGetErr::NoKey,
            context: Context::from(scheme.root_context.clone()).with($category.to_owned()).with(($key).to_owned()),
                })
            }
        }
    let mut inter_o = scheme.bindings.clone();
    for remap_name in &scheme_spec.remaps {
        let s_remaps = scheme.remaps.get(remap_name).ok_or(gen_error!("remaps", *remap_name))?;
        for val in inter_o.values_mut() {
            if let Some(remap) = s_remaps.get(val) {
                *val = *remap;
            }
        }
    }
    let mut o = RefMapping::<String>::from_iter(inter_o.into_iter().map(|(k, v)| (k, v.to_owned())));
    for function_name in &scheme_spec.functions {
        let s_function = scheme.functions.get(function_name).ok_or(gen_error!("functions", *function_name))?;
        for val in o.values_mut() {
            *val = s_function.apply(val.as_str(), meta_opts).map_err(|e| MainError::FunctionError(
                    spec_context.with("functions".to_owned()).with((*function_name).to_owned()),
                    val.to_owned(),
                    e))?;
        }
    }
    Ok(o)
}
pub fn extract_array_strings<'t>(handle: PotentialValueHandle<'t>) -> TableResult<Vec<&'t String>> {
    extract_value!(Array, handle)?
        .into_iter()
        .map(|v| extract_value!(String, v))
        .collect()
}
pub fn extract_char(handle: PotentialValueHandle) -> Result<char, ConfigError> {
    let raw = extract_value!(String, handle.clone())?.as_str();
    match raw.len() == 1 {
        true => Ok(raw.chars().next().unwrap()),
        false => Err(ConfigError::Misc(format!(
            "value for '{}' must be exactly 1 character",
            handle.context
        ))),
    }
}
pub fn extract_char_optional(handle: PotentialValueHandle) -> Result<Option<char>, ConfigError> {
    let rawopt = extract_value!(String, handle.clone()).optional()?;
    match rawopt {
        None => Ok(None),
        Some(raw) => Ok(match raw.len() == 1 {
            true => Some(raw.chars().next().unwrap()),
            false => {
                return Err(ConfigError::Misc(format!(
                    "value for '{}' must be exactly 1 character",
                    handle.context
                )))
            }
        }),
    }
}
pub fn escaped_manip<'s, F>(text: &'s str, escape: char, manip: F) -> String
where
    F: Fn(&'s str) -> String,
{
    let mut o = String::with_capacity(text.len());
    for (esc, string) in text.split(escape).map(|chunk| chunk.split_at(1)) {
        o.push_str(esc);
        o.push_str(manip(string).as_str());
    }
    o
}
