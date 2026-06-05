//! Lineage: prompt 00_nucleo/prompt/0003-adaptador_l3.md
//!
//! Structs-espelho do JSON do fork. Só servem ao `serde_json`; campos como
//! `kind`/`visibility`/`relation` ficam como `String` aqui e só viram os enums
//! fortes do `lente_core` na tradução (validação na borda — ADR-0002 D1).

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct GrafoDTO {
    #[serde(rename = "crate")]
    pub(crate) crate_name: String,
    pub(crate) nodes: Vec<NoDTO>,
    pub(crate) edges: Vec<ArestaDTO>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct NoDTO {
    pub(crate) id: usize,
    pub(crate) path: String,
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) visibility: String,
    // Campos do descritor (fork 0.27.0). Emitidos só quando aplicam, por isso
    // todos com `#[serde(default)]` — ausência é normal, não erro (≠ `id`).
    #[serde(default)]
    pub(crate) is_const: bool,
    #[serde(default)]
    pub(crate) is_async: bool,
    #[serde(default)]
    pub(crate) is_unsafe: bool,
    #[serde(default)]
    pub(crate) is_non_exhaustive: bool,
    // `trait` é palavra reservada em Rust → rename.
    #[serde(default, rename = "trait")]
    pub(crate) trait_: Option<String>,
    #[serde(default)]
    pub(crate) trait_ref: Option<String>,
    // `cfg` vem ESTRUTURADO do fork (ex.: `[{"Flag":"unix"}]`), não como
    // string. Aceitamos como `Value` cru e serializamos para texto na
    // tradução (o `lente_core` modela `cfg: Option<String>`).
    #[serde(default)]
    pub(crate) cfg: Option<serde_json::Value>,
    #[serde(default)]
    pub(crate) macro_kind: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ArestaDTO {
    pub(crate) from: String,
    pub(crate) id_from: usize,
    pub(crate) to: String,
    pub(crate) id_to: usize,
    pub(crate) relation: String,
    /// `uses_kind` emitido pelo fork pós-commit `b44aa96` (prompt 0034). Só
    /// para arestas `Uses`. `None` quando o JSON é antigo (campo ausente).
    /// Texto cru aqui; a tradução para o enum acontece em `traducao.rs`.
    #[serde(default)]
    pub(crate) uses_kind: Option<String>,
}
