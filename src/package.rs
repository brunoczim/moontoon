use std::{
    collections::{HashMap, hash_map},
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    Error,
    error::ErrorInner,
    source::{ModName, Module, enumerate_modules},
};

pub type PkgName = ModName;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PkgConfig {
    pub name: PkgName,
    pub version: String,
}

impl PkgConfig {
    pub fn load(path: &Path) -> Result<Self, Error> {
        let content = fs::read(path).map_err(|source| {
            ErrorInner::ReadPackageConfig { path: path.to_owned(), source }
        })?;
        let this = toml::from_slice(&content).map_err(|source| {
            ErrorInner::ParsePackageConfig { path: path.to_owned(), source }
        })?;
        Ok(this)
    }

    pub fn store(&self, path: &Path) -> Result<(), Error> {
        let content = toml::to_string_pretty(self).map_err(|source| {
            ErrorInner::EmitPackageConfig { path: path.to_owned(), source }
        })?;
        fs::write(path, content.as_bytes()).map_err(|source| {
            ErrorInner::WritePackageConfig { path: path.to_owned(), source }
        })?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Package {
    pub config: PkgConfig,
    pub modules: HashMap<ModName, Module>,
}

impl Package {
    pub fn load(root_dir: PathBuf) -> Result<Self, Error> {
        let pkg_path = root_dir.join("pkg.toml");

        let config = PkgConfig::load(&pkg_path)?;

        let mut modules = HashMap::new();
        let mut errors = Vec::new();

        let src_dir = root_dir.join("src");

        for result in enumerate_modules(src_dir, &config.name) {
            let module = match result {
                Ok(x) => x,
                Err(e) => {
                    errors.push(e);
                    continue;
                },
            };
            match modules.entry(module.name.clone()) {
                hash_map::Entry::Occupied(_) => {
                    errors.push(
                        ErrorInner::DuplicateModName {
                            module_name: module.name,
                            path: module.path,
                        }
                        .into(),
                    );
                },
                hash_map::Entry::Vacant(entry) => {
                    entry.insert(module);
                },
            }
        }

        Error::multi(errors)?;

        Ok(Self { config, modules })
    }
}
