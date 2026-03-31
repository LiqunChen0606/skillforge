use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub hash: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    #[serde(skip)]
    file_path: PathBuf,
    skills: BTreeMap<String, RegistryEntry>,
}

impl Registry {
    pub fn new(file_path: PathBuf) -> Self {
        Self {
            file_path,
            skills: BTreeMap::new(),
        }
    }

    pub fn load(path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let mut registry: Registry = serde_json::from_str(&content)?;
        registry.file_path = path.clone();
        Ok(registry)
    }

    pub fn register(&mut self, name: &str, version: &str, hash: &str, path: &str) {
        self.skills.insert(
            name.to_string(),
            RegistryEntry {
                name: name.to_string(),
                version: version.to_string(),
                hash: hash.to_string(),
                path: path.to_string(),
            },
        );
    }

    pub fn lookup(&self, name: &str) -> Option<&RegistryEntry> {
        self.skills.get(name)
    }

    pub fn lookup_by_hash(&self, hash: &str) -> Option<&RegistryEntry> {
        self.skills.values().find(|e| e.hash == hash)
    }

    pub fn list(&self) -> Vec<&RegistryEntry> {
        self.skills.values().collect()
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(&self)?;
        std::fs::write(&self.file_path, content)?;
        Ok(())
    }

    pub fn remove(&mut self, name: &str) -> bool {
        self.skills.remove(name).is_some()
    }
}
