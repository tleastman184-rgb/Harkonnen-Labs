use std::path::PathBuf;

use crate::setup::SetupConfig;

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
    pub factory: PathBuf,
    pub specs: PathBuf,
    pub scenarios: PathBuf,
    pub artifacts: PathBuf,
    pub logs: PathBuf,
    pub workspaces: PathBuf,
    pub memory: PathBuf,
    pub db_file: PathBuf,
    pub products: PathBuf,
    pub setup: SetupConfig,
}

impl Paths {
    pub fn discover() -> anyhow::Result<Self> {
        let root = std::env::current_dir()?;
        let factory = root.join("factory");
        let setup = SetupConfig::discover(&root)?;
        Ok(Self {
            specs: factory.join("specs"),
            scenarios: factory.join("scenarios"),
            artifacts: factory.join("artifacts"),
            logs: factory.join("logs"),
            workspaces: factory.join("workspaces"),
            memory: factory.join("memory"),
            db_file: factory.join("state.db"),
            products: root.join("products"),
            factory,
            root,
            setup,
        })
    }
}
