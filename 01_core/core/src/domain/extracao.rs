//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/extracao.md
//! @prompt-hash 00000000
//! @layer L1
//! @updated 2026-09-26
//! Envelope puro da fronteira de extração (ADR-0007).

use crate::entities::grafo::Grafo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentidadeProvedor {
    RustCargo,
    Json,
    Memoria,
}

/// O grafo cru satisfaz os invariantes da spec; paths ainda podem colidir.
/// Diagnósticos são observações não fatais, nunca substitutos de um erro fatal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtracaoGrafo {
    pub grafo: Grafo,
    pub provedor: IdentidadeProvedor,
    pub diagnosticos: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn envelope_preserva_dados_e_diagnosticos() {
        let e = ExtracaoGrafo {
            grafo: Grafo { crate_name: "amostra".into(), nodes: vec![], edges: vec![] },
            provedor: IdentidadeProvedor::Memoria,
            diagnosticos: vec!["limite declarado".into()],
        };
        assert_eq!(e.clone(), e);
        assert_eq!(e.grafo.crate_name, "amostra");
        assert_eq!(e.diagnosticos, ["limite declarado"]);
    }
}
