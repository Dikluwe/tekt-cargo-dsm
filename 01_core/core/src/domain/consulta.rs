//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/consulta.md
//! @prompt-hash a3fa1581
//! @layer L1
//! @updated 2026-06-07
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
//!
//! O **vocabulário de pedido** da lente: como o usuário aponta a fonte do grafo,
//! o alvo, o escopo e o modo de `Uses`. Eram tipos definidos no `lente_wiring`
//! (L4); o Estágio 2 do refactor V3+V12 (mapa 0054) os trouxe para o L1 — são
//! **dados puros** (só `String`/[`Path`]/unit), e a fiação passa a importá-los
//! daqui nas assinaturas. Mover o vocabulário ao L1 é o que tira a CLI (L2) de
//! depender da fachada L4.

use crate::entities::grafo::Path;

/// De onde vem o grafo: JSON pronto ou nome de pacote (invoca o fork).
pub enum FonteGrafo {
    /// JSON pronto (o L2 leu de arquivo ou stdin).
    Json(String),
    /// Nome de pacote — a fiação invoca o fork via `lente_infra::fork`.
    Pacote(String),
}

/// Escopo do grafo sobre o qual a lente responde — escolha do usuário
/// (prompt 0030).
///
/// **`Completo`** (default): forma resolvida inclui sysroot (`core::*`,
/// `std::*`, `alloc::*`, …). É o grafo cru-mas-resolvido como o fork
/// `cargo-modules` o entrega. Classificações refletem o que o nó usa,
/// inclusive stdlib.
///
/// **`SeuCodigo`**: forma resolvida com `lente_filtro::filtrar_stdlib`
/// aplicado — sysroot escondido (laudo 0025). Classificações refletem
/// só o que o nó usa **dentro do código do usuário** (mais
/// dependências não-stdlib).
///
/// **Invariante do montante** (prompt 0030 Fase 1, confirmado): para um
/// nó do código do usuário, o `montante` (quem-depende-de-mim) é o
/// **mesmo** nos dois escopos — stdlib não depende de código do
/// usuário. O escopo só muda `uses_saida` e, por consequência, a
/// classificação. Os campos `uses_entrada` (= "diretos") e
/// `montante.len()` (= "transitivos") permanecem.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Escopo {
    Completo,
    SeuCodigo,
}

impl Default for Escopo {
    fn default() -> Self {
        Escopo::Completo
    }
}

/// Modo de inclusão das arestas `Uses` no modo `--estrutura` — escolha
/// do usuário (prompt 0034).
///
/// **`Todas`** (default): inclui todas as `Uses` (a vista do laudo 0031;
/// SCC de 85 módulos no egui).
///
/// **`SoReferencia`**: inclui apenas `Uses` cujo `uses_kind == Reference`
/// (uso de tipo direto). Descarta `Import` (Limite 4) — o acoplamento
/// de tipo "real" (laudo 0033: SCC cai para 42).
///
/// **Caso `None`** (fork antigo, sem `uses_kind` no JSON): se o usuário
/// pediu `SoReferencia` e nenhuma aresta `Uses` do grafo tem `uses_kind`,
/// a fiação retorna `ErroLente::ForkSemUsesKind` — não silencia produzindo
/// `Todas` disfarçado.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModoUses {
    Todas,
    SoReferencia,
}

impl Default for ModoUses {
    fn default() -> Self {
        ModoUses::Todas
    }
}

/// Como o alvo do raio é apontado: por path canônico ou por id.
pub enum AlvoBusca {
    PorPath(Path),
    PorId(usize),
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Contrato dos defaults (estabelecido no 0056): o escopo default é o grafo
    /// completo (sysroot incluído) — a fiação conta com isso quando o usuário não
    /// pede `--seu-codigo`.
    #[test]
    fn escopo_default_e_completo() {
        assert_eq!(Escopo::default(), Escopo::Completo);
    }

    /// Contrato dos defaults (0056): o modo de `Uses` default inclui todas as
    /// arestas — a vista do laudo 0031, antes de o usuário pedir `SoReferencia`.
    #[test]
    fn modo_uses_default_e_todas() {
        assert_eq!(ModoUses::default(), ModoUses::Todas);
    }

    /// `FonteGrafo` não tem default (a fonte é sempre escolha explícita do L2);
    /// trava a construção e a discriminação dos dois variantes.
    #[test]
    fn fonte_grafo_discrimina_json_e_pacote() {
        assert!(matches!(FonteGrafo::Json("{}".into()), FonteGrafo::Json(_)));
        assert!(matches!(
            FonteGrafo::Pacote("egui".into()),
            FonteGrafo::Pacote(_)
        ));
    }

    /// `AlvoBusca` não tem default (o alvo é sempre apontado); trava a construção
    /// por path e por id, e que o `usize` apontado é preservado.
    #[test]
    fn alvo_busca_por_path_e_por_id() {
        assert!(matches!(
            AlvoBusca::PorPath(Path::new("crate::a::b")),
            AlvoBusca::PorPath(_)
        ));
        match AlvoBusca::PorId(42) {
            AlvoBusca::PorId(id) => assert_eq!(id, 42),
            _ => panic!("esperado PorId"),
        }
    }
}
