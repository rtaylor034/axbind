use crate::configs::*;
use crate::{
    extract_array_strings, HashMap, Path, RootErr, TableGetError, TableHandle, TableResultOptional,
    extract_value,
    TableRoot,
    PathBuf,
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
    pub path: PathBuf,
    pub main: TableRoot,
    pub groups: Option<Vec<TableRoot>>,
}
#[derive(Debug)]
pub enum GenerateErr {
    Root(RootErr),
    TableGet(TableGetError),
    FilesAndGroupExist(PathBuf),
}
impl std::fmt::Display for GenerateErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Root(e) => e.fmt(f),
            Self::TableGet(e) => e.fmt(f),
            Self::FilesAndGroupExist(path) => write!(f, "'main' tagfiles must have either a 'files' or 'groups' key, not both. ({:?})", path),
        }
    }
}
impl TagRoot {
    //silly ass function
    pub fn generate_from_dir(mut path: PathBuf) -> Result<TagRoot, GenerateErr> {
        path.push("main.toml");
        let main = TableRoot::from_file_path(&path)
            .map_err(|e| GenerateErr::Root(e))?;
        let group_paths = extract_array_strings(main.handle().get("groups"))
            .optional()
            .map_err(|e| GenerateErr::TableGet(e))?;
        let groups = match group_paths {
            Some(gvec) => Some(gvec.into_iter().map(|gpath| TableRoot::from_file_path(path.with_file_name(gpath))).collect::<Result<Vec<TableRoot>, RootErr>>().map_err(|e| GenerateErr::Root(e))?),
            None => None,
        };
        if main.table.get("files").is_some() && groups.is_some() {
            return Err(GenerateErr::FilesAndGroupExist(path.with_file_name("main.toml")));
        }
        Ok(TagRoot {
            main,
            groups,
            path,
        })
    }
}
impl TagGroup<'_> {
    pub fn from_table<'t>(table: &TableHandle<'t>) -> Result<TagGroup<'t>, ConfigError> {
        let files = extract_array_strings(table.get("files"))?;
        let scheme_table = extract_value!(Table, table.get("scheme"))?;
        let options = match extract_value!(Table, table.get("options")).optional()? {
            Some(option_table) => Options::from_table(option_table)?,
            None => Options::default(),
        };
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
