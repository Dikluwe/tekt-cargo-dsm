//! Lineage: prompt 00_nucleo/prompt/0001-dados_grafo.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0001-fonte-do-grafo-fork-externo.md
//!          00_nucleo/adr/0002-modelagem-do-grafo.md
//! Camada:  L1 — Núcleo (puro, apenas stdlib)

#![forbid(unsafe_code)]
#![deny(unused_must_use)]

pub mod domain;
pub mod entities;
