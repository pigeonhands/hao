use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use crate::{
    error::{HaoError, Result},
    sync::Shared,
    Module,
};

pub enum AssemblyLoadResult {
    Loaded(Shared<Module>),
    Ignore,
}

pub trait AssemblyResolver: Debug {
    fn load(&mut self, assembly_name: &str) -> Result<AssemblyLoadResult>;
}

/// Resolves assemblies from a list of directories, by default the module's own
/// directory followed by the newest installed `Microsoft.NETCore.App` framework.
#[derive(Debug, Default)]
pub struct PathAssemblyResolver {
    pub search_paths: Vec<PathBuf>,
    /// Skip assemblies that can't be found instead of returning an error.
    pub ignore_missing: bool,
    loaded: Vec<(String, Shared<Module>)>,
}

impl PathAssemblyResolver {
    pub fn new(base_path: &Path) -> Self {
        let mut search_paths = vec![base_path.to_owned()];
        search_paths.extend(Self::find_framework_dir());
        search_paths.extend(Self::find_mono_dir());
        Self::with_search_paths(search_paths)
    }

    pub fn with_search_paths(search_paths: Vec<PathBuf>) -> Self {
        Self {
            search_paths,
            ignore_missing: true,
            loaded: Vec::new(),
        }
    }

    /// Resolver for a Unity game: the game's `Managed` folder, its bundled Mono
    /// class libraries, then the system Mono and .NET installs.
    pub fn for_game(game_dir: &Path) -> Self {
        let mut search_paths: Vec<PathBuf> = Self::find_game_managed_dirs(game_dir);
        search_paths.push(game_dir.to_owned());
        search_paths.push(game_dir.join("MonoBleedingEdge/lib/mono/4.5"));
        search_paths.extend(Self::find_mono_dir());
        search_paths.extend(Self::find_framework_dir());
        search_paths.retain(|p| p.is_dir());
        Self::with_search_paths(search_paths)
    }

    /// `<Game>_Data/Managed` (Windows, Linux) or `Contents/Resources/Data/Managed` (macOS).
    pub fn find_game_managed_dirs(game_dir: &Path) -> Vec<PathBuf> {
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(game_dir)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.ends_with("_Data"))
            })
            .map(|p| p.join("Managed"))
            .chain([game_dir.join("Contents/Resources/Data/Managed")])
            .filter(|p| p.is_dir())
            .collect();
        dirs.sort();
        dirs
    }

    /// The system Mono `4.5` class library directory.
    pub fn find_mono_dir() -> Option<PathBuf> {
        [
            "/usr/lib/mono/4.5",
            "/usr/local/lib/mono/4.5",
            "/Library/Frameworks/Mono.framework/Versions/Current/lib/mono/4.5",
            r"C:\Program Files\Mono\lib\mono\4.5",
        ]
        .into_iter()
        .map(PathBuf::from)
        .find(|p| p.is_dir())
    }

    /// The newest `shared/Microsoft.NETCore.App/<version>` directory.
    pub fn find_framework_dir() -> Option<PathBuf> {
        let roots = std::env::var_os("DOTNET_ROOT")
            .map(PathBuf::from)
            .into_iter()
            .chain(
                [
                    "/usr/share/dotnet",
                    "/usr/lib/dotnet",
                    "/usr/local/share/dotnet",
                    "/opt/dotnet",
                    r"C:\Program Files\dotnet",
                ]
                .map(PathBuf::from),
            );

        roots
            .map(|root| root.join("shared").join("Microsoft.NETCore.App"))
            .filter_map(|dir| std::fs::read_dir(dir).ok())
            .flat_map(|entries| entries.filter_map(|e| e.ok()).map(|e| e.path()))
            .filter(|p| p.is_dir())
            .max_by_key(|p| parse_version(p))
    }

    fn find_path_for(&self, assembly_name: &str) -> Option<PathBuf> {
        self.search_paths
            .iter()
            .flat_map(|dir| {
                ["dll", "exe"].map(|ext| dir.join(format!("{}.{}", assembly_name, ext)))
            })
            .find(|p| p.exists())
    }
}

fn parse_version(path: &Path) -> Vec<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| {
            n.split(['.', '-'])
                .map(|p| p.parse().unwrap_or(0))
                .collect()
        })
        .unwrap_or_default()
}

impl AssemblyResolver for PathAssemblyResolver {
    fn load(&mut self, assembly_name: &str) -> Result<AssemblyLoadResult> {
        if let Some((_, module)) = self.loaded.iter().find(|(name, _)| name == assembly_name) {
            return Ok(AssemblyLoadResult::Loaded(module.clone()));
        }

        match self.find_path_for(assembly_name) {
            Some(path) => {
                let module = Shared::new(Module::from_path_no_resolve(path)?);
                self.loaded.push((assembly_name.into(), module.clone()));
                module.load_dependancies(self)?;
                Ok(AssemblyLoadResult::Loaded(module))
            }
            None if self.ignore_missing => Ok(AssemblyLoadResult::Ignore),
            None => Err(HaoError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Could not resolve assembly {:?}", assembly_name),
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_unity_managed_folder() {
        let game = std::env::temp_dir().join(format!("hao-game-{}", std::process::id()));
        let managed = game.join("MyGame_Data/Managed");
        std::fs::create_dir_all(&managed).unwrap();
        std::fs::write(managed.join("Assembly-CSharp.dll"), b"").unwrap();

        let resolver = PathAssemblyResolver::for_game(&game);
        assert_eq!(resolver.search_paths[0], managed);
        assert_eq!(
            resolver.find_path_for("Assembly-CSharp"),
            Some(managed.join("Assembly-CSharp.dll"))
        );
        std::fs::remove_dir_all(game).unwrap();
    }
}
