use crate::{TableHandle, Path, HashMap, get_array_strings};
use crate::configs::*;
pub struct SchemeSpec<'t> {
    pub scheme: &'t String,
    pub remaps: Vec<&'t String>,
    pub functions: Box<dyn Fn(&str) -> String>,
}
pub struct TagFile<'t> {
    pub files: Vec<&'t Path>,
    pub schem_spec: SchemeSpec<'t>,
    pub options: Options<'t>,
}
impl TagFile<'_> {
    pub fn generate_from<'t>(table: &'t TableHandle<'t>) -> Result<TagFile<'t>, ConfigError> {
        use gfunc::tomlutil::TableResultOptional;
        let files: Vec<&Path> = get_array_strings(table, "files")?.iter().map(|file| Path::new(file)).collect();
        let scheme_table = TableHandle {
            table: table.get_table("schemes")?.table,
            context: table.context.with("schemes".to_string()),
        };
        let option_table = TableHandle {
            table: table.get_table("options")?.table,
            context: table.context.with("options".to_string()),
        };
        let options = Options {
            keyfmt: option_table.get_string("keyfmt").optional()?,
            escapechar: {
                let raw = option_table.get_string("escapechar").optional()?;
                match raw {
                    None => None,
                    Some(s) => {
                        if s.len() != 1 {
                            return Err(ConfigError::Misc(format!(
                                "value for key 'escapechar' in {} must be exactly 1 character",
                                option_table.context)));
                        }
                        Some(s.chars().next().unwrap())
                    }
                }
            }
        };
        todo!();
    }
}
