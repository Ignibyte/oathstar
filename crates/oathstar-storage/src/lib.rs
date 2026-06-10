use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use serde::{de::DeserializeOwned, Serialize};

/// Why a save slot id was rejected before it could become a filesystem path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveSlotError {
    /// The slot id was empty.
    Empty,
    /// The slot id contained a path separator (`/` or `\`).
    ContainsSeparator { name: String },
    /// The slot id attempted parent-directory traversal (contained `..`).
    Traversal { name: String },
    /// The slot id contained a character outside the allowed set.
    DisallowedCharacter { name: String, character: char },
    /// The slot id was longer than [`MAX_SAVE_SLOT_LEN`].
    TooLong { name: String, max: usize },
    /// The slot id matched a reserved device name (e.g. Windows `CON`/`NUL`).
    ReservedName { name: String },
}

impl std::fmt::Display for SaveSlotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "save slot id must not be empty"),
            Self::ContainsSeparator { name } => {
                write!(f, "save slot id '{name}' must not contain a path separator")
            }
            Self::Traversal { name } => write!(
                f,
                "save slot id '{name}' must not contain parent-directory traversal"
            ),
            Self::DisallowedCharacter { name, character } => write!(
                f,
                "save slot id '{name}' contains disallowed character '{character}'"
            ),
            Self::TooLong { name, max } => write!(
                f,
                "save slot id '{name}' is longer than the {max}-character maximum"
            ),
            Self::ReservedName { name } => {
                write!(f, "save slot id '{name}' is a reserved device name")
            }
        }
    }
}

impl std::error::Error for SaveSlotError {}

/// The longest permitted save slot id (kept well under filesystem name limits).
pub const MAX_SAVE_SLOT_LEN: usize = 64;

/// Validate a caller-supplied save slot id before it is used to build a path.
///
/// A slot id is valid iff it is non-empty and every character is ASCII
/// alphanumeric, `-`, or `_`. Path separators and parent-directory traversal
/// are rejected with their own variants so callers can distinguish them. This is
/// the single boundary save/load call sites should use instead of ad-hoc checks.
///
/// # Errors
/// Returns a [`SaveSlotError`] describing the first violation found.
pub fn validate_save_slot_name(name: &str) -> Result<(), SaveSlotError> {
    if name.is_empty() {
        return Err(SaveSlotError::Empty);
    }
    if name.len() > MAX_SAVE_SLOT_LEN {
        return Err(SaveSlotError::TooLong {
            name: name.to_owned(),
            max: MAX_SAVE_SLOT_LEN,
        });
    }
    if name.contains('/') || name.contains('\\') {
        return Err(SaveSlotError::ContainsSeparator {
            name: name.to_owned(),
        });
    }
    if name.contains("..") {
        return Err(SaveSlotError::Traversal {
            name: name.to_owned(),
        });
    }
    for character in name.chars() {
        if !(character.is_ascii_alphanumeric() || character == '-' || character == '_') {
            return Err(SaveSlotError::DisallowedCharacter {
                name: name.to_owned(),
                character,
            });
        }
    }
    if is_reserved_device_name(name) {
        return Err(SaveSlotError::ReservedName {
            name: name.to_owned(),
        });
    }
    Ok(())
}

/// Whether `name` matches a Windows reserved device name (case-insensitive):
/// `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, `LPT1`-`LPT9`. Rejected on every
/// platform so save files stay portable.
fn is_reserved_device_name(name: &str) -> bool {
    let upper = name.to_ascii_uppercase();
    if matches!(upper.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }
    for prefix in ["COM", "LPT"] {
        if let Some(rest) = upper.strip_prefix(prefix) {
            if rest.len() == 1 && matches!(rest.chars().next(), Some('1'..='9')) {
                return true;
            }
        }
    }
    false
}

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

    fn path_for(&self, name: &str) -> Result<PathBuf, SaveSlotError> {
        validate_save_slot_name(name)?;
        Ok(self.root.join(format!("{name}.json")))
    }

    /// Defense-in-depth: refuse a save target that is a symlink, so a planted
    /// symlink in the save directory cannot redirect a read or write outside the
    /// save root. (Full TOCTOU safety would need `O_NOFOLLOW`/`openat`.)
    fn ensure_not_symlink(&self, path: &Path) -> anyhow::Result<()> {
        if let Ok(meta) = fs::symlink_metadata(path) {
            if meta.file_type().is_symlink() {
                bail!("refusing save slot: '{}' is a symlink", path.display());
            }
        }
        Ok(())
    }
}

