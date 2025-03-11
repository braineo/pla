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
    /// Remove prelease suffix from version.
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
