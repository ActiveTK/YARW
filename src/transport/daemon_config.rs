use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct DaemonConfig {
    pub address: String,
    pub port: u16,
    #[serde(flatten)]
    pub modules: HashMap<String, ModuleConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModuleConfig {
    pub path: PathBuf,
    #[serde(default)]
    pub read_only: bool,
    pub auth_users: Option<Vec<String>>,
    pub secrets_file: Option<PathBuf>,
}
