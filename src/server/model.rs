use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CratesPublish {
    // List of strings of the authors.
    // May be empty.
    pub authors: Vec<String>,
    // Optional object of "status" badges. Each value is an object of
    // arbitrary string to string mappings.
    // crates.io has special interpretation of the format of the badges.
    pub badges: Badge,
    // Array of strings of categories for the package.
    pub categories: Vec<String>,
    // Array of direct dependencies of the package.
    pub deps: Vec<Dep>,
    // Description field from the manifest.
    // May be null. crates.io requires at least some content.
    pub description: String,
    // String of the URL to the website for this package's documentation.
    // May be null.
    pub documentation: String,
    // Set of features defined for the package.
    // Each feature maps to an array of features or dependencies it enables.
    // Cargo does not impose limitations on feature names, but crates.io
    // requires alphanumeric ASCII, `_` or `-` characters.
    pub features: BTreeMap<String, Vec<String>>,
    // String of the URL to the website for this package's home page.
    // May be null.
    pub homepage: String,
    // Array of strings of keywords for the package.
    pub keywords: Vec<String>,
    // String of the license for the package.
    // May be null. crates.io requires either `license` or `license_file` to be set.
    pub license: String,
    // String of a relative path to a license file in the crate.
    // May be null.
    pub license_file: Option<String>,
    // The `links` string value from the package's manifest, or null if not
    // specified. This field is optional and defaults to null.
    pub links: Option<String>,
    pub name: String,
    // String of the content of the README file.
    // May be null.
    pub readme: String,
    // String of a relative path to a README file in the crate.
    // May be null.
    pub readme_file: String,
    // String of the URL to the website for the source repository of this package.
    // May be null.
    pub repository: String,
    // The version of the package being published.
    pub vers: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Dep {
    // Boolean of whether or not default features are enabled.
    pub default_features: bool,
    // Array of features (as strings) enabled for this dependency.
    pub features: Vec<String>,
    // The dependency kind.
    // "dev", "build", or "normal".
    pub kind: String,
    // Name of the dependency.
    // If the dependency is renamed from the original package name,
    // this is the original name. The new package name is stored in
    // the `explicit_name_in_toml` field.
    pub name: String,
    // Boolean of whether or not this is an optional dependency.
    pub optional: bool,
    // The URL of the index of the registry where this dependency is
    // from as a string. If not specified or null, it is assumed the
    // dependency is in the current registry.
    pub registry: String,
    // The target platform for the dependency.
    // null if not a target dependency.
    // Otherwise, a string such as "cfg(windows)".
    pub target: Option<String>,
    // The semver requirement for this dependency.
    pub version_req: String,
    // If the dependency is renamed, this is a string of the new
    // package name. If not specified or null, this dependency is not
    // renamed.
    pub explicit_name_in_toml: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Badge {}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PublishRsp {
    // Optional object of warnings to display to the user.
    pub warnings: Warning,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Warning {
    // Array of strings of categories that are invalid and ignored.
    pub invalid_categories: Vec<String>,
    // Array of strings of badge names that are invalid and ignored.
    pub invalid_badges: Vec<String>,
    // Array of strings of arbitrary warnings to display to the user.
    pub other: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Errors {
    // Array of errors to display to the user.
    pub errors: Vec<ErrorDetail>,
}

impl Errors {
    pub fn new(detail: String) -> Errors {
        Errors {
            errors: vec![ErrorDetail { detail }],
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    // The error message as a string.
    pub detail: String,
}
