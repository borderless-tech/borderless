use crate::error::ErrorKind;
use crate::rt::CodeStore;
use crate::{error::Result, Error};
use base64::{engine::general_purpose, Engine as _};
use borderless::ContractId;
use borderless_format::{Bundle, Ident, Metadata as Meta, Source};
use borderless_kv_store::Db;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hex;
use std::collections::HashMap;
use wasmtime::{Engine, ExternType, FuncType, Module};

/// Describe the grade of verficiation
pub enum VerificationLevel {
    /// Author signed the contract bundle
    /// and key identity is linked to
    /// our chain.
    AuthorVerified,

    /// Someone signed the contract bundle
    /// Private Key is unkown.
    SignatureVerified,

    /// Contract bundle is not signed!
    NotVerified,
}

pub struct Registry<D: Db> {
    code_store: CodeStore<D>,

    // store the meta information from the package
    // NOTE
    // 1. should also live in the db not in the heap
    // 2. Option<Ident> and the VerificationLevel correspond
    //    refactor this in a way thah Ident equals None not implicit mean
    //    VerificationLevel::NotVerified.
    // 3. ContractId does not work for agents? maybe the hash of the wasm module
    //    itself is a better key
    meta: HashMap<ContractId, (Option<Ident>, VerificationLevel, Meta)>,
}

impl<D> Registry<D>
where
    D: Db,
{
    pub fn new(store: CodeStore<D>) -> Self {
        Registry {
            code_store: store,
            meta: HashMap::new(),
        }
    }

    pub fn insert_contract(
        &mut self,
        engine: &Engine,
        contract_id: ContractId,
        data: Bundle,
    ) -> Result<()> {
        // split the bundle in parts();
        let (ident, meta, src) = data.parts();

        // verify the bundle
        let verification_level = Self::verify_bundle(&data)?;

        // create the wasmtime module from bundle
        let module = Self::init_wasm_module(engine, &src)?;

        // check the contract
        Self::check_contract_module(&engine, &module)?;

        // insert module to code store
        self.code_store.insert_contract(contract_id, module)?;

        // store metadata
        self.meta.insert(
            contract_id,
            (ident.clone(), verification_level, meta.clone()),
        );

        Ok(())
    }

    fn init_wasm_module(engine: &Engine, src: &Source) -> Result<Module> {
        let wasm_bytes: Vec<u8> = general_purpose::STANDARD.decode(&src.wasm)?;
        let module = Module::new(engine, &wasm_bytes)?;
        Ok(module)
    }

    fn verify_bundle(bundle: &Bundle) -> Result<VerificationLevel> {
        if bundle.ident.is_none() {
            return Ok(VerificationLevel::NotVerified);
        }

        let ident: &Ident = bundle.ident.as_ref().unwrap();
        let verifying_key = Self::decode_verifying_key(&ident.public_key)?;
        let signature = Self::decode_signature(&ident.signature)?;

        // verify the contract
        let json = serde_json::to_string(&bundle.contract)?;
        verifying_key.verify(json.as_bytes(), &signature)?;

        // TODO
        // check against chain of thrust
        // for now we are fine!
        Ok(VerificationLevel::SignatureVerified)
    }

    fn decode_verifying_key(hex_key: &str) -> Result<VerifyingKey> {
        let key_bytes = hex::decode(hex_key)?;

        if key_bytes.len() != 32 {
            return Err(Error::from(hex::FromHexError::InvalidStringLength));
        }

        let key: VerifyingKey = VerifyingKey::from_bytes(&key_bytes.try_into().unwrap())?;
        Ok(key)
    }

    fn decode_signature(hex_signature: &str) -> Result<Signature> {
        let signature_bytes = hex::decode(hex_signature)?;

        if signature_bytes.len() != 64 {
            return Err(Error::from(hex::FromHexError::InvalidStringLength));
        }

        let signature: Signature = Signature::from_bytes(&signature_bytes.try_into().unwrap());
        Ok(signature)
    }

    fn check_contract_module(engine: &Engine, module: &Module) -> Result<()> {
        let functions = [
            "process_transaction",
            "process_introduction",
            "process_revocation",
            "http_get_state",
            "http_post_action",
            "get_symbols",
        ];
        for func in functions {
            let exp = module
                .get_export(func)
                .ok_or_else(|| ErrorKind::MissingExport { func })?;
            if let ExternType::Func(func_type) = exp {
                if !func_type.matches(&FuncType::new(engine, [], [])) {
                    return Err(ErrorKind::InvalidFuncType { func }.into());
                }
            } else {
                return Err(ErrorKind::InvalidExport { func }.into());
            }
        }
        Ok(())
    }

    fn check_agent_module(engine: &Engine, module: &Module) -> Result<()> {
        let functions = [
            "on_init",
            "on_shutdown",
            "process_action",
            "process_introduction",
            "process_revocation",
            "http_get_state",
            "http_post_action",
            "get_symbols",
        ];
        for func in functions {
            let exp = module
                .get_export(func)
                .ok_or_else(|| ErrorKind::MissingExport { func })?;
            if let ExternType::Func(func_type) = exp {
                if !func_type.matches(&FuncType::new(engine, [], [])) {
                    return Err(ErrorKind::InvalidFuncType { func }.into());
                }
            } else {
                return Err(ErrorKind::InvalidExport { func }.into());
            }
        }
        Ok(())
    }
}

impl<D> AsMut<CodeStore<D>> for Registry<D>
where
    D: Db,
{
    fn as_mut(&mut self) -> &mut CodeStore<D> {
        &mut self.code_store
    }
}

impl<D> AsRef<CodeStore<D>> for Registry<D>
where
    D: Db,
{
    fn as_ref(&self) -> &CodeStore<D> {
        &self.code_store
    }
}
