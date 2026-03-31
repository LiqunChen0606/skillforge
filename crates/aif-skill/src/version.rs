/// Simple semver: major.minor.patch
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Semver {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
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
