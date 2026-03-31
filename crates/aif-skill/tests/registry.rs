use aif_skill::registry::Registry;
use tempfile::NamedTempFile;

#[test]
fn register_and_lookup() {
    let tmp = NamedTempFile::new().unwrap();
    let mut reg = Registry::new(tmp.path().to_path_buf());
    reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");

    let entry = reg.lookup("debugging").unwrap();
    assert_eq!(entry.name, "debugging");
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.hash, "abc123");
    assert_eq!(entry.path, "/skills/debugging.aif");

    assert!(reg.lookup("nonexistent").is_none());
}

#[test]
fn list_all_skills() {
    let tmp = NamedTempFile::new().unwrap();
    let mut reg = Registry::new(tmp.path().to_path_buf());
    reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");
    reg.register("refactoring", "2.0.0", "def456", "/skills/refactoring.aif");

    let all = reg.list();
    assert_eq!(all.len(), 2);
    let names: Vec<&str> = all.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"debugging"));
    assert!(names.contains(&"refactoring"));
}

#[test]
fn save_and_load() {
    let tmp = NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();

    {
        let mut reg = Registry::new(path.clone());
        reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");
        reg.register("refactoring", "2.0.0", "def456", "/skills/refactoring.aif");
        reg.save().unwrap();
    }

    let loaded = Registry::load(&path).unwrap();
    assert_eq!(loaded.list().len(), 2);
    let entry = loaded.lookup("debugging").unwrap();
    assert_eq!(entry.version, "1.0.0");
    assert_eq!(entry.hash, "abc123");
}

#[test]
fn update_existing_skill() {
    let tmp = NamedTempFile::new().unwrap();
    let mut reg = Registry::new(tmp.path().to_path_buf());
    reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");
    reg.register("debugging", "2.0.0", "xyz789", "/skills/debugging_v2.aif");

    let entry = reg.lookup("debugging").unwrap();
    assert_eq!(entry.version, "2.0.0");
    assert_eq!(entry.hash, "xyz789");
    assert_eq!(entry.path, "/skills/debugging_v2.aif");
    assert_eq!(reg.list().len(), 1);
}

#[test]
fn lookup_by_hash() {
    let tmp = NamedTempFile::new().unwrap();
    let mut reg = Registry::new(tmp.path().to_path_buf());
    reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");

    let entry = reg.lookup_by_hash("abc123").unwrap();
    assert_eq!(entry.name, "debugging");

    assert!(reg.lookup_by_hash("nonexistent").is_none());
}

#[test]
fn remove() {
    let tmp = NamedTempFile::new().unwrap();
    let mut reg = Registry::new(tmp.path().to_path_buf());
    reg.register("debugging", "1.0.0", "abc123", "/skills/debugging.aif");

    assert!(reg.remove("debugging"));
    assert!(reg.lookup("debugging").is_none());
    assert!(!reg.remove("debugging"));
    assert_eq!(reg.list().len(), 0);
}
