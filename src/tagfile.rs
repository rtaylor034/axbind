use crate::configs::*;
use crate::{HashMap, Path, TableHandle};
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
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<TagFile<'t>, ConfigError> {
        use crate::{extract_array_strings, extract_value, TableResultOptional};
        let files: Vec<&Path> = extract_array_strings(table.get("files"))?
            .into_iter()
            .map(Path::new)
            .collect();
        let scheme_table = extract_value!(Table, table.get("schemes"))?;
        let options = Options::from_table(&extract_value!(Table, table.get("options"))?)?;
        let remaps = extract_array_strings(scheme_table.get("remaps"))
            .optional()?
            .unwrap_or(vec![]);
        let functions = extract_array_strings(scheme_table.get("functions"))
            .optional()?
            .unwrap_or(vec![]);
        let scheme = extract_value!(String, scheme_table.get("name"))?;
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
