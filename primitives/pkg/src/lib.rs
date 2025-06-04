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

impl Source {
    /// 'flattens' the `Source` to create a [`SourceFlattened`]
    ///
    /// Useful for serializers that do not support advanced serde features.
    pub fn flatten(self) -> SourceFlattened {
        let (registry, wasm) = match self.code {
            SourceType::Registry { registry } => (Some(registry), None),
            SourceType::Wasm { wasm } => (None, Some(wasm)),
        };
        SourceFlattened {
            version: self.version,
            digest: self.digest,
            registry,
            wasm,
        }
    }
}

/// A 'flattened' version of [`Source`]
///
/// Some serializers do not support all serde features, like untagged enums or flattening.
/// This struct is a replacement for [`Source`], in case your serializer cannot properly serialize the `Source` type.
/// In this version, the content of [`SourceType`] is directly inlined into the struct definition using options.
/// Also the base64 encoding of the wasm bytes is removed in this version.
///
/// You can see this as an "on-disk" version of `Source`. For transfer over the wire (especially with json !) you should use [`Source`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceFlattened {
    pub version: SemVer,

    /// Sha3-256 digest of the module
    pub digest: Hash256,

    #[serde(default)]
    registry: Option<Registry>,

    #[serde(default)]
    #[serde(with = "serde_bytes")]
    wasm: Option<Vec<u8>>,
}

impl SourceFlattened {
    /// 'unflattens' the data back into a [`Source`]
    ///
    /// Inverse operation of [`Source::flatten`].
    ///
    /// # Safety
    ///
    /// This function panics, if the `SourceFlattened` cannot be converted into a `Source`,
    /// because both `registry` and `wasm` are set to either `None` or `Some` ( it should be either or ).
    pub fn unflatten(self) -> Source {
        let code = match (self.registry, self.wasm) {
            (Some(registry), None) => SourceType::Registry { registry  },
            (None, Some(wasm)) => SourceType::Wasm { wasm  },
            _ => panic!("Failed to convert into `Source` - either `registry` or `wasm` must be set, but neither both or none"),
        };
        Source {
            version: self.version,
            digest: self.digest,
            code,
        }
    }
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
    pub description: Option<String>,

    /// URL of the package documentation
    #[serde(default)]
    pub documentation: Option<String>,

    /// License information
    ///
    /// SPDX 2.3 license expression
    #[serde(default)]
    pub license: Option<String>,

    /// URL of the package source repository
    #[serde(default)]
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
    pub app_name: Option<String>,

    /// Name of the application module that this package is a part of
    ///
    /// An application module is a subset of wasm modules in an application.
    ///
    /// The full specifier for the package would be (if application and app-modules are used):
    /// `<app_name>/<app_module>/<pkg-name>`
    #[serde(default)]
    pub app_module: Option<String>,

    /// (Networking) Capabilities of the package
    ///
    /// This is only used for software agents, which can make network calls and may use a websocket.
    /// The capabilities are registered in the runtime, so that the agent cannot make any other network
    /// calls than specified by the url-whitelist in [`Capabilities`].
    #[serde(default)]
    pub capabilities: Option<Capabilities>,

    /// Package type (contract or agent)
    pub pkg_type: PkgType,

    /// Package metadata
    #[serde(default)]
    pub meta: PkgMeta,

    /// Package source
    pub source: Source,
}

impl WasmPkg {
    /// Split the `Source` out of the `WasmPkg`, so we can store or handle it separately
    pub fn into_def_and_source(self) -> (WasmPkgNoSource, Source) {
        let pkg_def = WasmPkgNoSource {
            name: self.name,
            app_name: self.app_name,
            app_module: self.app_module,
            capabilities: self.capabilities,
            pkg_type: self.pkg_type,
            meta: self.meta,
        };
        let source = self.source;
        (pkg_def, source)
    }

    /// Merge the `Source` back into the `WasmPkg`
    pub fn from_def_and_source(pkg_def: WasmPkgNoSource, source: Source) -> Self {
        Self {
            name: pkg_def.name,
            app_name: pkg_def.app_name,
            app_module: pkg_def.app_module,
            capabilities: pkg_def.capabilities,
            pkg_type: pkg_def.pkg_type,
            meta: pkg_def.meta,
            source,
        }
    }
}

/// Definition of a wasm package - without the actual source
///
/// There are cases where you want to handle the package definition and the source seperately,
/// so we need a type to represent a `WasmPkg` without the actual `Source`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkgNoSource {
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
    pub app_name: Option<String>,

    /// Name of the application module that this package is a part of
    ///
    /// An application module is a subset of wasm modules in an application.
    ///
    /// The full specifier for the package would be (if application and app-modules are used):
    /// `<app_name>/<app_module>/<pkg-name>`
    #[serde(default)]
    pub app_module: Option<String>,

    /// (Networking) Capabilities of the package
    ///
    /// This is only used for software agents, which can make network calls and may use a websocket.
    /// The capabilities are registered in the runtime, so that the agent cannot make any other network
    /// calls than specified by the url-whitelist in [`Capabilities`].
    #[serde(default)]
    pub capabilities: Option<Capabilities>,

    /// Package type (contract or agent)
    pub pkg_type: PkgType,

    /// Package metadata
    #[serde(default)]
    pub meta: PkgMeta,
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
    }
}
