use std::path::Path;
use std::{fmt::Debug, path::PathBuf, rc::Rc};

use crate::error::HaoError;
use crate::{error::Result, Module};

// Hard code for testing
const TRUSTED_ASSEMBLY_LOCATION: &str =
    r#"C:\Program Files\dotnet\shared\Microsoft.NETCore.App\6.0.16\"#;

pub enum AssemblyLoadResult {
    Loaded(Rc<Module>),
    Ignore,
}

pub trait AssemblyResolver: Debug {
    fn load(&mut self, assembly_name: &str) -> Result<AssemblyLoadResult>;
}

#[derive(Debug, Default)]
pub struct PathAssemblyResolver {
    base_path: std::path::PathBuf,
    pub(crate) assembly_list: Vec<(String, Rc<Module>)>,
}

impl PathAssemblyResolver {
    pub fn new(path: &std::path::Path) -> Self {
        let base_path = path.to_owned();
        Self {
            base_path,
            assembly_list: Vec::new(),
        }
    }
}

impl PathAssemblyResolver {
    fn find_path_for(&self, assembly_name: &str) -> Option<PathBuf> {
        const KNOWN_EXTENTIONS: [&str; 2] = ["dll", "CoreLib.dll"];
        let asm_locations = [Path::new(TRUSTED_ASSEMBLY_LOCATION), &self.base_path];

        for asm_root_path in asm_locations.into_iter() {
            let mut path = asm_root_path.join(format!("{}.ext", assembly_name));

            for ext in KNOWN_EXTENTIONS {
                path.set_extension(ext);
                if path.exists() {
                    return Some(path);
                }
            }
        }
        None
    }
}

impl AssemblyResolver for PathAssemblyResolver {
    fn load(&mut self, assembly_name: &str) -> Result<AssemblyLoadResult> {
        let cached_module = self
            .assembly_list
            .iter()
            .filter(|(saved_asm, _)| saved_asm == assembly_name)
            .map(|asm| asm.1.clone())
            .next();

        if let Some(asm) = cached_module {
            return Ok(AssemblyLoadResult::Loaded(asm));
        }

        if let Some(found_path) = self.find_path_for(assembly_name) {
            let loaded_asm = Rc::new(Module::from_path_no_resolve(found_path)?);
            self.assembly_list
                .push((assembly_name.into(), loaded_asm.clone()));
            loaded_asm.load_dependancies(self)?;
            Ok(AssemblyLoadResult::Loaded(loaded_asm))
        } else {
            Err(HaoError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not resolve assembly {:?}", assembly_name),
            )))
        }
    }
}
