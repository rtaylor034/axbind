use aho_corasick::{AhoCorasick, PatternID};
use configs::*;
use toml_context::*;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
pub mod args;
pub mod configs;
pub mod tagfile;

pub type Mapping<T> = HashMap<String, T>;
pub type RefMapping<'t, T> = HashMap<&'t String, T>;

fn do_axbind(text: &str, bindings: BTreeMap<String, String>, options: &configs::Options) -> String {
    let searcher = AhoCorasick::new(bindings.keys()).unwrap();
    for (ti, tc) in text.char_indices() {}

    todo!();
}
pub fn extract_array_strings<'t>(
    handle: PotentialValueHandle<'t>,
) -> TableResult<Vec<&'t String>> {
    extract_value!(Array, handle)?
        .into_iter()
        .map(|v| extract_value!(String, v))
        .collect()
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
