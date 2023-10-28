use crate::{
    escaped_manip,
    Mapping,
    Path,
    PathBuf,
    RefMapping,
    extract_array_strings,
};
use toml_context::*;
use optwrite::OptWrite;
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
impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ConfigError::*;
        match self {
            TableGet(e) => e.fmt(f),
            TableRefExpect(c, e) => {
                writeln!(f, "'{}'", e).and_then(|_| writeln!(f, " > expected from '{}'", c))
            }
            Misc(msg) => writeln!(f, "{}", msg),
        }
    }
}
pub struct CoreConfig {
    pub scheme_dir: PathBuf,
}
#[derive(Debug)]
pub struct BindFunction<'t> {
    shell: &'t String,
    rcommand: &'t String,
}
impl BindFunction<'_> {
    pub fn apply(&self, key: &str, metaopts: &MetaOptions) -> std::io::Result<String> {
        use std::process::Command;
        let command = escaped_manip(
            self.rcommand,
            metaopts.internal_escapechar.unwrap(),
            |text| text.replace(metaopts.wildcard_char.unwrap(), key),
        );
        Ok(std::str::from_utf8(
            Command::new(self.shell)
                .arg("-c")
                .arg(&command)
                .output()?
                .stdout
                .as_slice(),
        )
        .expect(format!("Invalid UTF-8 returned from function command '{}'", command).as_str())
        .to_owned())
    }
}
#[derive(Debug)]
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
//perhaps make validate <t> a feature of tomlutil
//also make a get_char
fn validate_char(raw: &str, context: &Context) -> Result<char, ConfigError> {
    if raw.len() != 1 {
        return Err(ConfigError::Misc(format!(
            "value for '{}' must be exactly 1 character",
            context
        )));
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
    //perhaps make from_table a derivable trait
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<MetaOptions<'t>, ConfigError> {
        Ok(MetaOptions {
            internal_escapechar: match extract_value!(String, table.get("internal_escapechar")).optional()? {
                None => None,
                Some(v) => Some(validate_char(v, &table.context)?),
            },
            wildcard_char: match extract_value!(String, table.get("wildcard_char")).optional()? {
                None => None,
                Some(v) => Some(validate_char(v, &table.context)?),
            },
            _p: std::marker::PhantomData,
        })
    }
    //cheugy solution (from_table_forced should also be derivable)
    pub fn from_table_forced<'t>(table: &TableHandle<'t>) -> Result<MetaOptions<'t>, ConfigError> {
        Ok(MetaOptions {
            internal_escapechar: Some(validate_char(
                extract_value!(String, table.get("internal_escapechar"))?,
                &table.context.with("internal_escapechar".to_owned()),
            )?),
            wildcard_char: Some(validate_char(
                extract_value!(String, table.get("wildcard_char"))?,
                &table.context.with("wildcard_char".to_owned()),
            )?),
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
            key_format: extract_value!(String, table.get("key_format"))
                .optional()?
                .map(|s| s.as_str()),
            escape_char: {
                let raw = extract_value!(String, table.get("escape_char")).optional()?;
                match raw {
                    None => None,
                    Some(s) => Some(validate_char(s, &table.context)?),
                }
            },
        };
        Ok(o)
    }
}
#[derive(Debug)]
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
                        "[Warn] 'axbind_scheme' key in {:?} is not a String type, file skipped.",
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
    where
        's: 'st,
    {
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
    where
        's: 'st,
    {
        if scheme.verified {
            return Ok(());
        }
        let handle = TableHandle {
            table: &scheme.table,
            context: scheme.root_context.clone().into(),
        };
        self.populate_bindmap(&mut scheme.bindings, extract_value!(Table, handle.get("bindings"))?)?;
        for (name, remaptable) in extract_value!(Table, handle.get("remaps"))? {
            let mut remap = RefMapping::<&String>::new();
            self.populate_bindmap(&mut remap, extract_value!(Table, remaptable)?)?;
            scheme.remaps.insert(&remaptable.context.branch, remap);
        }
        for (name, functiontable) in extract_value!(Table, handle.get("functions"))? {
            scheme.functions.insert(
                name,
                BindFunction {
                    shell: extract_value!(String, extract_value!(Table, functiontable)?.get("shell"))?,
                    rcommand: extract_value!(String, extract_value!(Table, functiontable)?.get("command"))?,
                },
            );
        }
        Ok(())
    }
    fn populate_bindmap<'s>(
        &'s self,
        map: &mut RefMapping<'st, &'st String>,
        handle: TableHandle<'st>,
    ) -> Result<(), ConfigError>
    where
        's: 'st,
    {
        for (k, v) in handle {
            match k.as_str() {
                "@INCLUDE" => {
                    for inclusion in extract_array_strings(v.into())? {
                        let (scheme, path) = inclusion
                            .split_once('.')
                            .unwrap_or((inclusion.as_str(), ""));
                        let scheme_table = match self.get(scheme)? {
                            Some(s) => TableHandle {
                                table: &s.table,
                                context: s.root_context.clone().into(),
                            },
                            None => {
                                return Err(ConfigError::Misc(format!(
                                    "Unrecognized scheme name '{}'. ({})",
                                    scheme,
                                    handle.context.with("@INCLUDE".to_owned())
                                )))
                            }
                        };
                        let mut nbindmap =
                            extract_value!(Table, scheme_table.get(&handle.context.branch))
                                .map_err(|e| {
                                    ConfigError::TableRefExpect(
                                        handle.context.with("@INCLUDE".to_owned()),
                                        e,
                                    )
                                })?;
                        if !path.is_empty() {
                            nbindmap = extract_value!(Table, nbindmap.get(path)).map_err(|e| {
                                ConfigError::TableRefExpect(
                                    handle.context.with("@INCLUDE".to_owned()),
                                    e,
                                )
                            })?;
                        }
                        self.populate_bindmap(map, nbindmap)?;
                    }
                },
                _ => { map.insert(k, extract_value!(String, v)?); },
            }
        }
        Ok(())
    }
}
