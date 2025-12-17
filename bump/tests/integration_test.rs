use bump::repo::Repo;
use std::fs;
use tempfile::TempDir;

fn setup_test_dir() -> TempDir {
    tempfile::tempdir().unwrap()
}

#[test]
fn test_bump_json_package_json() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a package.json
    let package_json = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "description": "A test package"
}"#;
    fs::write(temp_path.join("package.json"), package_json).unwrap();

    // Bump the version
    let repo = Repo::new(temp_path.clone()).unwrap();
    repo.bump_json("package.json", "2.0.0").unwrap();

    // Read and verify
    let content = fs::read_to_string(temp_path.join("package.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(json["version"], "2.0.0");
    assert_eq!(json["name"], "test-package");
    assert_eq!(json["description"], "A test package");
}

#[test]
fn test_bump_json_package_lock_json() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a package-lock.json with both top-level and packages[""] version
    let package_lock = r#"{
  "name": "test-package",
  "version": "1.0.0",
  "lockfileVersion": 3,
  "packages": {
    "": {
      "name": "test-package",
      "version": "1.0.0"
    }
  }
}"#;
    fs::write(temp_path.join("package-lock.json"), package_lock).unwrap();

    // Bump the version
    let repo = Repo::new(temp_path.clone()).unwrap();
    repo.bump_json("package-lock.json", "2.0.0").unwrap();

    // Read and verify
    let content = fs::read_to_string(temp_path.join("package-lock.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(json["version"], "2.0.0");
    assert_eq!(json["packages"][""]["version"], "2.0.0");
}

#[test]
fn test_bump_json_preserves_formatting() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    let package_json = r#"{
  "name": "test",
  "version": "1.0.0",
  "dependencies": {
    "foo": "1.0.0"
  }
}"#;
    fs::write(temp_path.join("package.json"), package_json).unwrap();

    let repo = Repo::new(temp_path.clone()).unwrap();
    repo.bump_json("package.json", "3.0.0").unwrap();

    let content = fs::read_to_string(temp_path.join("package.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert_eq!(json["version"], "3.0.0");
    assert_eq!(json["dependencies"]["foo"], "1.0.0");
}

#[test]
fn test_bump_toml_cargo_toml() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    // Create a Cargo.toml
    let cargo_toml = r#"[package]
name = "test-crate"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = "1.0"
"#;
    fs::write(temp_path.join("Cargo.toml"), cargo_toml).unwrap();

    // Bump the version
    let repo = Repo::new(temp_path.clone()).unwrap();
    repo.bump_toml("Cargo.toml", "0.2.0").unwrap();

    // Read and verify
    let content = fs::read_to_string(temp_path.join("Cargo.toml")).unwrap();
    let toml: toml_edit::DocumentMut = content.parse().unwrap();
    assert_eq!(toml["package"]["version"].as_str(), Some("0.2.0"));
    assert_eq!(toml["package"]["name"].as_str(), Some("test-crate"));
    assert_eq!(toml["package"]["edition"].as_str(), Some("2024"));
}

#[test]
fn test_bump_toml_preserves_structure() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    let cargo_toml = r#"[package]
name = "test"
version = "1.0.0"
authors = ["Test Author"]

[dependencies]
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
tempfile = "3.0"
"#;
    fs::write(temp_path.join("Cargo.toml"), cargo_toml).unwrap();

    let repo = Repo::new(temp_path.clone()).unwrap();
    repo.bump_toml("Cargo.toml", "1.1.0").unwrap();

    let content = fs::read_to_string(temp_path.join("Cargo.toml")).unwrap();

    assert!(content.contains(r#"version = "1.1.0""#));
    assert!(content.contains(r#"authors = ["Test Author"]"#));
    assert!(content.contains(r#"tokio = { version = "1.0", features = ["full"] }"#));
    assert!(content.contains(r#"tempfile = "3.0""#));
}

#[test]
fn test_repo_new_with_nonexistent_directory() {
    let result = Repo::new("/nonexistent/path".into());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[test]
fn test_bump_json_with_nonexistent_file() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    let repo = Repo::new(temp_path).unwrap();
    let result = repo.bump_json("nonexistent.json", "1.0.0");
    assert!(result.is_err());
}

#[test]
fn test_bump_toml_with_invalid_toml() {
    let temp_dir = setup_test_dir();
    let temp_path = temp_dir.path().to_path_buf();

    // Create an invalid TOML file
    fs::write(temp_path.join("invalid.toml"), "not valid toml [[[").unwrap();

    let repo = Repo::new(temp_path).unwrap();
    let result = repo.bump_toml("invalid.toml", "1.0.0");
    assert!(result.is_err());
}
