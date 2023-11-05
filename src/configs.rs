use crate::{
    escaped_manip, extract_array_strings, extract_char, extract_char_optional, Mapping, Path,
    PathBuf, RefMapping, tagfile::SchemeSpec,
};
use optwrite::OptWrite;
use toml_context::*;
//rather silly error handling for a rather silly program.
#[derive(Debug)]
pub enum ConfigError {
    TableGet(TableGetError),
    Misc(String),
    TableRefExpect(Context, TableGetError),
    SchemeExpected(Context, String),
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
            SchemeExpected(c, s) => {
                writeln!(f, "No scheme named '{}' exists", s).and_then(|_| writeln!(f, " > expected from '{}'", c))
            },
        }
    }
}
#[derive(Debug)]
pub struct MasterConfig<'t> {
    pub scheme_dir: &'t String,
    pub meta_options: MetaOptions<'t>,
    pub options: Options<'t>,
}
impl MasterConfig<'_> {
    pub fn from_table<'t>(handle: &TableHandle<'t>) -> Result<MasterConfig<'t>, ConfigError> {
        Ok(MasterConfig {
            scheme_dir: extract_value!(String, handle.get("scheme_dir"))?,
            meta_options: MetaOptions::from_table_forced(extract_value!(
                Table,
                handle.get("metaoptions")
            )?)?,
            options: Options::from_table_forced(&extract_value!(Table, handle.get("options"))?)?,
        })
    }
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
            metaopts.internal_escape_char.unwrap(),
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
        .unwrap_or_else(|_| panic!("Invalid UTF-8 returned from function command '{}'", command))
        .to_owned())
    }
}
#[derive(Debug)]
pub struct Scheme<'t> {
    pub bindings: RefMapping<'t, &'t String>,
    pub remaps: RefMapping<'t, RefMapping<'t, &'t String>>,
    pub functions: RefMapping<'t, BindFunction<'t>>,
    pub root_context: String,
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
#[derive(OptWrite, Debug)]
pub struct MetaOptions<'t> {
    pub internal_escape_char: Option<char>,
    pub wildcard_char: Option<char>,
    //temporary until non-primitive data type field is added.
    _p: core::marker::PhantomData<&'t toml::Table>,
}
impl MetaOptions<'_> {
    //perhaps make from_table a derivable trait
    pub fn from_table<'t>(table: TableHandle<'t>) -> Result<MetaOptions<'t>, ConfigError> {
        Ok(MetaOptions {
            internal_escape_char: extract_char_optional(table.get("internal_escape_char"))?,
            wildcard_char: extract_char_optional(table.get("wildcard_char"))?,
            _p: std::marker::PhantomData,
        })
    }
    //cheugy solution (from_table_forced should also be derivable)
    pub fn from_table_forced<'t>(table: TableHandle<'t>) -> Result<MetaOptions<'t>, ConfigError> {
        Ok(MetaOptions {
            internal_escape_char: Some(extract_char(table.get("internal_escape_char"))?),
            wildcard_char: Some(extract_char(table.get("wildcard_char"))?),
            _p: std::marker::PhantomData,
        })
    }
}
#[derive(Default, OptWrite, Debug, Clone)]
pub struct Options<'t> {
    pub key_format: Option<&'t String>,
    pub escape_char: Option<char>,
    pub axbind_file_format: Option<&'t String>,
}
impl Options<'_> {
    pub fn from_table<'t>(table: TableHandle<'t>) -> Result<Options<'t>, ConfigError> {
        Ok(Options {
            key_format: extract_value!(String, table.get("key_format"))
                .optional()?,
            escape_char: extract_char_optional(table.get("escape_char"))?,
            axbind_file_format: extract_value!(String, table.get("axbind_file_format")).optional()?,
        })
    }
    //silly function
    pub fn from_optional_table<'t>(opt_table: Option<TableHandle<'t>>) -> Result<Options<'t>, ConfigError> {
        match opt_table {
            Some(table) => Self::from_table(table),
            None => Ok(Options::default())
        }
    }
    pub fn from_table_forced<'t>(table: &TableHandle<'t>) -> Result<Options<'t>, ConfigError> {
        Ok(Options {
            key_format: Some(extract_value!(String, table.get("key_format"))?),
            escape_char: Some(extract_char(table.get("escape_char"))?),
            axbind_file_format: Some(extract_value!(String, table.get("axbind_file_format"))?),
        })
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
    //TODO: use TableRoot for implementation.
    pub fn load_dir(dir: &Path) -> Result<SchemeRegistry, std::io::Error> {
        use gfunc::fnav;
        use std::fs;
        use toml::Table;
        let files = fnav::rsearch_dir_pred(dir, |p| {
            p.extension().map(|os| os.to_str()) == Some(Some("toml"))
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
                None => Ok(None),
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
        self.populate_bindmap(
            "bindings",
            &mut scheme.bindings,
            extract_value!(Table, handle.get("bindings"))?,
        )?;
        if let Some(remaps) = extract_value!(Table, handle.get("remaps")).optional()? {
            for (name, remaptable) in remaps {
                let mut remap = RefMapping::<&String>::new();
                self.populate_bindmap("remaps",&mut remap, extract_value!(Table, remaptable)?)?;
                scheme.remaps.insert(name, remap);
            }
        }
        if let Some(functions) = extract_value!(Table, handle.get("functions")).optional()? {
            for (name, functiontable) in functions {
                scheme.functions.insert(
                    name,
                    BindFunction {
                        shell: extract_value!(
                            String,
                            extract_value!(Table, functiontable.clone())?.get("shell")
                        )?,
                        rcommand: extract_value!(
                            String,
                            extract_value!(Table, functiontable)?.get("command")
                        )?,
                    },
                );
            }
        }
        Ok(())
    }
    //shared_key is smelly :)
    fn populate_bindmap<'s>(
        &'s self,
        shared_key: &str,
        map: &mut RefMapping<'st, &'st String>,
        handle: TableHandle<'st>,
    ) -> Result<(), ConfigError>
    where
        's: 'st,
    {
        for (k, v) in handle.clone() {
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
                            extract_value!(Table, scheme_table.get(shared_key)).map_err(|e| {
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
                        self.populate_bindmap(shared_key, map, nbindmap)?;
                    }
                }
                _ => {
                    map.insert(k, extract_value!(String, v)?);
                }
            }
        }
        Ok(())
    }
}
