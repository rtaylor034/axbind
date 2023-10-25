use crate::{Path, PathBuf, Mapping, RefMapping, escaped_manip, get_array_strings};
use optwrite::OptWrite;
use gfunc::tomlutil::*;

#[derive(Debug)]
pub enum ConfigError {
    TableGet(TableGetError),
    Misc(String),
    TableRefExpect(Context, TableGetError),
}
impl From<TableGetError> for ConfigError {
    fn from(value: TableGetError) -> Self {
        Self::TableGet(value)
    }
}
pub struct CoreConfig {
    pub scheme_dir: PathBuf,
}
pub struct BindFunction<'t> {
    shell: &'t String,
    rcommand: &'t String,
}
impl BindFunction<'_> {
    pub fn apply(&self, key: &str, metaopts: &MetaOptions) -> std::io::Result<String> {
        use std::process::Command;
        let command = escaped_manip(self.rcommand, metaopts.internal_escapechar.unwrap(), |text| {
            text.replace(metaopts.wildcard_char.unwrap(), key)
        });
        Ok(std::str::from_utf8(Command::new(self.shell)
            .arg("-c")
            .arg(&command)
            .output()?
            .stdout.as_slice())
            .expect(format!("Invalid UTF-8 returned from function command '{}'", command).as_str()).to_owned())
    }
}
//schemes should be lazy loaded
pub struct Scheme<'t> {
    pub bindings: RefMapping<'t, &'t String>,
    pub remaps: RefMapping<'t, RefMapping<'t, &'t String>>,
    pub functions: RefMapping<'t, BindFunction<'t>>,
    root_context: String,
    table: toml::Table,
    verified: bool,
}
impl<'st> Scheme<'st> {
    fn construct_unverified<'t>(table: toml::Table, root_context: String) -> Scheme<'t> {
        Scheme {
            table,
            root_context,
            verified: false,
            bindings: RefMapping::new(),
            remaps: RefMapping::new(),
            functions: RefMapping::new(),
        }
    }
}
fn validate_char(raw: &str, context: &Context) -> Result<char, ConfigError> { 
    if raw.len() != 1 {
        return Err(ConfigError::Misc(format!(
            "value for key 'escapechar' in {} must be exactly 1 character",
            context)));
    }
    Ok(raw.chars().next().unwrap())
}
#[derive(OptWrite)]
pub struct MetaOptions<'t> {
    pub internal_escapechar: Option<char>,
    pub wildcard_char: Option<char>,
    //temporary until non-primitive data type field is added.
    _p: core::marker::PhantomData<&'t toml::Table>,
}
impl MetaOptions<'_> {
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<MetaOptions<'t>, ConfigError> {
        Ok(MetaOptions {
            internal_escapechar: match table.get_string("internal_escapechar").optional()? {
                None => None,
                Some(v) => Some(validate_char(v, &table.context)?),
            },
            wildcard_char: match table.get_string("wildcard_char").optional()? {
                None => None,
                Some(v) => Some(validate_char(v, &table.context)?),
            },
            _p: std::marker::PhantomData,
        })
    }
}
#[derive(OptWrite)]
pub struct Options<'t> {
    pub key_format: Option<&'t str>,
    pub escape_char: Option<char>,
}
impl Options<'_> {
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<Options<'t>, ConfigError> {
        let o = Options {
            key_format: table.get_string("key_format").optional()?.map(|s| s.as_str()),
            escape_char: {
                let raw = table.get_string("escape_char").optional()?;
                match raw {
                    None => None,
                    Some(s) => Some(validate_char(s, &table.context)?),
                }
            }
        };
        Ok(o)
    }
}
pub struct SchemeRegistry<'t> {
    //rust warns that 'schemes' is unread becuase it is only read through raw pointers via 'lookup'
    #[allow(unused)]
    ///Must not grow after load_dir is called
    schemes: Vec<Scheme<'t>>,
    lookup: Mapping<*mut Scheme<'t>>,
}
impl<'st> SchemeRegistry<'st> {
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
    pub fn get<'s>(&'s self, name: &str) -> Result<Option<&'st Scheme>, ConfigError>
    where 's: 'st {
        unsafe {
            match self.lookup.get(name) {
                Some(ptr) => match self.verify_scheme(&mut **ptr) {
                    Ok(_) => Ok(Some(&**ptr)),
                    Err(e) => Err(e),
                },
                None => return Ok(None),
            }
        }
    }
    fn verify_scheme<'s>(&'s self, scheme: &'st mut Scheme<'st>) -> Result<(), ConfigError>
    where 's: 'st {
        if scheme.verified {
            return Ok(());
        }
        let handle = TableHandle {
            table: &scheme.table,
            context: scheme.root_context.clone().into(),
        };
        self.populate_bindmap(&mut scheme.bindings, handle.get_table("bindings")?)?;
        for (name, remaptable) in handle.get_table("remaps")?.collect_tables()? {
            let mut remap = RefMapping::<&String>::new();
            self.populate_bindmap(&mut remap, remaptable)?;
            scheme.remaps.insert(name, remap);
        }
        for (name, functiontable) in handle.get_table("functions")?.collect_tables()? {
            scheme.functions.insert(name, BindFunction {
                shell: functiontable.get_string("shell")?,
                rcommand: functiontable.get_string("command")?,
            });
        }
        Ok(())
    }
    fn populate_bindmap<'s>(&'s self, map: &mut RefMapping<'st, &'st String>, handle: TableHandle<'st>) -> Result<(), ConfigError>
    where 's: 'st {
        for (k, v) in handle.table {
            match k.as_str() {
                "@INCLUDE" => {
                    //weirdchamp as hell but not gunna rewrite get_array_strings
                     for inclusion in get_array_strings(&handle, "@INCLUDE")? {
                         let (scheme, path) = inclusion.split_once('.').unwrap_or((inclusion.as_str(), ""));
                         let scheme_table = match self.get(scheme)? {
                            Some(s) => TableHandle {
                                table: &s.table,
                                context: s.root_context.clone().into(),
                            },
                            None => return Err(ConfigError::Misc(
                                    format!("Unrecognized scheme name '{}'. ({})",
                                            scheme,
                                            handle.context.with("@INCLUDE".to_owned())))),
                         };
                         let mut nbindmap = scheme_table.get_table(&handle.context.branch).map_err(|e| {
                             ConfigError::TableRefExpect(handle.context.with("@INCLUDE".to_owned()), e)
                         })?;
                         if !path.is_empty() {
                             nbindmap = nbindmap.get_table(path).map_err(|e| {
                                ConfigError::TableRefExpect(handle.context.with("@INCLUDE".to_owned()), e)
                             })?;
                         }
                         self.populate_bindmap(map, nbindmap)?;
                     }
                }
                _ =>
                match v {
                    toml::Value::String(s) => { map.insert(k, s); },
                    _ => return Err(ConfigError::TableGet(TableGetError::new(
                            handle.context,
                            k,
                            TableGetErr::WrongType("STRING")))),
                },
            };
        }
        Ok(())
    }
    
}
