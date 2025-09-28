use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use credit_engine_core::amm::errors::{AmmErrorDescriptor, AMM_ERROR_DESCRIPTORS};

#[derive(Debug, Clone)]
pub struct CatalogDocument {
    pub meta: CatalogMeta,
    pub errors: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogMeta {
    pub domain: String,
    pub prefix: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogEntry {
    pub variant: String,
    pub code: String,
    pub message: String,
    pub http_status: u16,
}

struct EntryBuilder {
    variant: Option<String>,
    code: Option<String>,
    message: Option<String>,
    http_status: Option<u16>,
}

impl EntryBuilder {
    fn new() -> Self {
        Self {
            variant: None,
            code: None,
            message: None,
            http_status: None,
        }
    }

    fn insert(&mut self, key: &str, value: &str) {
        let value = value.trim();
        let cleaned = value.trim_matches('"').to_string();
        match key {
            "variant" => self.variant = Some(cleaned),
            "code" => self.code = Some(cleaned),
            "default_message" => self.message = Some(cleaned),
            "http_status" => {
                let parsed: u16 = cleaned
                    .parse()
                    .unwrap_or_else(|_| panic!("http_status inválido: {cleaned}"));
                self.http_status = Some(parsed);
            }
            other => panic!("chave desconhecida em entrada do catálogo: {other}"),
        }
    }

    fn finish(self) -> CatalogEntry {
        CatalogEntry {
            variant: self.variant.expect("variant ausente em entrada do catálogo"),
            code: self.code.expect("code ausente em entrada do catálogo"),
            message: self.message.expect("default_message ausente em entrada do catálogo"),
            http_status: self
                .http_status
                .expect("http_status ausente em entrada do catálogo"),
        }
    }
}

fn parse_catalog(contents: &str) -> CatalogDocument {
    #[derive(PartialEq)]
    enum Section {
        None,
        Meta,
        Errors,
    }

    let mut section = Section::None;
    let mut meta = CatalogMeta {
        domain: String::new(),
        prefix: String::new(),
        version: 0,
    };
    let mut errors: Vec<CatalogEntry> = Vec::new();
    let mut builder: Option<EntryBuilder> = None;

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        match trimmed {
            "meta:" => {
                section = Section::Meta;
                continue;
            }
            "errors:" => {
                if let Some(current) = builder.take() {
                    errors.push(current.finish());
                }
                section = Section::Errors;
                continue;
            }
            _ => {}
        }

        match section {
            Section::Meta => {
                if let Some((key, value)) = trimmed.split_once(':') {
                    let cleaned = value.trim().trim_matches('"');
                    match key.trim() {
                        "domain" => meta.domain = cleaned.to_string(),
                        "prefix" => meta.prefix = cleaned.to_string(),
                        "version" => {
                            meta.version = cleaned
                                .parse()
                                .unwrap_or_else(|_| panic!("versão inválida no catálogo: {cleaned}"));
                        }
                        other => panic!("chave de meta desconhecida: {other}"),
                    }
                }
            }
            Section::Errors => {
                if trimmed.starts_with('-') {
                    if let Some(current) = builder.take() {
                        errors.push(current.finish());
                    }
                    let mut entry = EntryBuilder::new();
                    let rest = trimmed.trim_start_matches('-').trim();
                    let (key, value) = rest
                        .split_once(':')
                        .unwrap_or_else(|| panic!("linha inválida no catálogo: {trimmed}"));
                    entry.insert(key.trim(), value);
                    builder = Some(entry);
                } else if let Some((key, value)) = trimmed.split_once(':') {
                    let entry = builder
                        .as_mut()
                        .unwrap_or_else(|| panic!("linha fora de entrada de erro: {trimmed}"));
                    entry.insert(key.trim(), value);
                }
            }
            Section::None => {}
        }
    }

    if let Some(current) = builder.take() {
        errors.push(current.finish());
    }

    if meta.domain.is_empty() || meta.prefix.is_empty() || meta.version == 0 {
        panic!("metadata incompleta no catálogo");
    }

    CatalogDocument { meta, errors }
}

pub fn load_catalog() -> CatalogDocument {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("ops/errors/catalog_amm.yaml");
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("não foi possível ler {path:?}: {err}"));
    parse_catalog(&raw)
}

pub fn descriptors_by_variant() -> BTreeMap<String, &'static AmmErrorDescriptor> {
    AMM_ERROR_DESCRIPTORS
        .iter()
        .map(|descriptor| (descriptor.variant.variant_name().to_string(), descriptor))
        .collect()
}

pub fn runtime_catalog_snapshot() -> BTreeMap<String, CatalogEntry> {
    descriptors_by_variant()
        .into_iter()
        .map(|(variant, descriptor)| {
            let entry = CatalogEntry {
                variant: variant.clone(),
                code: descriptor.code.to_string(),
                message: descriptor.message.to_string(),
                http_status: descriptor.http_status,
            };
            (variant, entry)
        })
        .collect()
}

impl CatalogDocument {
    pub fn entries_by_variant(&self) -> BTreeMap<String, CatalogEntry> {
        self.errors
            .iter()
            .cloned()
            .map(|entry| (entry.variant.clone(), entry))
            .collect()
    }
}
