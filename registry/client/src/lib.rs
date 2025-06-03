pub mod error;

use async_trait::async_trait;
use borderless_format::{pkg::Pkg, registry::ContractService};
use borderless_hash::Hash256;
use error::Error;
use reqwest::Client;
use serde_json;

pub struct ContractRegistryClient {
    url: String,
    client: Client,
}

impl ContractRegistryClient {
    pub fn new(url: &str) -> Self {
        ContractRegistryClient {
            client: Client::new(),
            url: url.to_string(),
        }
    }

    fn build_url(&self, route: &str) -> String {
        format!("{}/{}", self.url, route)
    }
}

#[async_trait]
impl ContractService for ContractRegistryClient {
    type Error = Error;

    async fn get_contract(&self, hash: Hash256) -> Result<Vec<u8>, Self::Error> {
        let url = format!("{}{}", self.build_url("/registry/contract/"), hash);

        let response = reqwest::get(&url).await?;

        if !response.status().is_success() {
            return Err(Error::Dummy);
        }

        let bytes = response.bytes().await?.to_vec();
        Ok(bytes)
    }

    async fn list_pkg(&self) -> Result<Vec<String>, Self::Error> {
        let response = self
            .client
            .get(self.build_url("registry/pkg"))
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Dummy);
        }

        let pks: Vec<String> = serde_json::from_str(&response.text().await?)?;
        Ok(pks)
    }

    async fn create_pkg(&self, pkg: borderless_format::pkg::InsertPkg) -> Result<(), Self::Error> {
        let url = self.build_url("registry/pkg/create");
        let json = serde_json::to_string(&pkg)?;

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/octet-stream")
            .body(json)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(Error::Dummy);
        }

        Ok(())
    }

    async fn read_pkg(&self, name: String) -> Result<borderless_format::pkg::Pkg, Self::Error> {
        let url = format!("{}{}", self.build_url("registry/pkg/"), name);
        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(Error::Dummy);
        }

        let pkg: Pkg = serde_json::from_str(&response.text().await?)?;
        Ok(pkg)
    }
}

#[cfg(test)]
mod tests {
    use borderless_format::registry::ContractService;

    use crate::ContractRegistryClient;

    #[tokio::test]
    async fn list_pkg() -> Result<(), Box<dyn std::error::Error>> {
        let client = ContractRegistryClient::new("http://127.0.0.1:3000");
        client.list_pkg().await?;
        assert!(true);
        Ok(())
    }
}
