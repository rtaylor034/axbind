use aho_corasick::{AhoCorasick, PatternID};
use configs::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use toml_context::*;
pub mod args;
pub mod configs;
pub mod tagfile;

pub type Mapping<T> = HashMap<String, T>;
pub type RefMapping<'t, T> = HashMap<&'t String, T>;

pub fn remapping<'t, S: AsRef<str>, F>(original: &RefMapping<'t, S>, remap: F) -> RefMapping<'t, String> 
where F: Fn(&str) -> String {
    //just fucking inline it lol!
    HashMap::from_iter(original.iter().map(|(k, v)| (*k, remap(v.as_ref()))))
}
pub fn axbind_replace<S: AsRef<str>>(text: &str, bindings: RefMapping<S>, options: &configs::Options) -> String {
    let searcher = AhoCorasick::new(bindings.keys()).unwrap();
    for (ti, tc) in text.char_indices() {}

    todo!();
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
