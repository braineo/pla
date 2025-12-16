use clap::ValueEnum;
use semver::Version;
use serde::{Deserialize, Serialize};

pub trait BumpVersion {
    /// Increments the major version number.
    fn increment_major(&self) -> Self;
    /// Increments the minor version number.
    fn increment_minor(&self) -> Self;
    /// Increments the patch version number.
    fn increment_patch(&self) -> Self;
    /// Increments the prerelease version number.
    fn increment_prerelease(&self) -> Self;
    /// Add identifiers to version for prerelease
    fn append_prerelease_identifiers(&self, identifiers: &str) -> Self;
    /// Remove prerelease from version
    fn convert_prerelease_to_release(&self) -> Self;
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, ValueEnum)]
pub enum BumpType {
    /// Bump major version.
    Major,
    /// Bump minor version.
    Minor,
    /// Bump patch version.
    Patch,
    /// Bump major version for pre-release.
    PreMajor,
    /// Bump minor version for pre-release.
    PreMinor,
    /// Bump patch version for pre-release.
    PrePatch,
    /// Increase prerelease version.
    Prerelease,
    /// Remove prerelease suffix from version.
    Release,
}

impl BumpVersion for Version {
    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L38
    fn increment_major(&self) -> Self {
        Self {
            major: self.major + 1,
            minor: 0,
            patch: 0,
            pre: semver::Prerelease::EMPTY,
            build: self.build.clone(),
        }
    }

    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L46
    fn increment_minor(&self) -> Self {
        Self {
            minor: self.minor + 1,
            patch: 0,
            pre: semver::Prerelease::EMPTY,
            ..self.clone()
        }
    }

    // taken from https://github.com/killercup/cargo-edit/blob/643e9253a84db02c52a7fa94f07d786d281362ab/src/version.rs#L53
    fn increment_patch(&self) -> Self {
        Self {
            patch: self.patch + 1,
            pre: semver::Prerelease::EMPTY,
            ..self.clone()
        }
    }

    fn increment_prerelease(&self) -> Self {
        let next_pre = increment_last_identifier(self.pre.as_str());
        let next_pre = semver::Prerelease::new(&next_pre).expect("pre release increment failed.");
        Self {
            pre: next_pre,
            ..self.clone()
        }
    }

    fn append_prerelease_identifiers(&self, identifiers: &str) -> Self {
        let next_pre = semver::Prerelease::new(identifiers).expect("pre release increment failed.");
        Self {
            pre: next_pre,
            ..self.clone()
        }
    }

    fn convert_prerelease_to_release(&self) -> Self {
        Self {
            pre: semver::Prerelease::EMPTY,
            ..self.clone()
        }
    }
}

fn increment_last_identifier(release: &str) -> String {
    if let Ok(release_number) = release.parse::<u32>() {
        return (release_number + 1).to_string();
    }
    match release.rsplit_once('.') {
        Some((left, right)) => {
            if let Ok(right_num) = right.parse::<u32>() {
                format!("{left}.{}", right_num + 1)
            } else {
                format!("{release}.1")
            }
        }
        None => format!("{release}.1"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_increment_major() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.increment_major();
        assert_eq!(bumped, Version::parse("2.0.0").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_major_with_prerelease() {
        let version = Version::parse("1.2.3-beta.1").unwrap();
        let bumped = version.increment_major();
        assert_eq!(bumped, Version::parse("2.0.0").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_minor() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.increment_minor();
        assert_eq!(bumped, Version::parse("1.3.0").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_minor_with_prerelease() {
        let version = Version::parse("1.2.3-alpha.1").unwrap();
        let bumped = version.increment_minor();
        assert_eq!(bumped, Version::parse("1.3.0").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_patch() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.increment_patch();
        assert_eq!(bumped, Version::parse("1.2.4").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_patch_with_prerelease() {
        let version = Version::parse("1.2.3-rc.1").unwrap();
        let bumped = version.increment_patch();
        assert_eq!(bumped, Version::parse("1.2.4").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_prerelease_numeric() {
        let version = Version::parse("1.2.3-0").unwrap();
        let bumped = version.increment_prerelease();
        assert_eq!(bumped, Version::parse("1.2.3-1").unwrap());
    }

    #[test]
    fn test_increment_prerelease_with_dot() {
        let version = Version::parse("1.2.3-beta.0").unwrap();
        let bumped = version.increment_prerelease();
        assert_eq!(bumped, Version::parse("1.2.3-beta.1").unwrap());
    }

    #[test]
    fn test_increment_prerelease_multiple_dots() {
        let version = Version::parse("1.2.3-alpha.0.5").unwrap();
        let bumped = version.increment_prerelease();
        assert_eq!(bumped, Version::parse("1.2.3-alpha.0.6").unwrap());
    }

    #[test]
    fn test_increment_prerelease_non_numeric() {
        let version = Version::parse("1.2.3-beta").unwrap();
        let bumped = version.increment_prerelease();
        assert_eq!(bumped, Version::parse("1.2.3-beta.1").unwrap());
    }

    #[test]
    fn test_append_prerelease_identifiers() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.append_prerelease_identifiers("beta.0");
        assert_eq!(bumped, Version::parse("1.2.3-beta.0").unwrap());
    }

    #[test]
    fn test_append_prerelease_identifiers_replaces_existing() {
        let version = Version::parse("1.2.3-alpha.1").unwrap();
        let bumped = version.append_prerelease_identifiers("beta.0");
        assert_eq!(bumped, Version::parse("1.2.3-beta.0").unwrap());
    }

    #[test]
    fn test_convert_prerelease_to_release() {
        let version = Version::parse("1.2.3-beta.5").unwrap();
        let bumped = version.convert_prerelease_to_release();
        assert_eq!(bumped, Version::parse("1.2.3").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_convert_prerelease_to_release_no_prerelease() {
        let version = Version::parse("1.2.3").unwrap();
        let bumped = version.convert_prerelease_to_release();
        assert_eq!(bumped, Version::parse("1.2.3").unwrap());
        assert!(bumped.pre.is_empty());
    }

    #[test]
    fn test_increment_last_identifier_numeric() {
        assert_eq!(increment_last_identifier("0"), "1");
        assert_eq!(increment_last_identifier("5"), "6");
        assert_eq!(increment_last_identifier("99"), "100");
    }

    #[test]
    fn test_increment_last_identifier_with_dot() {
        assert_eq!(increment_last_identifier("beta.0"), "beta.1");
        assert_eq!(increment_last_identifier("alpha.5"), "alpha.6");
        assert_eq!(increment_last_identifier("rc.99"), "rc.100");
    }

    #[test]
    fn test_increment_last_identifier_multiple_dots() {
        assert_eq!(increment_last_identifier("alpha.0.5"), "alpha.0.6");
        assert_eq!(increment_last_identifier("beta.1.2"), "beta.1.3");
    }

    #[test]
    fn test_increment_last_identifier_non_numeric() {
        assert_eq!(increment_last_identifier("beta"), "beta.1");
        assert_eq!(increment_last_identifier("alpha"), "alpha.1");
    }

    #[test]
    fn test_increment_last_identifier_non_numeric_suffix() {
        assert_eq!(increment_last_identifier("beta.abc"), "beta.abc.1");
    }

    #[test]
    fn test_build_metadata_preserved() {
        let version = Version::parse("1.2.3+build123").unwrap();
        let major = version.increment_major();
        assert_eq!(major.build, version.build);

        let minor = version.increment_minor();
        assert_eq!(minor.build, version.build);

        let patch = version.increment_patch();
        assert_eq!(patch.build, version.build);
    }
}
