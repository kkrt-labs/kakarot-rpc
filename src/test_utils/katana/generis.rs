use std::path::PathBuf;
use std::{collections::HashMap, path::Path};

use cairo_lang_starknet::casm_contract_class::CasmContractClass;
use cairo_lang_starknet::contract_class::ContractClass;
use eyre::Result;
use katana_primitives::{
    contract::ContractAddress,
    genesis::json::{GenesisClassJson, GenesisContractJson, PathOrFullArtifact},
};
use starknet::core::types::contract::legacy::LegacyContractClass;
use starknet::core::types::FieldElement;
use walkdir::WalkDir;

#[derive(Default)]
pub struct KatanaGenesisBuilder {
    classes: Vec<GenesisClassJson>,
    class_hashes: HashMap<String, FieldElement>,
    contracts: HashMap<ContractAddress, GenesisContractJson>,
}

impl KatanaGenesisBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_classes(&mut self, path: PathBuf) -> &mut Self {
        let entries = WalkDir::new(path).into_iter().filter(|e| e.as_ref().unwrap().file_type().is_file());
        self.classes = entries
            .map(|entry| GenesisClassJson {
                class: PathOrFullArtifact::Path(entry.unwrap().path().to_path_buf()),
                class_hash: None,
            })
            .collect::<Vec<_>>();

        self.class_hashes = self
            .classes
            .iter()
            .filter_map(|class| {
                let path = match &class.class {
                    PathOrFullArtifact::Path(path) => path,
                    _ => unreachable!("Expected path"),
                };
                let class_hash = compute_class_hash(path).ok()?;
                Some((path.file_stem().unwrap().to_str().unwrap().to_string(), class_hash))
            })
            .collect::<HashMap<_, _>>();

        self
    }

    pub fn with_kakarot(&mut self, contracts: Vec<GenesisContractJson>) -> Result<&mut Self> {}
}

fn compute_class_hash(class_path: &Path) -> Result<FieldElement> {
    let class_code = std::fs::read_to_string(class_path).expect("Failed to read class code");
    match serde_json::from_str::<ContractClass>(&class_code) {
        Ok(casm) => {
            let casm = CasmContractClass::from_contract_class(casm, true).expect("Failed to convert class");
            Ok(FieldElement::from_bytes_be(&casm.compiled_class_hash().to_be_bytes())?)
        }
        Err(_) => {
            let casm: LegacyContractClass = serde_json::from_str(&class_code).expect("Failed to parse class code v0");
            Ok(casm.class_hash()?)
        }
    }
}
