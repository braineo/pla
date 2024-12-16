use inquire::Select;
use owo_colors::{colors::xterm, OwoColorize};
use semver::Version;
use std::fmt::{Display, Formatter};

use crate::bump_version::BumpVersion;

struct VersionLabel {
    name: &'static str,
    version: Version,
}

impl VersionLabel {
    pub fn new(name: &'static str, version: Version) -> Self {
        Self { name, version }
    }
}

impl Display for VersionLabel {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{: >9} {}", self.name, self.version)
    }
}

pub fn prompt_version_select(current_version: &Version) -> Version {
    let options = vec![
        Some(VersionLabel::new(
            "major",
            current_version.increment_major(),
        )),
        Some(VersionLabel::new(
            "minor",
            current_version.increment_minor(),
        )),
        Some(VersionLabel::new(
            "patch",
            current_version.increment_patch(),
        )),
        Some(VersionLabel::new(
            "next",
            if current_version.pre.is_empty() {
                current_version.increment_patch()
            } else {
                current_version.increment_prerelease()
            },
        )),
        if !current_version.pre.is_empty() {
            Some(VersionLabel::new(
                "release",
                current_version.convert_prerelease_to_release(),
            ))
        } else {
            None
        },
        Some(VersionLabel::new(
            "pre-patch",
            current_version
                .increment_patch()
                .append_prerelease_identifiers("beta.0"),
        )),
        Some(VersionLabel::new(
            "pre-minor",
            current_version
                .increment_minor()
                .append_prerelease_identifiers("beta.0"),
        )),
        Some(VersionLabel::new(
            "pre-major",
            current_version
                .increment_major()
                .append_prerelease_identifiers("beta.0"),
        )),
        Some(VersionLabel::new("current", current_version.clone())),
    ]
    .into_iter()
    .flatten()
    .collect();

    let answer = Select::new(
        &format!("Current version {}", current_version.fg::<xterm::Green>()),
        options,
    )
    .with_starting_cursor(3)
    .prompt();

    match answer {
        Ok(choice) => choice.version,
        Err(_) => current_version.clone(),
    }
}
