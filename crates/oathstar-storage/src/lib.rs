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

    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::{FileSaveStore, SaveStore};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Hero {
        name: String,
        level: u32,
    }

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("oathstar-store-test-{tag}"))
    }

    #[test]
    fn write_then_read_round_trips() {
        let dir = scratch_dir("round-trip");
        std::fs::remove_dir_all(&dir).ok();
        let store = FileSaveStore::new(&dir);
        let original = Hero {
            name: "Aria".to_string(),
            level: 7,
        };
        store
            .write_json("hero", &original)
            .expect("write should succeed");
        // the bytes actually land at root/<name>.json
        assert!(dir.join("hero.json").exists(), "save file written");
        let loaded: Hero = store.read_json("hero").expect("read should succeed");
        assert_eq!(loaded, original);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_missing_is_err() {
        let store = FileSaveStore::new(scratch_dir("missing"));
        let result: anyhow::Result<Hero> = store.read_json("does-not-exist");
        assert!(result.is_err(), "reading an absent save is an error");
    }

    #[test]
    fn root_returns_configured_path() {
        let store = FileSaveStore::new(scratch_dir("root"));
        assert_eq!(store.root(), scratch_dir("root").as_path());
    }

    #[test]
    fn write_fails_when_dir_cannot_be_created() {
        // Seed a regular FILE, then point the store root *under* it so
        // create_dir_all must fail (a file cannot have child directories).
        let file = scratch_dir("as-file");
        std::fs::remove_dir_all(&file).ok();
        std::fs::remove_file(&file).ok();
        std::fs::write(&file, b"i am a file").expect("seed a regular file");
        let store = FileSaveStore::new(file.join("nested"));
        let err = store
            .write_json(
                "hero",
                &Hero {
                    name: "x".to_string(),
                    level: 1,
                },
            )
            .expect_err("create_dir under a regular file must fail");
        assert!(
            err.to_string().contains("failed to create save directory"),
            "unexpected error: {err}"
        );
        std::fs::remove_file(&file).ok();
    }

    #[test]
    fn write_fails_when_target_path_is_a_dir() {
        // Pre-create root/hero.json AS A DIRECTORY so fs::write must fail.
        let dir = scratch_dir("target-is-dir");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("make root");
        std::fs::create_dir(dir.join("hero.json")).expect("occupy the target path with a dir");
        let store = FileSaveStore::new(&dir);
        let err = store
            .write_json(
                "hero",
                &Hero {
                    name: "x".to_string(),
                    level: 1,
                },
            )
            .expect_err("writing onto a directory path must fail");
        assert!(
            err.to_string().contains("failed to write"),
            "unexpected error: {err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn read_fails_on_corrupt_json() {
        let dir = scratch_dir("corrupt");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("make root");
        std::fs::write(dir.join("hero.json"), b"{ this is not json").expect("seed corrupt file");
        let store = FileSaveStore::new(&dir);
        let result: anyhow::Result<Hero> = store.read_json("hero");
        let err = result.expect_err("corrupt JSON must fail to parse");
        assert!(
            err.to_string().contains("failed to parse"),
            "unexpected error: {err}"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
