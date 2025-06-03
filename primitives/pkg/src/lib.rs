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

// TODO: When using this with the CLI, it may be beneficial to add builders to all of those types.
// However, this should be gated behind a feature flag, as other consumers of this library only require the parsing logic.

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
#[serde(untagged)]
pub enum SourceType {
    /// Registry, where the wasm module can be fetched from
    Registry { registry: Registry },

    /// Ready to use, compiled wasm module
    Wasm {
        #[serde(with = "code_as_base64")]
        wasm: Vec<u8>,
    },
}

mod code_as_base64 {
    use base64::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        let base64 = BASE64_STANDARD.encode(v);
        String::serialize(&base64, s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let b64 = String::deserialize(d)?;
        BASE64_STANDARD
            .decode(b64.as_bytes())
            .map_err(|e| serde::de::Error::custom(e))
    }
}

/// Specifies the complete source of a wasm module
///
/// This contains the version, concrete source ( either local bytes or link to a remote registry ) and hash digest of the compiled module.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    /// Version of the wasm module
    pub version: SemVer,

    /// Sha3-256 digest of the module
    pub digest: Hash256,

    /// Concrete source - see [`SourceType`]
    #[serde(flatten)]
    pub code: SourceType,
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

/// Capabilities of a SW-Agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Weather or not the agent is allowed to make http-calls
    pub network: bool,
    /// Weather or not the agent is allowed to establish websocket connections
    pub websocket: bool,
    /// URLs that the agent is allowed to call
    pub url_whitelist: Vec<String>,
}

/// Definition of a wasm package
///
/// Contains the necessary information about the source, a name for the package
/// and (optional) package metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkg {
    /// Name of the package
    pub name: String,

    /// Name of the application that this package is a part of
    ///
    /// An application is just an abstraction for multiple wasm packages.
    /// It can be further split into application modules.
    ///
    /// The full specifier for the package would be (if application and app-modules are used):
    /// `<app_name>/<app_module>/<pkg-name>`
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    /// Name of the application module that this package is a part of
    ///
    /// An application module is a subset of wasm modules in an application.
    ///
    /// The full specifier for the package would be (if application and app-modules are used):
    /// `<app_name>/<app_module>/<pkg-name>`
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_module: Option<String>,

    /// (Networking) Capabilities of the package
    ///
    /// This is only used for software agents, which can make network calls and may use a websocket.
    /// The capabilities are registered in the runtime, so that the agent cannot make any other network
    /// calls than specified by the url-whitelist in [`Capabilities`].
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,

    /// Package type (contract or agent)
    pub pkg_type: PkgType,

    /// Package metadata
    #[serde(default)]
    pub meta: PkgMeta,

    /// Package source
    pub source: Source,
}

// TODO: Use json-proof package here
// TODO: Or - do we need this ? we could simply sign the Vec<u8> of the "WasmPkg" and call it a day.
//       I think the signing is also only a thing for the registries, because when sending introductions with inline code definition,
//       the message is always signed by our p2p protocols..
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_deserialize() {
        let s = r#"{
            "version": "1.2.3",
            "digest": "",
            "wasm": "AGFzbQEAAAABnAIqYAF/"
        }"#;
        let source: Result<Source, _> = serde_json::from_str(s);
        assert!(source.is_ok());
        let source = source.unwrap();
        dbg!(source);
        assert!(false);
    }
}
