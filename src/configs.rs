use crate::{Path, PathBuf, Mapping, RefMapping};
use gfunc::tomlutil::*;

#[derive(Debug)]
pub enum ConfigError {
    TableGet(TableGetError),
    Misc(String),
}
impl From<TableGetError> for ConfigError {
    fn from(value: TableGetError) -> Self {
        Self::TableGet(value)
    }
}
pub struct CoreConfig {
    pub scheme_dir: PathBuf,
}
//schemes should be lazy loaded
pub struct Scheme<'t> {
    pub bindings: RefMapping<'t, &'t String>,
    pub remaps: RefMapping<'t, &'t String>,
    pub functions: RefMapping<'t, Box<dyn Fn(String) -> String>>,
    root_context: Context,
    table: toml::Table,
    verified: bool,
}
impl<'st> Scheme<'st> {
    fn construct_unverified<'t>(table: toml::Table, root_context: String) -> Scheme<'t> {
        Scheme {
            table,
            root_context: Context::from(root_context),
            verified: false,
            bindings: RefMapping::new(),
            remaps: RefMapping::new(),
            functions: RefMapping::new(),
        }
    }
    fn verify(&'st mut self) -> Result<(), ConfigError> {
        if self.verified {
            return Ok(());
        }
        let handle = TableHandle {
            table: &self.table,
            context: self.root_context.clone(),
        };
        Self::populate_bindmap(&mut self.bindings, handle.get_table("bindings")?)?;
        Self::populate_bindmap(&mut self.remaps, handle.get_table("remaps")?)?;
        todo!();
    }
    fn populate_bindmap<'t>(map: &mut RefMapping<'t, &'t String>, handle: TableHandle<'t>) -> Result<(), ConfigError> {
        for (k, v) in handle.table {
            match v {
                toml::Value::String(s) => map.insert(k, s),
                _ => return Err(ConfigError::TableGet(TableGetError::new(
                        handle.context,
                        k,
                        TableGetErr::WrongType("STRING")))),
            };
        }
        Ok(())
    }

}
pub struct Options<'t> {
    pub keyfmt: Option<&'t String>,
    pub escapechar: Option<char>,
}
impl Options<'_> {
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<Options<'t>, ConfigError> {
        let o = Options {
            keyfmt: table.get_string("keyfmt").optional()?,
            escapechar: {
                let raw = table.get_string("escapechar").optional()?;
                match raw {
                    None => None,
                    Some(s) => {
                        if s.len() != 1 {
                            return Err(ConfigError::Misc(format!(
                                "value for key 'escapechar' in {} must be exactly 1 character",
                                table.context)));
                        }
                        Some(s.chars().next().unwrap())
                    }
                }
            }
        };
        Ok(o)
    }
}
pub struct SchemeRegistry<'t> {
    //Must not grow after load_dir is called
    schemes: Vec<Scheme<'t>>,
    lookup: Mapping<*mut Scheme<'t>>,
}
impl<'t> SchemeRegistry<'t> {
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
            schemes.push(Scheme::construct_unverified(table, context_string));
            lookup.insert(keyname, schemes.last_mut().unwrap() as *mut Scheme);
        }
        Ok(SchemeRegistry { schemes, lookup })
    }
    ///self.schemes MUST not grow.
    pub fn get<'s>(&'s self, name: &str) -> Result<Option<&'s Scheme>, ConfigError> {
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
