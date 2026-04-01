use serde::{Deserialize, Serialize};

/// Simple semver: major.minor.patch
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Semver {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Semver {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.major
            .cmp(&other.major)
            .then(self.minor.cmp(&other.minor))
            .then(self.patch.cmp(&other.patch))
    }
}

/// Version constraint for dependency resolution
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VersionConstraint {
    /// Matches any version
    Any,
    /// Matches exactly this version
    Exact(Semver),
    /// Matches >= this version
    MinVersion(Semver),
    /// Matches >= min and < max
    Range { min: Semver, max: Semver },
}

impl VersionConstraint {
    /// Parse a version constraint string like "*", "=1.0.0", ">=1.0.0", ">=1.0.0+<2.0.0"
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        if s == "*" {
            return Some(VersionConstraint::Any);
        }
        if let Some(rest) = s.strip_prefix('=') {
            let v = Semver::parse(rest.trim())?;
            return Some(VersionConstraint::Exact(v));
        }
        if s.contains('+') {
            // Range: ">=1.0.0+<2.0.0"
            let parts: Vec<&str> = s.splitn(2, '+').collect();
            let min_str = parts[0].strip_prefix(">=")?;
            let max_str = parts[1].strip_prefix('<')?;
            let min = Semver::parse(min_str.trim())?;
            let max = Semver::parse(max_str.trim())?;
            // Reject inverted ranges where min >= max
            if min >= max {
                return None;
            }
            return Some(VersionConstraint::Range { min, max });
        }
        if let Some(rest) = s.strip_prefix(">=") {
            let v = Semver::parse(rest.trim())?;
            return Some(VersionConstraint::MinVersion(v));
        }
        None
    }

    /// Check if a version satisfies this constraint
    pub fn satisfies(&self, version: &Semver) -> bool {
        match self {
            VersionConstraint::Any => true,
            VersionConstraint::Exact(v) => version == v,
            VersionConstraint::MinVersion(min) => version >= min,
            VersionConstraint::Range { min, max } => version >= min && version < max,
        }
    }
}

impl std::fmt::Display for VersionConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionConstraint::Any => write!(f, "*"),
            VersionConstraint::Exact(v) => write!(f, "={}", v),
            VersionConstraint::MinVersion(v) => write!(f, ">={}", v),
            VersionConstraint::Range { min, max } => write!(f, ">={}+<{}", min, max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BumpLevel {
    Major,
    Minor,
    Patch,
}

impl Semver {
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Semver {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }

    pub fn bump(self, level: BumpLevel) -> Self {
        match level {
            BumpLevel::Major => Semver { major: self.major + 1, minor: 0, patch: 0 },
            BumpLevel::Minor => Semver { major: self.major, minor: self.minor + 1, patch: 0 },
            BumpLevel::Patch => Semver { major: self.major, minor: self.minor, patch: self.patch + 1 },
        }
    }
}

impl Default for Semver {
    fn default() -> Self {
        Semver { major: 0, minor: 1, patch: 0 }
    }
}

impl std::fmt::Display for Semver {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}
