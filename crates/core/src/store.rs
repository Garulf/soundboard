use crate::model::Library;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const LIBRARY_FILE: &str = "library.toml";
const SOUNDS_DIR: &str = "sounds";

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("could not serialize library: {0}")]
    Serialize(#[from] toml::ser::Error),
}

fn io_err(path: &Path) -> impl FnOnce(io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

pub struct LoadOutcome {
    pub library: Library,
    pub recovered_from: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct LibraryStore {
    root: PathBuf,
}

impl LibraryStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn sounds_dir(&self) -> PathBuf {
        self.root.join(SOUNDS_DIR)
    }

    pub fn sound_path(&self, file: &str) -> PathBuf {
        self.sounds_dir().join(file)
    }

    fn library_path(&self) -> PathBuf {
        self.root.join(LIBRARY_FILE)
    }

    pub fn load(&self) -> Result<LoadOutcome, StoreError> {
        let path = self.library_path();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                return Ok(LoadOutcome {
                    library: Library::new_default(),
                    recovered_from: None,
                });
            }
            Err(e) => return Err(io_err(&path)(e)),
        };
        match toml::from_str::<Library>(&text) {
            Ok(library) if !library.tabs.is_empty() => Ok(LoadOutcome {
                library,
                recovered_from: None,
            }),
            Ok(_) => Ok(LoadOutcome {
                library: Library::new_default(),
                recovered_from: Some(self.set_aside(&path)?),
            }),
            Err(e) => {
                tracing::warn!("library file is corrupt: {e}");
                Ok(LoadOutcome {
                    library: Library::new_default(),
                    recovered_from: Some(self.set_aside(&path)?),
                })
            }
        }
    }

    fn set_aside(&self, path: &Path) -> Result<PathBuf, StoreError> {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let broken = self.root.join(format!("{LIBRARY_FILE}.broken-{stamp}"));
        fs::rename(path, &broken).map_err(io_err(path))?;
        Ok(broken)
    }

    pub fn save(&self, library: &Library) -> Result<(), StoreError> {
        fs::create_dir_all(&self.root).map_err(io_err(&self.root))?;
        let text = toml::to_string_pretty(library)?;
        let path = self.library_path();
        let tmp = self.root.join(format!("{LIBRARY_FILE}.tmp"));
        fs::write(&tmp, text).map_err(io_err(&tmp))?;
        fs::rename(&tmp, &path).map_err(io_err(&path))
    }

    /// Copies `source` into the managed sounds folder and returns the stored
    /// file name. Identical content maps to the same name, so re-imports
    /// share one file.
    pub fn import_file(&self, source: &Path) -> Result<String, StoreError> {
        let bytes = fs::read(source).map_err(io_err(source))?;
        let hash = blake3::hash(&bytes).to_hex();
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_else(|| "bin".into());
        let name = format!("{}.{ext}", &hash[..32]);
        let dir = self.sounds_dir();
        fs::create_dir_all(&dir).map_err(io_err(&dir))?;
        let dest = dir.join(&name);
        if !dest.exists() {
            let tmp = dir.join(format!("{name}.tmp"));
            fs::write(&tmp, &bytes).map_err(io_err(&tmp))?;
            fs::rename(&tmp, &dest).map_err(io_err(&dest))?;
        }
        Ok(name)
    }
}

#[cfg(test)]
#[path = "store_tests.rs"]
mod tests;
