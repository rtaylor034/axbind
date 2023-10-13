use aho_corasick::{AhoCorasick, PatternID};
use configs::*;
use gfunc::tomlutil::TableHandle;
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
pub mod args;
pub mod configs;
pub mod tagfile;

pub type Mapping<T> = HashMap<String, T>;

pub fn remap<F>(original: &mut Mapping<String>, remap_function: F)
where
    F: Fn(&mut String),
{
    for (_, v) in original.iter_mut() {
        remap_function(v);
    }
}
fn do_axbind(text: &str, bindings: BTreeMap<String, String>, options: &configs::Options) -> String {
    let searcher = AhoCorasick::new(bindings.keys()).unwrap();
    for (ti, tc) in text.char_indices() {}

    todo!();
}
pub fn get_array_strings<'t>(tag_entry: &TableHandle<'t>, key: &str) -> gfunc::tomlutil::TableResult<Vec<&'t String>> {
    use gfunc::tomlutil::*;
    use toml::Value;
    let mut o = Vec::<&String>::new();
        for val in tag_entry.get_array(key)? {
            match val {
                Value::String(str) => o.push(str),
                _ => {
                    return Err(TableGetError::new(tag_entry.context.with(key.to_string()), key, TableGetErr::WrongType("ARRAY (of STRINGs}")));
                }
            }
        }
    Ok(o)
}

