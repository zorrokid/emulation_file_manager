use std::{
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

/// Introducing FsOps trait so that MockFsOps can be implemented and used in tests.
pub trait FsOps {
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64>;
    fn remove_file(&self, path: &Path) -> io::Result<()>;
}

#[derive(Default)]
pub struct StdFsOps;

impl FsOps for StdFsOps {
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        std::fs::rename(from, to)
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        std::fs::copy(from, to)
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
}

#[derive(Default)]
pub struct FsOpsOutcome {
    pub create_dir_all_result: Option<io::Result<()>>,
    pub rename_result: Option<io::Result<()>>,
    pub copy_result: Option<io::Result<u64>>,
    pub remove_result: Option<io::Result<()>>,
}

pub enum FsOpsCall {
    CreateDir { path: PathBuf },
    Rename { from: PathBuf, to: PathBuf },
    Copy { from: PathBuf, to: PathBuf },
    Remove { path: PathBuf },
}

#[derive(Default)]
pub struct MockFsOpsState {
    pub outcome: FsOpsOutcome,
    pub calls: Vec<FsOpsCall>,
}

pub struct MockFsOps {
    state: Arc<Mutex<MockFsOpsState>>,
}

impl MockFsOps {
    pub fn new(state: Arc<Mutex<MockFsOpsState>>) -> Self {
        Self { state }
    }
}

impl FsOps for MockFsOps {
    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut guard = self.state.lock().unwrap();
        guard.calls.push(FsOpsCall::CreateDir {
            path: PathBuf::from(path),
        });
        guard
            .outcome
            .create_dir_all_result
            .take()
            .expect("No mock outcome defined for create_dir_all")
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut guard = self.state.lock().unwrap();
        guard.calls.push(FsOpsCall::Rename {
            from: PathBuf::from(from),
            to: PathBuf::from(to),
        });
        guard
            .outcome
            .rename_result
            .take()
            .expect("No mock outcome defined for rename")
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<u64> {
        let mut guard = self.state.lock().unwrap();
        guard.calls.push(FsOpsCall::Copy {
            from: PathBuf::from(from),
            to: PathBuf::from(to),
        });
        guard
            .outcome
            .copy_result
            .take()
            .expect("No mock outcome defined for copy")
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut guard = self.state.lock().unwrap();
        guard.calls.push(FsOpsCall::Remove {
            path: PathBuf::from(path),
        });
        guard
            .outcome
            .remove_result
            .take()
            .expect("No mock outcome defined for remove_file")
    }
}
