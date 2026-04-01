use crate::chain::ChainError;
use crate::registry::Registry;
use crate::remote::{RemoteError, RemoteRegistry};
use crate::version::{Semver, VersionConstraint};
use std::path::PathBuf;

/// A resolved skill with its metadata and source
#[derive(Debug)]
pub struct ResolvedSkill {
    pub name: String,
    pub version: Semver,
    pub path: PathBuf,
    pub source: ResolveSource,
}

/// Where a skill was resolved from
#[derive(Debug, PartialEq)]
pub enum ResolveSource {
    Local,
    Cache,
    Remote,
}

/// Unified resolver that checks local registry first, then remote
pub struct SkillResolver {
    pub local: Registry,
    pub remote: Option<RemoteRegistry>,
    pub cache_dir: PathBuf,
}

impl SkillResolver {
    pub fn new(local: Registry, remote: Option<RemoteRegistry>) -> Self {
        let cache_dir = dirs_or_default();
        Self {
            local,
            remote,
            cache_dir,
        }
    }

    /// Resolve a skill by name and version constraint.
    /// Checks local registry first, then remote.
    pub fn resolve(
        &self,
        name: &str,
        constraint: &VersionConstraint,
    ) -> Result<ResolvedSkill, ChainError> {
        // Try local first
        if let Some(entry) = self.local.lookup(name) {
            let version = Semver::parse(&entry.version).unwrap_or_default();
            if constraint.satisfies(&version) {
                return Ok(ResolvedSkill {
                    name: name.to_string(),
                    version,
                    path: PathBuf::from(&entry.path),
                    source: ResolveSource::Local,
                });
            }
        }

        // Try cache
        if let Some(cached) = self.find_in_cache(name, constraint) {
            return Ok(cached);
        }

        // Try remote (if configured)
        if let Some(remote) = &self.remote {
            match remote.fetch_metadata(name, None) {
                Ok(entry) => {
                    let version = Semver::parse(&entry.version).unwrap_or_default();
                    if constraint.satisfies(&version) {
                        // Would download and cache here
                        return Err(ChainError::SkillNotFound(format!(
                            "{} (available remotely at v{}, but download not yet implemented)",
                            name, version
                        )));
                    }
                }
                Err(RemoteError::NotConfigured) => {}
                Err(_) => {}
            }
        }

        Err(ChainError::SkillNotFound(name.to_string()))
    }

    /// Look for a cached version of the skill
    fn find_in_cache(
        &self,
        name: &str,
        constraint: &VersionConstraint,
    ) -> Option<ResolvedSkill> {
        let skill_cache_dir = self.cache_dir.join("skills").join(name);
        if !skill_cache_dir.exists() {
            return None;
        }

        let mut best: Option<(Semver, PathBuf)> = None;

        if let Ok(entries) = std::fs::read_dir(&skill_cache_dir) {
            for entry in entries.flatten() {
                let filename = entry.file_name();
                let name_str = filename.to_string_lossy();
                if let Some(version_str) = name_str.strip_suffix(".aif") {
                    if let Some(version) = Semver::parse(version_str) {
                        if constraint.satisfies(&version) {
                            if best.as_ref().map_or(true, |(best_v, _)| version > *best_v) {
                                best = Some((version, entry.path()));
                            }
                        }
                    }
                }
            }
        }

        best.map(|(version, path)| ResolvedSkill {
            name: name.to_string(),
            version,
            path,
            source: ResolveSource::Cache,
        })
    }

    /// Install a skill from remote to local cache
    pub fn install(
        &mut self,
        name: &str,
        version: &str,
    ) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let remote = self
            .remote
            .as_ref()
            .ok_or_else(|| RemoteError::NotConfigured)?;

        let data = remote.download(name, version)?;

        // Write to cache
        let cache_path = self
            .cache_dir
            .join("skills")
            .join(name);
        std::fs::create_dir_all(&cache_path)?;
        let file_path = cache_path.join(format!("{}.aif", version));
        std::fs::write(&file_path, &data)?;

        // Register in local registry
        let hash = format!("sha256:{}", hex_digest(&data));
        self.local
            .register(name, version, &hash, file_path.to_str().unwrap_or(""));

        Ok(file_path)
    }
}

fn dirs_or_default() -> PathBuf {
    if let Some(home) = home_dir() {
        home.join(".aif").join("cache")
    } else {
        PathBuf::from("/tmp/aif/cache")
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .map(PathBuf::from)
}

fn hex_digest(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    result.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn resolver_finds_local_skill() {
        let mut registry = Registry::new(PathBuf::from("/tmp/test_registry.json"));
        registry.register("tdd", "1.0.0", "sha256:abc", "/skills/tdd.aif");

        let resolver = SkillResolver::new(registry, None);
        let result = resolver.resolve("tdd", &VersionConstraint::Any);
        assert!(result.is_ok());
        let skill = result.unwrap();
        assert_eq!(skill.name, "tdd");
        assert_eq!(skill.source, ResolveSource::Local);
    }

    #[test]
    fn resolver_version_mismatch_fails() {
        let mut registry = Registry::new(PathBuf::from("/tmp/test_registry.json"));
        registry.register("tdd", "0.5.0", "sha256:abc", "/skills/tdd.aif");

        let resolver = SkillResolver::new(registry, None);
        let constraint = VersionConstraint::MinVersion(Semver { major: 1, minor: 0, patch: 0 });
        let result = resolver.resolve("tdd", &constraint);
        assert!(result.is_err());
    }

    #[test]
    fn resolver_not_found() {
        let registry = Registry::new(PathBuf::from("/tmp/test_registry.json"));
        let resolver = SkillResolver::new(registry, None);
        let result = resolver.resolve("nonexistent", &VersionConstraint::Any);
        assert!(result.is_err());
    }

    #[test]
    fn resolve_source_equality() {
        assert_eq!(ResolveSource::Local, ResolveSource::Local);
        assert_ne!(ResolveSource::Local, ResolveSource::Remote);
    }

    #[test]
    fn cache_dir_default() {
        let dir = dirs_or_default();
        assert!(dir.to_string_lossy().contains("aif"));
    }
}
