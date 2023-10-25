use crate::configs::*;
use crate::{get_array_strings, HashMap, Path, TableHandle};
use std::process;
pub struct SchemeSpec<'t> {
    pub scheme: &'t String,
    pub remaps: Vec<&'t String>,
    pub functions: Vec<&'t String>,
}
pub struct TagFile<'t> {
    pub files: Vec<&'t Path>,
    pub scheme_spec: SchemeSpec<'t>,
    pub options: Options<'t>,
}
impl TagFile<'_> {
    pub fn generate_from<'t>(table: &TableHandle<'t>) -> Result<TagFile<'t>, ConfigError> {
        use gfunc::tomlutil::TableResultOptional;
        let files: Vec<&Path> = get_array_strings(table, "files")?
            .into_iter()
            .map(|file| Path::new(file))
            .collect();
        let scheme_table = TableHandle {
            table: table.get_table("schemes")?.table,
            context: table.context.with("schemes".to_string()),
        };
        let options = Options::from_table(&TableHandle {
            table: table.get_table("options")?.table,
            context: table.context.with("options".to_string()),
        })?;
        let remaps = get_array_strings(&scheme_table, "remaps")
            .optional()?
            .unwrap_or(vec![]);
        let functions = get_array_strings(&scheme_table, "functions")
            .optional()?
            .unwrap_or(vec![]);
        let scheme = scheme_table.get_string("name")?;
        let scheme_spec = SchemeSpec {
            scheme,
            remaps,
            functions,
        };
        Ok(TagFile {
            files,
            options,
            scheme_spec,
        })
    }
}
