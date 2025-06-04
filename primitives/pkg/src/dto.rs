//! Digital Transfer Object (DTO) definitions
//!
//! Since some serializers (like postcard e.g.) cannot work with serde's skip feature (e.g. `#[serde(skip_serializing_if = "Option::is_none")]`),
//! the base definitions in this crate do not use this feature. However, when working with API's we typically encode things as jsons
//! (and the json serializer does not have this problem). In such cases, including fields that are not present creates a lot of waste in the encoding:
//! ```json
//! {
//!  "name": "flipper-contract",
//!  "app_name": null,
//!  "app_module": null,
//!  "capabilities": null,
//!  "pkg_type": "contract",
//!  "meta": {
//!    "authors": [],
//!    "description": null,
//!    "documentation": null,
//!    "license": null,
//!    "repository": null
//!  }
//!  // ...
//! }
//! ````
//! This is where the DTO's come in handy. They are an almost identical representation, but with additional serde features, that make working with
//! APIs more easy and produce a cleaner user experience.
//!
//! Please note: Not every datatype requires a DTO version, since this is only necessary if some conditional serialization for defaultable fields is involved.

use super::*;

/// DTO for [`Registry`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryDto {
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registry_type: Option<String>, // NOTE: We can expand on that later
    pub registry_hostname: String,
    pub namespace: String,
}

impl From<RegistryDto> for Registry {
    fn from(value: RegistryDto) -> Self {
        Self {
            registry_type: value.registry_type,
            registry_hostname: value.registry_hostname,
            namespace: value.namespace,
        }
    }
}

impl From<Registry> for RegistryDto {
    fn from(value: Registry) -> Self {
        Self {
            registry_type: value.registry_type,
            registry_hostname: value.registry_hostname,
            namespace: value.namespace,
        }
    }
}

/// DTO for [`PkgMeta`]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PkgMetaDto {
    /// Authors of the package
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<Author>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

impl From<PkgMetaDto> for PkgMeta {
    fn from(value: PkgMetaDto) -> Self {
        Self {
            authors: value.authors,
            description: value.description,
            documentation: value.documentation,
            license: value.license,
            repository: value.repository,
        }
    }
}

impl From<PkgMeta> for PkgMetaDto {
    fn from(value: PkgMeta) -> Self {
        Self {
            authors: value.authors,
            description: value.description,
            documentation: value.documentation,
            license: value.license,
            repository: value.repository,
        }
    }
}

impl PkgMetaDto {
    /// Returns true, if all fields are either empty or set to their default value
    pub fn is_empty(&self) -> bool {
        self.authors.is_empty()
            && self.description.is_none()
            && self.documentation.is_none()
            && self.license.is_none()
            && self.repository.is_none()
    }
}

/// DTO for [`WasmPkg`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkgDto {
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_module: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,

    pub pkg_type: PkgType,

    #[serde(default)]
    #[serde(skip_serializing_if = "PkgMetaDto::is_empty")]
    pub meta: PkgMetaDto,

    pub source: Source,
}

impl From<WasmPkgDto> for WasmPkg {
    fn from(value: WasmPkgDto) -> Self {
        Self {
            name: value.name,
            app_name: value.app_name,
            app_module: value.app_module,
            capabilities: value.capabilities,
            pkg_type: value.pkg_type,
            meta: value.meta.into(),
            source: value.source,
        }
    }
}

impl From<WasmPkg> for WasmPkgDto {
    fn from(value: WasmPkg) -> Self {
        Self {
            name: value.name,
            app_name: value.app_name,
            app_module: value.app_module,
            capabilities: value.capabilities,
            pkg_type: value.pkg_type,
            meta: value.meta.into(),
            source: value.source,
        }
    }
}

/// DTO for [`WasmPkgNoSource`]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmPkgNoSourceDto {
    pub name: String,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_name: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_module: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,

    pub pkg_type: PkgType,

    #[serde(default)]
    #[serde(skip_serializing_if = "PkgMetaDto::is_empty")]
    pub meta: PkgMetaDto,
}

impl From<WasmPkgNoSourceDto> for WasmPkgNoSource {
    fn from(value: WasmPkgNoSourceDto) -> Self {
        Self {
            name: value.name,
            app_name: value.app_name,
            app_module: value.app_module,
            capabilities: value.capabilities,
            pkg_type: value.pkg_type,
            meta: value.meta.into(),
        }
    }
}

impl From<WasmPkgNoSource> for WasmPkgNoSourceDto {
    fn from(value: WasmPkgNoSource) -> Self {
        Self {
            name: value.name,
            app_name: value.app_name,
            app_module: value.app_module,
            capabilities: value.capabilities,
            pkg_type: value.pkg_type,
            meta: value.meta.into(),
        }
    }
}
