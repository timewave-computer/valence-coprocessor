use alloc::vec::Vec;
use buf_fs::{File, FileSystem};
use valence_coprocessor_types::{DataBackend, Hasher};

use crate::{ExecutionContext, Permission};

impl<H, D> ExecutionContext<H, D>
where
    H: Hasher,
    D: DataBackend,
{
    /// Returns the controller storage.
    pub fn get_storage(&self) -> anyhow::Result<FileSystem> {
        let raw = self.get_raw_storage()?;

        match raw {
            Some(r) if !r.is_empty() => Ok(FileSystem::from_raw_device_unchecked(r)),
            _ => FileSystem::new(256 * 1024 * 1024),
        }
    }

    /// Overrides the controller storage.
    pub fn set_storage(&self, fs: &FileSystem) -> anyhow::Result<()> {
        self.ensure(&Permission::CircuitStorageWrite(*self.controller()))?;

        let fs = fs.try_to_raw_device()?;

        self.set_raw_storage(&fs)
    }

    /// Returns the controller storage file from the given path.
    pub fn get_storage_file(&self, path: &str) -> anyhow::Result<Option<Vec<u8>>> {
        let file = self.get_storage()?.open(path)?;

        Ok((!file.new).then_some(file.contents))
    }

    /// Overrides the controller storage file.
    pub fn set_storage_file(&self, path: &str, contents: &[u8]) -> anyhow::Result<()> {
        tracing::debug!("saving storage file to path `{path}`");

        self.ensure(&Permission::CircuitStorageWrite(*self.controller()))?;

        // TODO buf-fs doesn't support large extensions
        if path.split('.').nth(1).filter(|s| s.len() <= 3).is_none() {
            tracing::debug!("file path with length smaller than 3");

            #[cfg(feature = "std")]
            self.extend_log([alloc::format!("the provided file path extension `{path}` has more than 3 characters, which is not supported on FAT-16 filesystems")]).ok();
        }

        let mut fs = self.get_storage()?;

        if let Err(e) = fs.save(File::new(path.into(), contents.to_vec(), true)) {
            tracing::debug!("error saving storage file to path `{path}`: {e}");
        }

        self.set_storage(&fs)
    }

    /// Returns the controller raw storage.
    pub fn get_raw_storage(&self) -> anyhow::Result<Option<Vec<u8>>> {
        self.data
            .get_bulk(Self::PREFIX_CONTROLLER, &self.controller)
    }

    /// Overrides the controller raw storage.
    pub fn set_raw_storage(&self, storage: &[u8]) -> anyhow::Result<()> {
        self.ensure(&Permission::CircuitStorageWrite(*self.controller()))?;

        self.data
            .set_bulk(Self::PREFIX_CONTROLLER, &self.controller, storage)
            .map(|_| ())
    }
}