impl SaveStore for FileSaveStore {
    fn write_json<T: Serialize>(&self, name: &str, value: &T) -> anyhow::Result<()> {
        let path = self.path_for(name)?;
        fs::create_dir_all(&self.root)
            .with_context(|| format!("failed to create save directory {}", self.root.display()))?;
        self.ensure_not_symlink(&path)?;
        let json = serde_json::to_string_pretty(value).context("failed to serialize save JSON")?;
        // Write a temp sibling, then rename over the slot: the replacement is
        // atomic within the directory, so a crash or a concurrent save can
        // leave a stale temp file but never torn JSON at the slot itself.
        let tmp = self.root.join(format!("{name}.json.tmp"));
        self.ensure_not_symlink(&tmp)?;
        fs::write(&tmp, json).with_context(|| format!("failed to write {}", tmp.display()))?;
        if let Err(error) = fs::rename(&tmp, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(error).with_context(|| format!("failed to write {}", path.display()));
        }
        Ok(())
    }

    fn read_json<T: DeserializeOwned>(&self, name: &str) -> anyhow::Result<T> {
        let path = self.path_for(name)?;
        self.ensure_not_symlink(&path)?;
        let json = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&json).with_context(|| format!("failed to parse {}", path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::{validate_save_slot_name, FileSaveStore, SaveSlotError, SaveStore};
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
        // ticket #28 (atomic write): the temp sibling was renamed into place.
        assert!(
            !dir.join("hero.json.tmp").exists(),
            "no temp file remains after a successful write"
        );
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
        // ticket #28 (atomic write): the failed rename cleans up its temp
        // sibling instead of littering the save root.
        assert!(
            !dir.join("hero.json.tmp").exists(),
            "the temp file is removed after a failed rename"
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

    // REQ-001: a name of only allowed characters (alphanumeric, '-', '_') validates.
    #[test]
    fn validate_accepts_allowed_characters() {
        assert_eq!(validate_save_slot_name("valid-name_01"), Ok(()));
    }

    // REQ-005: an empty name is rejected.
    #[test]
    fn validate_rejects_empty() {
        assert_eq!(validate_save_slot_name(""), Err(SaveSlotError::Empty));
    }

    // REQ-002: both path separators are rejected with the separator variant.
    #[test]
    fn validate_rejects_path_separators() {
        assert_eq!(
            validate_save_slot_name("a/b"),
            Err(SaveSlotError::ContainsSeparator {
                name: "a/b".to_string()
            })
        );
        assert_eq!(
            validate_save_slot_name("a\\b"),
            Err(SaveSlotError::ContainsSeparator {
                name: "a\\b".to_string()
            })
        );
    }

    // REQ-003: parent-directory traversal is rejected with the traversal variant.
    #[test]
    fn validate_rejects_parent_traversal() {
        assert_eq!(
            validate_save_slot_name(".."),
            Err(SaveSlotError::Traversal {
                name: "..".to_string()
            })
        );
        assert_eq!(
            validate_save_slot_name("a..b"),
            Err(SaveSlotError::Traversal {
                name: "a..b".to_string()
            })
        );
    }

    // REQ-005: a disallowed character is rejected, naming the offending character.
    #[test]
    fn validate_rejects_disallowed_character() {
        assert_eq!(
            validate_save_slot_name("a b"),
            Err(SaveSlotError::DisallowedCharacter {
                name: "a b".to_string(),
                character: ' ',
            })
        );
    }

    // Display covers all four variants (and names the offender).
    #[test]
    fn save_slot_error_messages_render() {
        assert!(SaveSlotError::Empty
            .to_string()
            .contains("must not be empty"));

        let separator = SaveSlotError::ContainsSeparator {
            name: "x".to_string(),
        }
        .to_string();
        assert!(separator.contains("'x'") && separator.contains("path separator"));

        let traversal = SaveSlotError::Traversal {
            name: "x".to_string(),
        }
        .to_string();
        assert!(traversal.contains("'x'") && traversal.contains("parent-directory traversal"));

        let disallowed = SaveSlotError::DisallowedCharacter {
            name: "x".to_string(),
            character: '!',
        }
        .to_string();
        assert!(disallowed.contains("'x'"));
        assert!(disallowed.contains("disallowed character"));
        assert!(disallowed.contains('!'));
    }

    // REQ-002/003 enforcement: write_json rejects a disallowed name BEFORE any fs side effect.
    #[test]
    fn write_json_rejects_disallowed_name() {
        let dir = scratch_dir("reject-write");
        std::fs::remove_dir_all(&dir).ok();
        let store = FileSaveStore::new(&dir);
        let err = store
            .write_json(
                "../escape",
                &Hero {
                    name: "x".to_string(),
                    level: 1,
                },
            )
            .expect_err("a disallowed slot name must be rejected");
        assert!(
            err.to_string().contains("save slot id"),
            "unexpected error: {err}"
        );
        assert!(
            !dir.exists(),
            "no save directory is created for a rejected name"
        );
    }

    // Enforcement: read_json rejects a disallowed name too.
    #[test]
    fn read_json_rejects_disallowed_name() {
        let store = FileSaveStore::new(scratch_dir("reject-read"));
        let result: anyhow::Result<Hero> = store.read_json("../escape");
        let err = result.expect_err("a disallowed slot name must be rejected");
        assert!(
            err.to_string().contains("save slot id"),
            "unexpected error: {err}"
        );
    }

    // Hardening: length cap (boundary at MAX_SAVE_SLOT_LEN).
    #[test]
    fn validate_rejects_too_long() {
        let max = super::MAX_SAVE_SLOT_LEN;
        assert_eq!(validate_save_slot_name(&"a".repeat(max)), Ok(()));
        let too_long = "a".repeat(max + 1);
        assert_eq!(
            validate_save_slot_name(&too_long),
            Err(SaveSlotError::TooLong {
                name: too_long.clone(),
                max,
            })
        );
    }

    // Hardening: Windows reserved device names (case-insensitive) are rejected.
    #[test]
    fn validate_rejects_reserved_names() {
        for reserved in [
            "con", "CON", "Nul", "aux", "prn", "COM1", "com9", "lpt1", "LPT9",
        ] {
            assert_eq!(
                validate_save_slot_name(reserved),
                Err(SaveSlotError::ReservedName {
                    name: reserved.to_string(),
                }),
                "{reserved} should be rejected as reserved"
            );
        }
    }

    // Hardening: near-misses are NOT reserved (exactness, not substring).
    #[test]
    fn validate_allows_reserved_lookalikes() {
        for ok in ["console", "com0", "com10", "lpt", "connection"] {
            assert_eq!(
                validate_save_slot_name(ok),
                Ok(()),
                "{ok} should be allowed"
            );
        }
    }

    // Hardening: Display for the two new variants.
    #[test]
    fn hardening_error_messages_render() {
        let too_long = SaveSlotError::TooLong {
            name: "x".to_string(),
            max: 64,
        }
        .to_string();
        assert!(too_long.contains("'x'") && too_long.contains("64") && too_long.contains("longer"));
        let reserved = SaveSlotError::ReservedName {
            name: "CON".to_string(),
        }
        .to_string();
        assert!(reserved.contains("'CON'") && reserved.contains("reserved device name"));
    }

    // Hardening: writing through a planted symlink is refused (no escape).
    #[cfg(unix)]
    #[test]
    fn write_json_rejects_symlink_target() {
        use std::os::unix::fs::symlink;
        let dir = scratch_dir("symlink-write");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("make root");
        let outside = scratch_dir("symlink-write-outside");
        std::fs::remove_file(&outside).ok();
        symlink(&outside, dir.join("hero.json")).expect("plant a symlink at the save path");
        let store = FileSaveStore::new(&dir);
        let err = store
            .write_json(
                "hero",
                &Hero {
                    name: "x".to_string(),
                    level: 1,
                },
            )
            .expect_err("writing through a symlink must be refused");
        assert!(
            err.to_string().contains("symlink"),
            "unexpected error: {err}"
        );
        assert!(
            !outside.exists(),
            "the symlink target outside the root was not written"
        );
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_file(&outside).ok();
    }

    // Hardening: reading through a planted symlink is refused.
    #[cfg(unix)]
    #[test]
    fn read_json_rejects_symlink_target() {
        use std::os::unix::fs::symlink;
        let dir = scratch_dir("symlink-read");
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).expect("make root");
        let outside = scratch_dir("symlink-read-outside");
        std::fs::write(&outside, b"{}").expect("seed an outside file");
        symlink(&outside, dir.join("hero.json")).expect("plant a symlink at the save path");
        let store = FileSaveStore::new(&dir);
        let result: anyhow::Result<Hero> = store.read_json("hero");
        let err = result.expect_err("reading through a symlink must be refused");
        assert!(
            err.to_string().contains("symlink"),
            "unexpected error: {err}"
        );
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_file(&outside).ok();
    }
}
