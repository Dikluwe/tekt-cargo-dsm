//! Lineage: prompt 00_nucleo/prompt/0020-l2-cli.md
//!          virou biblioteca de apresentação por prompt 00_nucleo/prompt/0057-estagio3_relocar_ponto_entrada.md
//! Camada:  L2 — Casca (apresentação). Nasce sob o Tekt ADR-0002: zero literais
//!          de apresentação fora do `lente_catalogo`.
//!
//! **Apresentação pura** da lente: os argumentos (`args`, structs `clap`) e os
//! formatadores (`saida`, sobre tipos **L1**). **Não importa L4** — o ponto de
//! entrada (o `main`, o dispatch e a tradução do `ErroLente`) vive no crate
//! `lente_app` (L4, `04_wiring/app`), que compõe esta apresentação com a
//! orquestração (Estágio 3 do refactor V3+V12, prompt 0057).

pub mod args;
pub mod saida;
