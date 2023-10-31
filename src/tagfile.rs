use crate::configs::*;
use crate::{
    extract_array_strings, HashMap, Path, RootErr, TableGetError, TableHandle, TableResultOptional,
    extract_value,
    TableRoot,
};
use std::process;
#[derive(Debug)]
pub struct SchemeSpec<'t> {
    pub scheme: &'t String,
    pub remaps: Vec<&'t String>,
    pub functions: Vec<&'t String>,
}
#[derive(Debug)]
pub struct TagGroup<'t> {
    pub files: Vec<&'t String>,
    pub scheme_spec: SchemeSpec<'t>,
    pub options: Options<'t>,
}
#[derive(Debug)]
pub struct TagRoot {
    pub main: TableRoot,
    pub groups: Option<Vec<TableRoot>>,
}
pub enum GenerateErr {
    Root(RootErr),
    TableGet(TableGetError),
}
impl TagRoot {
    //silly ass function
    pub fn generate_from_dir<P>(path: P) -> Result<TagRoot, GenerateErr>
    where
        P: AsRef<Path>,
    {
        let main = TableRoot::from_file_path(path.as_ref().with_file_name("main.toml"))
            .map_err(|e| GenerateErr::Root(e))?;
        let group_paths = extract_array_strings(main.handle().get("groups"))
            .optional()
            .map_err(|e| GenerateErr::TableGet(e))?;
        let groups = match group_paths {
            Some(gvec) => Some(gvec.into_iter().map(|gpath| TableRoot::from_file_path(gpath)).collect::<Result<Vec<TableRoot>, RootErr>>().map_err(|e| GenerateErr::Root(e))?),
            None => None,
        };
        Ok(TagRoot {
            main,
            groups,
        })
    }
}
impl TagGroup<'_> {
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<TagGroup<'t>, ConfigError> {
        let files = extract_array_strings(table.get("files"))?;
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
        Ok(TagGroup {
            files,
            options,
            scheme_spec,
        })
    }
}
