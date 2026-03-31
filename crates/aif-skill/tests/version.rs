use aif_skill::version::{Semver, BumpLevel};

#[test]
fn parse_semver() {
    let v = Semver::parse("1.2.3").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 2);
    assert_eq!(v.patch, 3);
}

#[test]
fn parse_invalid() {
    assert!(Semver::parse("not-a-version").is_none());
    assert!(Semver::parse("1.2").is_none());
}

#[test]
fn bump_major() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Major);
    assert_eq!(bumped.to_string(), "2.0.0");
}

#[test]
fn bump_minor() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Minor);
    assert_eq!(bumped.to_string(), "1.3.0");
}

#[test]
fn bump_patch() {
    let v = Semver::parse("1.2.3").unwrap();
    let bumped = v.bump(BumpLevel::Patch);
    assert_eq!(bumped.to_string(), "1.2.4");
}

#[test]
fn default_version() {
    assert_eq!(Semver::default().to_string(), "0.1.0");
}
