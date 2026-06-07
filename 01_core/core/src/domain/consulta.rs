//! Lineage: prompt 00_nucleo/prompt/0056-estagio2_mover_vocabulario_l1.md
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
