//! Definition of a borderless wasm package
//!
//! SmartContracts aswell as SoftwareAgents are compiled to webassembly, to be then executed on our runtime.
//! However, since it is not very handy to directly work with the compiled modules, we defined a package format,
//! that bundles the `.wasm` module together with some meta information about the package.
//!
use borderless_hash::Hash256;
use serde::{Deserialize, Serialize};

pub use crate::author::Author;
pub use crate::semver::SemVer;

mod author;
pub mod semver;

/// Defines how to fetch the wasm code from a registry
///
/// For now the definition is quite basic, but we can later expand on this and support different
/// types of registries, that may have different interfaces.
///
/// Right now the idea is to use the OCI standard here, so the full URI of some package will be
/// `registry_hostname/namespace/pkg-name:pkg-version`
///
/// This then has to be translated into a proper URL based on the registry type to fetch the actual content.
///
/// Please note: The definition of the package-name and version is not part of the `Registry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Registry {
    /// Type of registry. If none given, the OCI standard is used.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_type: Option<String>, // NOTE: We can expand on that later

    /// Base-URL of the registry
    pub registry_hostname: String,

    /// Namespace in the registry
    ///
    /// This can be an organization or arbitrary namespace.
    pub namespace: String,
}

/// Specifies the source type - aka how to get the wasm module
///
/// This is either a [`Registry`], which can be used to download the `.wasm` blob,
/// or it is an inline definition, that just contains the compiled `.wasm` module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SourceType {
    /// Registry, where the wasm module can be fetched from
    Registry(Registry),

    /// Ready to use, compiled wasm module
    #[serde(with = "serde_bytes")]
    Wasm(Vec<u8>),
}

/// Specifies the complete source of a wasm module
///
/// This contains the version, concrete source ( either local bytes or link to a remote registry ) and hash digest of the compiled module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Version of the wasm module
    version: SemVer,

    /// Sha3-256 digest of the module
    digest: Hash256,

    /// Concrete source - see [`SourceType`]
    #[serde(flatten)]
    code: SourceType,
}

/// Package metadata
///
/// Contains things like the authors, license, link to documentation etc.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PkgMeta {
    /// Authors of the package
    #[serde(default)]
    pub authors: Vec<Author>,

    /// A description of the package
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// URL of the package documentation
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,

    /// License information
    ///
    /// SPDX 2.3 license expression
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// URL of the package source repository
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PkgType {
    Contract,
    Agent,
}

/// Definition of a wasm package
///
/// Contains the necessary information about the source, a name for the package
/// and (optional) package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkg {
    /// Name of the package
    pub name: String,

    /// Package type (contract or agent)
    pub pkg_type: PkgType,

    /// Package metadata
    #[serde(default)]
    pub meta: PkgMeta,

    /// Package source
    pub source: Source,
}

/// A signed wasm package
///
/// The signature is generated, by first generating the json-proof for the [`WasmPkg`] and then signing it with some private-key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkgSigned {
    /// Package definition
    #[serde(flatten)]
    pub pkg: WasmPkg,

    /// Base-16 encoded signature
    pub signature: String,
}
