use crate::{Path, PathBuf, Mapping};
use gfunc::tomlutil::*;

#[derive(Debug)]
pub enum ConfigError<'c> {
    TableGet(TableGetError<'c>),
    Misc(String),
}
impl<'c> From<TableGetError<'c>> for ConfigError<'c> {
    fn from(value: TableGetError<'c>) -> Self {
        Self::TableGet(value)
    }
}
pub struct CoreConfig {
    pub scheme_dir: PathBuf,
}
//schemes should be lazy loaded
pub struct Scheme<'t, 'c> {
    pub name: &'t String,
    pub bindings: Mapping<&'t String>,
    pub remaps: Mapping<Mapping<&'t String>>,
    pub functions: Mapping<Box<dyn Fn(String) -> String>>,
    table_handle: TableHandle<'t, 'c>,
    table: toml::Table,
    verified: bool,
}
impl Scheme<'_, '_> {
    fn construct_unverified<'t, 'c>(table: toml::Table, context: Context<'c>) -> Scheme<'t, 'c> {
        todo!();
    }
    fn verify(&mut self) -> Result<(), ConfigError> {
        if self.verified {
            return Ok(());
        }
        todo!();
    }
}
pub struct Options<'t> {
    pub keyfmt: Option<&'t String>,
    pub escapechar: Option<char>,
}
pub struct SchemeRegistry<'t, 'c> {
    //Must not grow after load_dir is called
    schemes: Vec<Scheme<'t, 'c>>,
    lookup: Mapping<*mut Scheme<'t, 'c>>,
}
impl<'t, 'c> SchemeRegistry<'t, 'c> {
    pub fn load_dir<E>(dir: &Path) -> Result<SchemeRegistry, std::io::Error> {
        use gfunc::fnav;
        use std::fs;
        use toml::Table;
        let files = fnav::rsearch_dir_pred(dir, |p| {
            p.extension().map(|os| os.to_str()) == Some(Some(".toml"))
        })?;
        let mut schemes = Vec::<Scheme>::with_capacity(files.len());
        let mut lookup = Mapping::<*mut Scheme>::with_capacity(files.len());
        for file in &files {
            let content = match fs::read_to_string(file) {
                Ok(content) => content,
                Err(e) => {
                    eprintln!(
                        "[Warn] Error reading from file {:?} in scheme directory, file skipped.",
                        file
                    );
                    eprintln!(" - {}", e);
                    continue;
                }
            };
            let table = match content.parse::<Table>() {
                Ok(t) => t,
                Err(e) => {
                    eprintln!(
                        "[Warn] Error parsing toml from {:?} in scheme directory, file skipped.",
                        file
                    );
                    eprintln!(" - {}", e);
                    continue;
                }
            };
            let name = match table.get("axbind_scheme") {
                Some(toml::Value::String(t)) => t,
                None => {
                    eprintln!(
                        "[Info] No 'axbind_scheme' key present in {:?}, file skipped.",
                        file
                    );
                    continue;
                }
                _ => {
                    eprintln!(
                        "[Warn] 'axbind_scheme' key in {:?} is not a STRING type, file skipped.",
                        file
                    );
                    continue;
                }
            };
            let keyname = name.to_string();
            let context_string: String = dir.join(&keyname).to_string_lossy().into();
            schemes.push(Scheme::construct_unverified(table, context_string.into()));
            lookup.insert(keyname, schemes.last_mut().unwrap() as *mut Scheme);
        }
        Ok(SchemeRegistry { schemes, lookup })
    }
    ///self.schemes MUST not grow.
    pub fn get<'s>(&'s self, name: &str) -> Result<Option<&'s Scheme>, ConfigError<'s>> {
        unsafe {
            match self.lookup.get(name) {
                Some(ptr) => match (**ptr).verify() {
                    Ok(_) => Ok(Some(&**ptr)),
                    Err(e) => Err(e),
                },
                None => return Ok(None),
            }
        }
    }
}
