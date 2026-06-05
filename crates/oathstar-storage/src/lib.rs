use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::Context;
use serde::{de::DeserializeOwned, Serialize};

pub trait SaveStore {
    fn write_json<T: Serialize>(&self, name: &str, value: &T) -> anyhow::Result<()>;
    fn read_json<T: DeserializeOwned>(&self, name: &str) -> anyhow::Result<T>;
}

#[derive(Debug, Clone)]
pub struct FileSaveStore {
    root: PathBuf,
}

impl FileSaveStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn path_for(&self, name: &str) -> PathBuf {
        self.root.join(format!("{name}.json"))
    }
}

impl SaveStore for FileSaveStore {
    fn write_json<T: Serialize>(&self, name: &str, value: &T) -> anyhow::Result<()> {
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create save directory {}", self.root.display()))?;
        let path = self.path_for(name);
        let json = serde_json::to_string_pretty(value).context("failed to serialize save JSON")?;
        fs::write(&path, json).with_context(|| format!("failed to write {}", path.display()))?;
        Ok(())
    }

    fn read_json<T: DeserializeOwned>(&self, name: &str) -> anyhow::Result<T> {
        let path = self.path_for(name);
        let json = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&json).with_context(|| format!("failed to parse {}", path.display()))
    }
}
