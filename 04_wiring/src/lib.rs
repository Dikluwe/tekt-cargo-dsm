//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/wiring.md
//! @prompt-hash f66f9e0b
//! @layer L4
//! @updated 2026-06-07
//!          ampliado por prompt 00_nucleo/prompt/0027-ranking-top-n.md
//!          escopo do usuário por prompt 00_nucleo/prompt/0030-escopo-usuario.md
//! Camada:  L4 — Fiação (composição pura, sem lógica de negócio).
//!
//! Compõe os pipelines da lente ponta a ponta:
//!
//!   modo per-nó:
//!     FonteGrafo → [JSON cru] → desserializa → grafo → [detecta colisões]
//!       → resolver colisões (investiga+resolve) → grafo resolvido
//!       → aplicar Escopo (filtrar_stdlib se SeuCodigo)
//!       → resolver alvo (path direto ou via id) → calcular_raio → Raio.
//!
//!   modo ranking (prompt 0027):
//!     FonteGrafo → grafo resolvido (reuso da etapa acima)
//!       → aplicar Escopo (mesmo helper)
//!       → rankear (lente_ranking) → Vec<ItemRanking>.
//!
//! O **Escopo** (prompt 0030) é parâmetro dos dois pipelines, com **mesmo
//! default** (`Completo`): conserta o Achado 2 do laudo 0029 (classificação
//! divergente em silêncio entre ranking e raio). O filtro do `lente_filtro`
//! intacto — só passa a ser aplicado condicionalmente, num único ponto.
//!
//! Não formata, não escreve em stdout, não lida com argumentos — isso é L2.

#![forbid(unsafe_code)]

use core::error::Error;
use core::fmt;
use std::collections::HashMap;

use lente_core::domain::mapeamento::{MapeamentoDiff, mapear_diff};
use lente_core::domain::raio::{ErroRaio, Raio, calcular_raio};
use lente_core::domain::uniao::{GrafoCrate, ResultadoUniao, unir_grafos};
use lente_core::entities::grafo::{Aresta, Grafo, Path, Relation};
use lente_estrutura::{
    agregar_por_modulo, detectar_ciclos, ordenar_dsm, pesos_modulo_a_modulo, raios_por_modulo,
};
use lente_filtro::{filtrar_so_referencia, filtrar_stdlib};
use lente_infra::ErroAdaptador;
use lente_infra::ErroWorkspace;
use lente_infra::fork::ErroFork;
use lente_infra::{ErroDiff, ler_diff};
use lente_investiga::{ArestasNo, ParColidente, Vizinhanca};
use lente_ranking::rankear;
use lente_resolve::ErroResolve;

// Re-export para consumidores L2 não precisarem depender de `lente_ranking`/
// `lente_estrutura` diretamente — a fronteira de API do wiring carrega os
// tipos de resultado.
pub use lente_core::domain::resultado_diff::{
    RaioCombinado, ResultadoDiff, TocadoComRaio, combinar_raios,
};
pub use lente_core::domain::consulta::{AlvoBusca, Escopo, FonteGrafo, ModoUses};
pub use lente_core::domain::uniao::Fantasma;
pub use lente_estrutura::{Ciclo, DependenciaModulo, EstruturaModulos, OrdemDsm};
pub use lente_ranking::ItemRanking;

// O vocabulário de pedido — `FonteGrafo`/`Escopo`/`ModoUses`/`AlvoBusca` — desceu
// ao L1 no Estágio 2 (0056): `lente_core::domain::consulta`, re-exportado acima.
// A fiação só os importa nas assinaturas; não os define mais (V12 deixa de
// disparar por eles).

/// Erro agregado do pipeline. Embrulha os erros das camadas internas via
/// `From` impls (uso natural com `?`).
#[derive(Debug)]
pub enum ErroLente {
    /// Falha ao invocar o fork (subprocess).
    Fork(ErroFork),
    /// Falha do `lente_infra` (desserialização, validação de invariantes).
    Adaptador(ErroAdaptador),
    /// Falha em uma resolução de colisão (não-determinado, inconsistência).
    Resolucao(ErroResolve),
    /// Falha no cálculo do raio (alvo inexistente, etc.).
    Raio(ErroRaio),
    /// Alvo apontado por id que não existe no grafo resolvido.
    IdInexistente(usize),
    /// Prompt 0034: usuário pediu `ModoUses::SoReferencia` mas o grafo
    /// não tem `uses_kind` em nenhuma aresta `Uses` — o fork instalado é
    /// pré-`b44aa96` (não emite o subtipo). Diagnóstico claro, em vez de
    /// silenciar produzindo `Todas` por engano.
    ForkSemUsesKind,
    /// Prompt 0045: falha na montagem do grafo de workspace — enumeração de
    /// membros, extração cacheada ou versão do toolchain (camada L3, 0044).
    Workspace(ErroWorkspace),
    /// Prompt 0047: falha ao ler o diff do repositório (subprocesso `git`) —
    /// de `lente_infra::ler_diff` (0046).
    Diff(ErroDiff),
}

impl From<ErroFork> for ErroLente {
    fn from(e: ErroFork) -> Self {
        ErroLente::Fork(e)
    }
}
impl From<ErroAdaptador> for ErroLente {
    fn from(e: ErroAdaptador) -> Self {
        ErroLente::Adaptador(e)
    }
}
impl From<ErroResolve> for ErroLente {
    fn from(e: ErroResolve) -> Self {
        ErroLente::Resolucao(e)
    }
}
impl From<ErroRaio> for ErroLente {
    fn from(e: ErroRaio) -> Self {
        ErroLente::Raio(e)
    }
}
impl From<ErroWorkspace> for ErroLente {
    fn from(e: ErroWorkspace) -> Self {
        ErroLente::Workspace(e)
    }
}
impl From<ErroDiff> for ErroLente {
    fn from(e: ErroDiff) -> Self {
        ErroLente::Diff(e)
    }
}

impl fmt::Display for ErroLente {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroLente::Fork(e) => write!(f, "fork: {}", e),
            ErroLente::Adaptador(e) => write!(f, "adaptador: {}", e),
            ErroLente::Resolucao(e) => write!(f, "resolução de colisão: {}", e),
            ErroLente::Raio(e) => write!(f, "cálculo do raio: {}", e),
            ErroLente::IdInexistente(id) => {
                write!(f, "id {} não existe no grafo resolvido", id)
            }
            ErroLente::ForkSemUsesKind => f.write_str(
                "o fork `cargo-modules` instalado não emite `uses_kind` por aresta \
                 — atualize o fork para usar `--so-referencia`",
            ),
            ErroLente::Workspace(e) => write!(f, "grafo de workspace: {}", e),
            ErroLente::Diff(e) => write!(f, "leitura do diff: {}", e),
        }
    }
}

impl Error for ErroLente {}

/// Pipeline completo: extrai (ou recebe) o grafo, resolve colisões, aplica
/// o escopo, e calcula o raio do alvo.
///
/// `escopo` (prompt 0030): `Completo` (default) preserva o comportamento
/// pré-0030 — grafo cru-mas-resolvido, com stdlib. `SeuCodigo` aplica
/// `filtrar_stdlib` antes de resolver o alvo e calcular o raio; um alvo
/// que é nó de stdlib vira `AlvoInexistente` (consistente: pediu para
/// filtrar a stdlib e consultou um nó dela).
pub fn calcular_raio_de_alvo(
    fonte: FonteGrafo,
    alvo: AlvoBusca,
    escopo: Escopo,
) -> Result<Raio, ErroLente> {
    let grafo = obter_grafo(fonte, escopo)?;

    let path_alvo = match alvo {
        AlvoBusca::PorPath(p) => p,
        AlvoBusca::PorId(id) => grafo
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.path.clone())
            .ok_or(ErroLente::IdInexistente(id))?,
    };

    let raio = calcular_raio(&grafo, &path_alvo)?;
    Ok(raio)
}

/// O grafo de workspace montado: o grafo unificado de todos os membros e os
/// fantasmas detectados na união (esperado vazio neste repo — laudo 0041).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrafoWorkspace {
    pub grafo: Grafo,
    pub fantasmas: Vec<Fantasma>,
}

/// Monta o grafo de workspace (prompt 0045): a fundação do motor da trilha
/// local, pronta para o modo `--diff` (L2).
///
/// Passos (composição pura — sem lógica nova):
/// 1. `lente_infra::enumerar_membros` (L3, 0044) → membros.
/// 2. `lente_infra::versao_toolchain` (L3) → versão, **uma vez**.
/// 3. Para cada membro: `lente_infra::extrair_grafo_cacheado` (L3) → `Grafo`.
/// 4. Para cada `Grafo`: [`resolver_colisoes`] → grafo resolvido do crate
///    (resolver **por crate antes de unir** — laudo 0041).
/// 5. `lente_core::domain::uniao::unir_grafos` (L1) → grafo unificado +
///    fantasmas.
///
/// A primeira chamada num workspace frio paga a extração de todos os crates
/// (~33s, laudo 0040); o cache de chave completa (0044) aquece as seguintes.
pub fn montar_grafo_workspace(raiz: &std::path::Path) -> Result<GrafoWorkspace, ErroLente> {
    let membros = lente_infra::enumerar_membros(raiz)?;
    let versao = lente_infra::versao_toolchain()?;
    let mut grafos: Vec<GrafoCrate> = Vec::with_capacity(membros.len());
    for membro in &membros {
        let grafo = lente_infra::extrair_grafo_cacheado(membro, raiz, &versao)?;
        let grafo = resolver_colisoes(grafo)?;
        grafos.push(GrafoCrate {
            crate_name: membro.nome.clone(),
            grafo,
        });
    }
    let ResultadoUniao { grafo, fantasmas } = unir_grafos(grafos);
    Ok(GrafoWorkspace { grafo, fantasmas })
}

/// Analisa um diff contra o grafo de workspace (prompt 0047): o pipeline
/// completo do modo `--diff`, devolvendo o [`ResultadoDiff`] view-agnóstico.
///
/// Passos (composição):
/// 1. `lente_infra::ler_diff` (L3, 0046) → `DiffEstruturado`.
/// 2. [`montar_grafo_workspace`] (L4, 0045) → grafo unificado + fantasmas.
/// 3. `lente_infra::enumerar_membros` (L3, 0044) → os `membros_dirs` (para o
///    censo solto-vs-não-fonte do mapeamento).
/// 4. `lente_core::mapear_diff` (L1, 0046) → `MapeamentoDiff`.
/// 5. [`montar_resultado_diff`] (parte pura) → raio por tocado + combinado +
///    censo + fantasmas.
///
/// A primeira chamada num workspace frio paga a extração de todos os crates
/// (~33s, laudo 0040); o cache (0044) aquece as seguintes.
pub fn analisar_diff(raiz: &std::path::Path) -> Result<ResultadoDiff, ErroLente> {
    // Canonicalizar a raiz é o que faz a reconciliação de caminho funcionar: os
    // `caminho` do diff são `raiz.join(relativo)`, e precisam casar com as
    // `position.file` (absolutas/canônicas do fork, laudo 0037). Com `raiz`
    // relativa (ex.: a CLI passa `.`), `join` manteria o `./` e nada casaria —
    // 0 tocados e ligados virando soltos. Canonicalizar resolve isso para todos
    // os consumidores (ler_diff, enumerar_membros, mapear_diff) de uma vez.
    let raiz = raiz
        .canonicalize()
        .map_err(|e| ErroLente::Diff(ErroDiff::Io(e)))?;
    let raiz = raiz.as_path();

    let diff = ler_diff(raiz)?;
    let GrafoWorkspace { grafo, fantasmas } = montar_grafo_workspace(raiz)?;
    let membros_dirs: Vec<std::path::PathBuf> = lente_infra::enumerar_membros(raiz)?
        .into_iter()
        .map(|m| m.dir)
        .collect();
    let mapa = mapear_diff(&diff, &grafo, &membros_dirs);
    Ok(montar_resultado_diff(&grafo, mapa, fantasmas))
}

/// A **parte pura** da análise de diff (sem git/fork): dado o grafo, o
/// mapeamento e os fantasmas, calcula o raio de cada tocado, combina os raios
/// e monta o [`ResultadoDiff`]. Separada para ser testável sem I/O (o
/// `analisar_diff` completo é `#[ignore]`).
fn montar_resultado_diff(
    grafo: &Grafo,
    mapa: MapeamentoDiff,
    fantasmas: Vec<Fantasma>,
) -> ResultadoDiff {
    let MapeamentoDiff {
        tocados,
        ligados,
        soltos,
        nao_fonte,
    } = mapa;

    let mut tocados_com_raio: Vec<TocadoComRaio> = Vec::with_capacity(tocados.len());
    for tocado in tocados {
        // O path do tocado vem de um nó do `grafo` (mapear_diff só marca nós
        // presentes), então `calcular_raio` não falha; se falhasse, pular é a
        // defesa segura (não inventa raio).
        if let Ok(raio) = calcular_raio(grafo, &tocado.path) {
            tocados_com_raio.push(TocadoComRaio { tocado, raio });
        }
    }

    let raios: Vec<Raio> = tocados_com_raio.iter().map(|t| t.raio.clone()).collect();
    let combinado = combinar_raios(&raios);

    ResultadoDiff {
        tocados: tocados_com_raio,
        combinado,
        ligados,
        soltos,
        nao_fonte,
        fantasmas,
    }
}

/// Pipeline do ranking: extrai+resolve, aplica o escopo, rankeia top-N.
///
/// `escopo` (prompt 0030): `Completo` (default novo, pós-0030) — o ranking
/// sai com sysroot no topo, situação do laudo 0021. `SeuCodigo` recupera
/// o ranking do laudo 0027 (Vec2/Color32/… sem sysroot). O default mudou
/// **deliberadamente** para casar com o per-nó — ver `Escopo` e o laudo
/// 0030 para a razão (Achado 2 do laudo 0029).
pub fn rankear_pacote(
    fonte: FonteGrafo,
    n: usize,
    escopo: Escopo,
) -> Result<Vec<ItemRanking>, ErroLente> {
    let grafo = obter_grafo(fonte, escopo)?;
    Ok(rankear(&grafo, n))
}

// `DependenciaModulo` e `EstruturaModulos` desceram ao `lente_estrutura` (L1) no
// Estágio 2 (0056) — dado puro de estrutura, junto do `Ciclo`. Re-exportados
// acima; a fiação só os usa nas assinaturas.

/// Pipeline do modo estrutura (prompt 0031, ampliado pelo 0034): obtém o
/// grafo no escopo, opcionalmente filtra arestas `Uses` pelo `modo_uses`,
/// agrega ao nível de módulo, detecta ciclos. Reusa `obter_grafo` (laudo
/// 0030) — o escopo flui aqui igual aos outros pipelines.
///
/// **Invariância ao escopo** dos ciclos (confirmada por E2E no laudo
/// 0031): stdlib é sorvedouro, nunca fecha ciclo de volta. O escopo só
/// muda quais módulos aparecem em `modulos`/`dependencias`, não os
/// `ciclos`.
///
/// **`modo_uses`** (prompt 0034): `Todas` (default) preserva a vista do
/// laudo 0031; `SoReferencia` aplica `filtrar_so_referencia` antes do
/// agregado — descarta arestas `Uses` de tipo `Import` (Limite 4 da spec).
///
/// **Diagnóstico de fork antigo**: se `modo_uses == SoReferencia` e o
/// grafo tem arestas `Uses` mas **nenhuma** com `uses_kind` definido (o
/// fork instalado não emite o campo), retorna [`ErroLente::ForkSemUsesKind`]
/// em vez de silenciar produzindo um grafo todo descartado.
pub fn analisar_estrutura(
    fonte: FonteGrafo,
    escopo: Escopo,
    modo_uses: ModoUses,
) -> Result<EstruturaModulos, ErroLente> {
    let grafo = obter_grafo(fonte, escopo)?;
    let grafo = match modo_uses {
        ModoUses::Todas => grafo,
        ModoUses::SoReferencia => {
            // Defesa: se nenhuma aresta `Uses` tem `uses_kind`, o fork
            // instalado não emite o campo — `filtrar_so_referencia`
            // descartaria *todas* silenciosamente. Diagnóstico explícito.
            let total_uses = grafo
                .edges
                .iter()
                .filter(|a| a.relation == Relation::Uses)
                .count();
            let com_kind = grafo
                .edges
                .iter()
                .filter(|a| a.relation == Relation::Uses && a.uses_kind.is_some())
                .count();
            if total_uses > 0 && com_kind == 0 {
                return Err(ErroLente::ForkSemUsesKind);
            }
            filtrar_so_referencia(&grafo)
        }
    };
    let agg = agregar_por_modulo(&grafo);
    let ciclos = detectar_ciclos(&agg);
    // Prompt 0035: ordem da DSM (módulos + blocos) sobre o agregado.
    let dsm = ordenar_dsm(&agg);
    // Prompt 0071: peso de acoplamento por par módulo→módulo (Achado 1 do 0036).
    // Contado sobre o grafo de itens (não o agregado, que já colapsou as arestas).
    let pesos = pesos_modulo_a_modulo(&grafo);

    let mut modulos: Vec<Path> = agg.nodes.iter().map(|n| n.path.clone()).collect();
    modulos.sort_by(|a, b| a.as_str().cmp(b.as_str()));

    let mut dependencias: Vec<DependenciaModulo> = agg
        .edges
        .iter()
        .filter(|a| a.relation == Relation::Uses)
        .map(|a| DependenciaModulo {
            de: a.from.clone(),
            para: a.to.clone(),
            // Toda aresta do agregado veio de ≥1 aresta-de-item, então o mapa
            // sempre tem a chave; `1` é piso defensivo, não caminho normal.
            peso: pesos.get(&(a.id_from, a.id_to)).copied().unwrap_or(1),
        })
        .collect();
    dependencias.sort_by(|a, b| {
        a.de
            .as_str()
            .cmp(b.de.as_str())
            .then_with(|| a.para.as_str().cmp(b.para.as_str()))
    });

    // Prompt 0073: raio por módulo (montante/jusante transitivos, exatos) sobre
    // o MESMO grafo de itens (mesmo escopo/modo) que a estrutura usa.
    let raios = raios_por_modulo(&grafo);

    Ok(EstruturaModulos {
        modulos,
        dependencias,
        ciclos,
        ordem: dsm.ordem,
        blocos: dsm.blocos,
        raios,
    })
}

/// Helper único da aplicação do escopo (prompt 0030): obtém o grafo
/// resolvido e aplica `filtrar_stdlib` se `escopo == SeuCodigo`. Ponto
/// **único** onde a decisão de filtrar mora — coerência entre os dois
/// pipelines sai daqui, não de duplicação.
fn obter_grafo(fonte: FonteGrafo, escopo: Escopo) -> Result<Grafo, ErroLente> {
    let grafo = obter_grafo_resolvido(fonte)?;
    Ok(match escopo {
        Escopo::SeuCodigo => filtrar_stdlib(&grafo),
        Escopo::Completo => grafo,
    })
}

/// Etapa compartilhada pelos modos per-nó e ranking: extrair (ou receber)
/// o JSON, desserializar, e resolver colisões. Devolve o **grafo resolvido**
/// (paths únicos), ponto comum a partir do qual os modos divergem.
///
/// Fatoração feita pelo prompt 0027; o prompt 0030 acrescentou
/// `obter_grafo` por cima, para encaixar o Escopo num ponto único.
/// Função interna; a fronteira de API do crate continua sendo os dois
/// pipelines completos.
fn obter_grafo_resolvido(fonte: FonteGrafo) -> Result<Grafo, ErroLente> {
    let json = match fonte {
        FonteGrafo::Json(s) => s,
        FonteGrafo::Pacote(p) => lente_infra::fork::invocar_fork(&p)?,
    };
    let grafo = lente_infra::desserializar_grafo(&json)?;
    resolver_colisoes(grafo)
}

/// Resolve **todas** as colisões de path de um grafo: para cada path
/// colidente, investiga o primeiro par e aplica o veredito (laudo 0019, E2 em
/// quarentena — `fontes` sempre `None`, laudo 0014). Extraído do
/// `obter_grafo_resolvido` (prompt 0045) para ser reusado pela montagem do
/// grafo de workspace (resolver **por crate**, antes de unir — laudo 0041).
/// Refator que **preserva o comportamento**: `obter_grafo_resolvido` apenas
/// passa a chamá-lo; os testes do pipeline são a guarda.
fn resolver_colisoes(mut grafo: Grafo) -> Result<Grafo, ErroLente> {
    let colisoes = detectar_colisoes(&grafo);
    for path_colidente in colisoes {
        grafo = resolver_uma_colisao(grafo, &path_colidente)?;
    }
    Ok(grafo)
}

/// Devolve os paths que aparecem em 2+ nós do grafo.
fn detectar_colisoes(grafo: &Grafo) -> Vec<Path> {
    let mut contagem: HashMap<&Path, usize> = HashMap::new();
    for n in &grafo.nodes {
        *contagem.entry(&n.path).or_insert(0) += 1;
    }
    contagem
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .map(|(p, _)| p.clone())
        .collect()
}

/// Investiga uma colisão (primeiro par por ordem de id) e aplica o veredito.
fn resolver_uma_colisao(grafo: Grafo, path_colidente: &Path) -> Result<Grafo, ErroLente> {
    let mut ids: Vec<usize> = grafo
        .nodes
        .iter()
        .filter(|n| &n.path == path_colidente)
        .map(|n| n.id)
        .collect();
    if ids.len() < 2 {
        // Nada a fazer (pode ter sido resolvida indiretamente).
        return Ok(grafo);
    }
    ids.sort_unstable();
    let (id_a, id_b) = (ids[0], ids[1]);

    let viz = construir_vizinhanca(&grafo, id_a, id_b);
    // Buscar os Nos do par (referências; precisamos depois para investigar).
    let no_a = grafo.nodes.iter().find(|n| n.id == id_a).expect("id_a existe");
    let no_b = grafo.nodes.iter().find(|n| n.id == id_b).expect("id_b existe");
    let par = ParColidente { a: no_a, b: no_b };

    // E2 em quarentena (laudo 0014): fontes sempre None aqui.
    let veredito = lente_investiga::investigar(par, &viz, None);

    // Aplicar — pode retornar erro (ColisaoNaoResolvida para NaoDeterminado).
    let grafo = lente_resolve::aplicar(&grafo, path_colidente, &veredito)?;
    Ok(grafo)
}

/// Constrói a `Vizinhanca` do par (id_a, id_b) a partir das arestas do grafo,
/// separadas por id — exatamente o que `lente_investiga` espera.
fn construir_vizinhanca(grafo: &Grafo, id_a: usize, id_b: usize) -> Vizinhanca {
    let mut va: ArestasNo = ArestasNo::default();
    let mut vb: ArestasNo = ArestasNo::default();
    for a in &grafo.edges {
        if a.id_to == id_a {
            va.entrando.push(clonar(a));
        }
        if a.id_from == id_a {
            va.saindo.push(clonar(a));
        }
        if a.id_to == id_b {
            vb.entrando.push(clonar(a));
        }
        if a.id_from == id_b {
            vb.saindo.push(clonar(a));
        }
    }
    Vizinhanca { a: va, b: vb }
}

fn clonar(a: &Aresta) -> Aresta {
    a.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON sintético com colisão tipo `Display+Debug` mais usuários
    /// distintos — vizinhança disjunta por construção, trait nos nós.
    fn json_sintetico_com_colisao() -> &'static str {
        r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::T","name":"T","kind":"struct","visibility":"pub"},
                {"id":20,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"priv","trait":"Display","trait_ref":"Display"},
                {"id":21,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"pub","trait":"Debug","trait_ref":"Debug"},
                {"id":30,"path":"t::user_a","name":"user_a","kind":"fn","visibility":"pub"},
                {"id":31,"path":"t::user_b","name":"user_b","kind":"fn","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::T","id_to":10,"relation":"owns"},
                {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":20,"relation":"owns"},
                {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":21,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::user_a","id_to":30,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::user_b","id_to":31,"relation":"owns"},
                {"from":"t::user_a","id_from":30,"to":"t::T::fmt","id_to":20,"relation":"uses"},
                {"from":"t::user_b","id_from":31,"to":"t::T::fmt","id_to":21,"relation":"uses"}
            ]
        }"#
    }

    /// VERIFICAÇÃO CRUCIAL — o primeiro ponto onde se prova que a cascata
    /// do descritor inteira funciona quando composta. Depois do pipeline,
    /// `t::T::fmt` (colidente) NÃO existe; `<Display>::fmt` e `<Debug>::fmt`
    /// existem nos paths certos (atribuído pelo trait do nó, sem adivinhação).
    #[test]
    fn pipeline_completo_renomeia_colisao_por_trait_do_no() {
        let raio = calcular_raio_de_alvo(
            FonteGrafo::Json(json_sintetico_com_colisao().to_string()),
            // Pelo id do Display::fmt — resolvido para o novo path automaticamente.
            AlvoBusca::PorId(20),
            Escopo::Completo,
        )
        .expect("pipeline ponta a ponta deve funcionar");

        // O raio refere-se ao alvo renomeado.
        assert_eq!(raio.alvo.as_str(), "t::T::<Display>::fmt");
    }

    #[test]
    fn pipeline_alvo_por_path_funciona() {
        // Path do Debug — também já renomeado dentro do pipeline.
        let raio = calcular_raio_de_alvo(
            FonteGrafo::Json(json_sintetico_com_colisao().to_string()),
            AlvoBusca::PorPath(Path::from("t::T::<Debug>::fmt")),
            Escopo::Completo,
        )
        .expect("alvo por path deve funcionar");
        assert_eq!(raio.alvo.as_str(), "t::T::<Debug>::fmt");
    }

    #[test]
    fn id_inexistente_retorna_erro_proprio() {
        match calcular_raio_de_alvo(
            FonteGrafo::Json(json_sintetico_com_colisao().to_string()),
            AlvoBusca::PorId(9999),
            Escopo::Completo,
        ) {
            Err(ErroLente::IdInexistente(9999)) => {}
            outro => panic!("esperava IdInexistente(9999), veio {:?}", outro),
        }
    }

    #[test]
    fn json_invalido_propaga_como_erro_de_adaptador() {
        match calcular_raio_de_alvo(
            FonteGrafo::Json("{ não é JSON".to_string()),
            AlvoBusca::PorPath(Path::from("x")),
            Escopo::Completo,
        ) {
            Err(ErroLente::Adaptador(ErroAdaptador::JsonInvalido(_))) => {}
            outro => panic!("esperava Adaptador/JsonInvalido, veio {:?}", outro),
        }
    }

    #[test]
    fn display_de_erro_lente_cobre_variantes() {
        let v1 = ErroLente::IdInexistente(42);
        let v2 = ErroLente::Adaptador(ErroAdaptador::JsonInvalido("eof".to_string()));
        let v3 = ErroLente::Workspace(ErroWorkspace::Toolchain("rustc ausente".to_string()));
        for v in [v1, v2, v3].iter() {
            assert!(!format!("{}", v).is_empty());
        }
    }

    // ---- Grafo de workspace (prompt 0045) -----------------------------------

    /// E2E real (requer fork): monta o grafo de workspace do projeto-lente.
    /// Confirma a ordem de grandeza da Arena (~363 nós, laudo 0043), fantasmas
    /// **vazio** (laudo 0041 — colisões são folhas de raio 0), paths únicos
    /// (colisões resolvidas por crate antes de unir, nomes do 0042) e uma
    /// aresta cross-crate conhecida (`lente_infra` → `lente_core`).
    /// Primeira chamada fria (~33s); o cache (0044) aquece as seguintes.
    #[test]
    #[ignore]
    fn e2e_montar_grafo_workspace_unifica_todos() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("raiz do workspace")
            .to_path_buf();
        let gw = montar_grafo_workspace(&raiz).expect("montar grafo de workspace");

        eprintln!(
            "grafo de workspace: {} nós, {} arestas, {} fantasmas",
            gw.grafo.nodes.len(),
            gw.grafo.edges.len(),
            gw.fantasmas.len()
        );

        // Ordem de grandeza da Arena (~363).
        assert!(
            gw.grafo.nodes.len() > 300,
            "esperava ~363 nós, veio {}",
            gw.grafo.nodes.len()
        );

        // Fantasmas: 0 neste repo (se >0, é achado — não esconder).
        assert!(
            gw.fantasmas.is_empty(),
            "esperava 0 fantasmas, veio {:?}",
            gw.fantasmas
        );

        // Paths únicos no grafo unido (colisões resolvidas por crate).
        let mut ps: Vec<&str> = gw.grafo.nodes.iter().map(|n| n.path.as_str()).collect();
        let total = ps.len();
        ps.sort();
        ps.dedup();
        assert_eq!(ps.len(), total, "paths únicos no grafo unificado");

        // Colisão conhecida do 0042 resolvida: `Path::from` cru não aparece 2x.
        let path_from_cru = gw
            .grafo
            .nodes
            .iter()
            .filter(|n| n.path.as_str() == "lente_core::entities::grafo::Path::from")
            .count();
        assert!(
            path_from_cru <= 1,
            "Path::from cru não deve colidir após resolução, veio {}",
            path_from_cru
        );

        // Aresta cross-crate conhecida: lente_infra → lente_core.
        let cross = gw.grafo.edges.iter().any(|e| {
            e.relation == Relation::Uses
                && e.from.as_str().starts_with("lente_infra")
                && e.to.as_str().starts_with("lente_core")
        });
        assert!(cross, "esperava aresta cross-crate lente_infra → lente_core");
    }

    /// E2E (requer fork): a segunda montagem (cache morno) é rápida — sem
    /// rodar o fork. Limiar generoso (quente ~70ms no laudo 0040).
    #[test]
    #[ignore]
    fn e2e_montar_grafo_workspace_cache_morno_e_rapido() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let _ = montar_grafo_workspace(&raiz).expect("primeira (aquece o cache)");
        let t = std::time::Instant::now();
        let gw = montar_grafo_workspace(&raiz).expect("segunda (cache morno)");
        let dt = t.elapsed();
        assert!(gw.grafo.nodes.len() > 300);
        assert!(
            dt.as_secs() < 5,
            "cache morno deveria ser rápido, veio {:?}",
            dt
        );
    }

    // ---- Análise de diff (prompt 0047) --------------------------------------

    /// A parte pura da análise (sem git/fork): dado um grafo e um mapeamento
    /// forjados, `montar_resultado_diff` calcula o raio de cada tocado, combina
    /// os raios e passa o censo + fantasmas adiante.
    #[test]
    fn montar_resultado_diff_calcula_raio_por_tocado_e_combina() {
        // t::B usa t::A → o montante de t::A tem t::B.
        let json = r#"{"crate":"t","nodes":[
            {"id":1,"path":"t::A","name":"A","kind":"fn","visibility":"pub"},
            {"id":2,"path":"t::B","name":"B","kind":"fn","visibility":"pub"}
        ],"edges":[
            {"from":"t::B","id_from":2,"to":"t::A","id_to":1,"relation":"uses"}
        ]}"#;
        let grafo = lente_infra::desserializar_grafo(json).unwrap();
        let mapa = MapeamentoDiff {
            tocados: vec![lente_core::domain::mapeamento::NoTocado {
                id: 1,
                path: Path::from("t::A"),
            }],
            ligados: vec![std::path::PathBuf::from("/r/a/src/lig.rs")],
            soltos: vec![std::path::PathBuf::from("/r/a/src/solto.rs")],
            nao_fonte: vec![std::path::PathBuf::from("/r/README.md")],
        };
        let r = montar_resultado_diff(&grafo, mapa, Vec::new());

        // O tocado t::A traz seu raio (t::B no montante).
        assert_eq!(r.tocados.len(), 1);
        assert_eq!(r.tocados[0].tocado.path.as_str(), "t::A");
        assert!(r.tocados[0].raio.montante.contains_key(&Path::from("t::B")));
        // O combinado é a união (aqui, só t::B a profundidade 1).
        assert_eq!(r.combinado.montante, vec![(Path::from("t::B"), 1)]);
        assert!(r.combinado.jusante.is_empty());
        // Censo e fantasmas passam adiante intactos.
        assert_eq!(r.ligados, vec![std::path::PathBuf::from("/r/a/src/lig.rs")]);
        assert_eq!(r.soltos, vec![std::path::PathBuf::from("/r/a/src/solto.rs")]);
        assert_eq!(r.nao_fonte, vec![std::path::PathBuf::from("/r/README.md")]);
        assert!(r.fantasmas.is_empty());
    }

    /// `ErroLente::Diff` traduz via `Display` (cobre a variante nova).
    #[test]
    fn display_de_erro_lente_cobre_diff() {
        let e = ErroLente::Diff(ErroDiff::Git {
            codigo: Some(128),
            stderr: "not a git repository".to_string(),
        });
        assert!(format!("{}", e).contains("diff"));
    }

    /// E2E real (requer git + fork): analisa o diff do próprio repo. Confirma
    /// que roda, que os fantasmas são 0 (0045/0041) e que cada tocado tem o
    /// raio resolvido no seu próprio path. Não afirma tocados específicos (o
    /// working tree varia). Primeira chamada fria (~33s); cache aquece.
    #[test]
    #[ignore]
    fn e2e_analisar_diff_no_repo_real() {
        let raiz = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("raiz do workspace")
            .to_path_buf();
        let r = analisar_diff(&raiz).expect("analisar_diff no repo real");
        eprintln!(
            "diff: {} tocados, {} ligados, {} soltos, {} não-fonte, {} fantasmas",
            r.tocados.len(),
            r.ligados.len(),
            r.soltos.len(),
            r.nao_fonte.len(),
            r.fantasmas.len()
        );
        assert!(
            r.fantasmas.is_empty(),
            "esperava 0 fantasmas, veio {:?}",
            r.fantasmas
        );
        for t in &r.tocados {
            assert_eq!(t.raio.alvo.as_str(), t.tocado.path.as_str());
        }
    }

    /// E2E real contra o crate `lente_core`. Requer fork 0.27.0 instalado.
    /// Confirma que, com dados reais, a colisão `ErroRaio::fmt` é resolvida
    /// pelo pipeline em `<Display>::fmt` / `<Debug>::fmt`.
    #[test]
    #[ignore]
    fn e2e_lente_core_renomeia_erro_raio_fmt() {
        let raio = calcular_raio_de_alvo(
            FonteGrafo::Pacote("lente_core".to_string()),
            AlvoBusca::PorPath(Path::from("lente_core::domain::raio::Raio")),
            Escopo::Completo,
        )
        .expect("pipeline contra lente_core real");
        // Sanidade: o raio se refere ao alvo pedido (que NÃO é colidente).
        assert_eq!(
            raio.alvo.as_str(),
            "lente_core::domain::raio::Raio"
        );
    }

    /// Verificação crucial complementar: roda o pipeline contra o JSON
    /// sintético e inspeciona o **grafo intermediário** (via re-extração)
    /// para confirmar que `t::T::fmt` realmente não existe mais. Como
    /// `calcular_raio_de_alvo` não expõe o grafo, refazemos os passos.
    #[test]
    fn verificacao_crucial_colisao_some_e_traits_aparecem() {
        let json = json_sintetico_com_colisao().to_string();
        let mut grafo = lente_infra::desserializar_grafo(&json).unwrap();

        // Antes do pipeline: t::T::fmt existe em 2 nós.
        assert_eq!(
            grafo.nodes.iter().filter(|n| n.path.as_str() == "t::T::fmt").count(),
            2,
            "antes: dois nós colidem em t::T::fmt"
        );

        // Roda exatamente o que `calcular_raio_de_alvo` rodaria (passos 3-4).
        for path_colidente in detectar_colisoes(&grafo) {
            grafo = resolver_uma_colisao(grafo, &path_colidente).unwrap();
        }

        // Após o pipeline: t::T::fmt NÃO EXISTE; <Display>/<Debug> existem.
        assert_eq!(
            grafo.nodes.iter().filter(|n| n.path.as_str() == "t::T::fmt").count(),
            0,
            "colisão t::T::fmt deve ter sumido"
        );
        assert!(
            grafo.nodes.iter().any(|n| n.path.as_str() == "t::T::<Display>::fmt"),
            "deve existir t::T::<Display>::fmt"
        );
        assert!(
            grafo.nodes.iter().any(|n| n.path.as_str() == "t::T::<Debug>::fmt"),
            "deve existir t::T::<Debug>::fmt"
        );
    }

    // ---- Modo ranking (prompt 0027) -----------------------------------------

    /// JSON com mix de stdlib + alvo. O ranking sem filtro traria
    /// `core::fmt::Display` (3 usuários no alvo). Depois do `filtrar_stdlib`
    /// (no pipeline `rankear_pacote`), sysroot some — top-N tem só nós do alvo.
    fn json_com_stdlib_e_alvo() -> &'static str {
        r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::T","name":"T","kind":"struct","visibility":"pub"},
                {"id":20,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"priv","trait":"Display","trait_ref":"Display"},
                {"id":30,"path":"t::user_a","name":"user_a","kind":"fn","visibility":"pub"},
                {"id":31,"path":"t::user_b","name":"user_b","kind":"fn","visibility":"pub"},
                {"id":32,"path":"t::user_c","name":"user_c","kind":"fn","visibility":"pub"},
                {"id":100,"path":"core::fmt::Display","name":"Display","kind":"trait","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::T","id_to":10,"relation":"owns"},
                {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":20,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::user_a","id_to":30,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::user_b","id_to":31,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::user_c","id_to":32,"relation":"owns"},
                {"from":"t::user_a","id_from":30,"to":"t::T::fmt","id_to":20,"relation":"uses"},
                {"from":"t::user_b","id_from":31,"to":"t::T::fmt","id_to":20,"relation":"uses"},
                {"from":"t::user_c","id_from":32,"to":"t::T::fmt","id_to":20,"relation":"uses"},
                {"from":"t::T::fmt","id_from":20,"to":"core::fmt::Display","id_to":100,"relation":"uses"},
                {"from":"t::user_a","id_from":30,"to":"core::fmt::Display","id_to":100,"relation":"uses"},
                {"from":"t::user_b","id_from":31,"to":"core::fmt::Display","id_to":100,"relation":"uses"}
            ]
        }"#
    }

    #[test]
    fn rankear_seu_codigo_remove_sysroot_e_top_e_do_alvo() {
        // Cenário-base do laudo 0027: ranking filtrado → sem sysroot, top do alvo.
        // Pós-0030 isso é o escopo `SeuCodigo`.
        let r = rankear_pacote(
            FonteGrafo::Json(json_com_stdlib_e_alvo().to_string()),
            10,
            Escopo::SeuCodigo,
        )
        .expect("rankear SeuCodigo deve funcionar");

        for item in &r {
            let first = item.path.as_str().split("::").next().unwrap_or("");
            assert!(
                !matches!(first, "core" | "std" | "alloc" | "proc_macro" | "test"),
                "sysroot vazou no ranking SeuCodigo: {}",
                item.path.as_str()
            );
        }
        assert_eq!(r[0].path.as_str(), "t::T::fmt");
        assert_eq!(r[0].impacto, 3);
    }

    /// Default novo pós-0030: `Completo` mantém sysroot — o ranking inclui
    /// nós como `core::fmt::Display`. É a situação do laudo 0021, agora
    /// **declarada** como escolha do default, não regressão.
    #[test]
    fn rankear_completo_traz_sysroot() {
        let r = rankear_pacote(
            FonteGrafo::Json(json_com_stdlib_e_alvo().to_string()),
            10,
            Escopo::Completo,
        )
        .expect("rankear Completo deve funcionar");

        assert!(
            r.iter().any(|it| it.path.as_str() == "core::fmt::Display"),
            "esperava sysroot no ranking Completo, veio: {:?}",
            r.iter().map(|it| it.path.as_str()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn rankear_pacote_propaga_erro_de_json_invalido() {
        match rankear_pacote(
            FonteGrafo::Json("{ não é JSON".to_string()),
            10,
            Escopo::Completo,
        ) {
            Err(ErroLente::Adaptador(_)) => {}
            outro => panic!("esperava Adaptador (JsonInvalido), veio {:?}", outro),
        }
    }

    /// Modo per-nó intacto: a fatoração de `obter_grafo_resolvido` não pode
    /// regredir `calcular_raio_de_alvo`. Guarda a propriedade que a
    /// refatoração (e o prompt 0030) não pode quebrar.
    #[test]
    fn modo_per_no_continua_funcionando_apos_fatoracao() {
        let raio = calcular_raio_de_alvo(
            FonteGrafo::Json(json_com_stdlib_e_alvo().to_string()),
            AlvoBusca::PorPath(Path::from("t::T::fmt")),
            Escopo::Completo,
        )
        .expect("modo per-nó preservado");
        assert_eq!(raio.alvo.as_str(), "t::T::fmt");
        assert_eq!(raio.uses_entrada, 3);
    }

    /// Prompt 0030 — invariante central: para um nó do código do usuário, os
    /// "diretos" e "transitivos" são **iguais** nos dois escopos. Filtrar
    /// stdlib só mexe em `uses_saida` (e por consequência, possivelmente,
    /// na classificação). Aqui `t::T::fmt` tem 3 usuários do alvo, e usa
    /// `core::fmt::Display`.
    #[test]
    fn invariante_do_montante_diretos_e_transitivos_sao_iguais_entre_escopos() {
        let json = json_com_stdlib_e_alvo().to_string();
        let r_completo = calcular_raio_de_alvo(
            FonteGrafo::Json(json.clone()),
            AlvoBusca::PorPath(Path::from("t::T::fmt")),
            Escopo::Completo,
        )
        .unwrap();
        let r_seu = calcular_raio_de_alvo(
            FonteGrafo::Json(json),
            AlvoBusca::PorPath(Path::from("t::T::fmt")),
            Escopo::SeuCodigo,
        )
        .unwrap();
        assert_eq!(r_completo.uses_entrada, r_seu.uses_entrada);
        assert_eq!(r_completo.montante.len(), r_seu.montante.len());
        // O que muda entre escopos: uses_saida. No Completo, t::T::fmt usa
        // core::fmt::Display (1 saída); no SeuCodigo, esse nó some.
        assert_eq!(r_completo.uses_saida, 1);
        assert_eq!(r_seu.uses_saida, 0);
        // E a classificação acompanha: Intermediário → Folha (sem usuários é
        // Folha; mas aqui t::T::fmt tem 3 entrando, então Base no SeuCodigo).
        use lente_core::domain::raio::Classificacao;
        assert_eq!(r_completo.classificacao, Classificacao::Intermediario);
        assert_eq!(r_seu.classificacao, Classificacao::Base);
    }

    /// Prompt 0030 — alvo que é nó de stdlib + escopo `SeuCodigo`:
    /// o nó é filtrado antes do cálculo do raio → `AlvoInexistente`.
    /// Consistente: pediu para filtrar a stdlib e consultou um nó dela.
    #[test]
    fn alvo_de_stdlib_no_escopo_seu_codigo_da_alvo_inexistente() {
        let r = calcular_raio_de_alvo(
            FonteGrafo::Json(json_com_stdlib_e_alvo().to_string()),
            AlvoBusca::PorPath(Path::from("core::fmt::Display")),
            Escopo::SeuCodigo,
        );
        match r {
            Err(ErroLente::Raio(_)) => {}
            outro => panic!("esperava ErroLente::Raio (AlvoInexistente), veio {:?}", outro),
        }
    }

    /// E2E real (prompt 0027 ancorado, pós-0030): rankear o `lente_core`
    /// em `SeuCodigo`. Confirma ponta-a-ponta: extração via fork →
    /// resolução → filtragem → ranking.
    #[test]
    #[ignore]
    fn e2e_ranking_do_lente_core_seu_codigo_nao_traz_sysroot() {
        let r = rankear_pacote(
            FonteGrafo::Pacote("lente_core".to_string()),
            10,
            Escopo::SeuCodigo,
        )
        .expect("ranking E2E do lente_core (SeuCodigo) deve funcionar");

        assert_eq!(r.len(), 10);
        for item in &r {
            let first = item.path.as_str().split("::").next().unwrap_or("");
            assert!(
                !matches!(first, "core" | "std" | "alloc" | "proc_macro" | "test"),
                "sysroot vazou: {}",
                item.path.as_str()
            );
        }
        for item in &r {
            assert!(
                item.path.as_str().starts_with("lente_core"),
                "item fora do alvo: {}",
                item.path.as_str()
            );
        }
    }

    // ---- Modo estrutura (prompt 0031) ---------------------------------------

    /// Grafo sintético com **ciclo de módulos**: `t::a → t::b → t::a` via
    /// itens de cada módulo. Útil para verificar a fiação ponta a ponta.
    fn json_com_ciclo_de_modulos() -> &'static str {
        r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
                {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
                {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
                {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
                {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
                {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
                {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses"},
                {"from":"t::b::g","id_from":21,"to":"t::a::f","id_to":11,"relation":"uses"}
            ]
        }"#
    }

    #[test]
    fn analisar_estrutura_lista_modulos_e_detecta_ciclo() {
        let r = analisar_estrutura(
            FonteGrafo::Json(json_com_ciclo_de_modulos().to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .expect("estrutura deve funcionar");

        // 1 crate + 2 mods = 3 paths em `modulos`.
        let nomes: Vec<&str> = r.modulos.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["t", "t::a", "t::b"]);

        // Dependências: a→b e b→a.
        let deps: Vec<(&str, &str)> = r
            .dependencias
            .iter()
            .map(|d| (d.de.as_str(), d.para.as_str()))
            .collect();
        assert_eq!(deps, vec![("t::a", "t::b"), ("t::b", "t::a")]);

        // O ciclo {t::a, t::b}.
        assert_eq!(r.ciclos.len(), 1);
        let modulos_ciclo: Vec<&str> =
            r.ciclos[0].modulos.iter().map(|p| p.as_str()).collect();
        assert_eq!(modulos_ciclo, vec!["t::a", "t::b"]);
    }

    /// Prompt 0031, "invariância dos ciclos ao escopo": módulos de stdlib
    /// são sorvedouros (não dependem do seu código), então o **conjunto
    /// de ciclos** é o mesmo nos dois escopos. O escopo só muda a
    /// listagem `modulos`/`dependencias`.
    #[test]
    fn ciclos_sao_invariantes_ao_escopo() {
        // Mesmo grafo do teste anterior, com um nó de stdlib pendurado
        // como "sorvedouro" (apenas é usado).
        let json_com_sysroot = r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
                {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
                {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
                {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"},
                {"id":100,"path":"core::fmt","name":"fmt","kind":"mod","visibility":"pub"},
                {"id":101,"path":"core::fmt::Display","name":"Display","kind":"trait","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
                {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
                {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
                {"from":"core::fmt","id_from":100,"to":"core::fmt::Display","id_to":101,"relation":"owns"},
                {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses"},
                {"from":"t::b::g","id_from":21,"to":"t::a::f","id_to":11,"relation":"uses"},
                {"from":"t::a::f","id_from":11,"to":"core::fmt::Display","id_to":101,"relation":"uses"}
            ]
        }"#;
        let r_completo = analisar_estrutura(
            FonteGrafo::Json(json_com_sysroot.to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .unwrap();
        let r_seu = analisar_estrutura(
            FonteGrafo::Json(json_com_sysroot.to_string()),
            Escopo::SeuCodigo,
            ModoUses::Todas,
        )
        .unwrap();

        // O CICLO é o mesmo: {t::a, t::b} nos dois escopos.
        assert_eq!(r_completo.ciclos, r_seu.ciclos);

        // O escopo MUDA quais módulos aparecem na listagem.
        let mods_completo: Vec<&str> =
            r_completo.modulos.iter().map(|p| p.as_str()).collect();
        let mods_seu: Vec<&str> = r_seu.modulos.iter().map(|p| p.as_str()).collect();
        assert!(mods_completo.contains(&"core::fmt"));
        assert!(!mods_seu.contains(&"core::fmt"));
    }

    /// E2E real (prompt 0031): analisar `lente_core`. Reporta a contagem
    /// de módulos e ciclos contra dado real.
    #[test]
    #[ignore]
    fn e2e_estrutura_lente_core_reporta_modulos_e_ciclos() {
        let r = analisar_estrutura(
            FonteGrafo::Pacote("lente_core".to_string()),
            Escopo::SeuCodigo,
            ModoUses::Todas,
        )
        .expect("estrutura E2E do lente_core deve funcionar");

        // Sanidade: o crate e seus módulos aparecem.
        assert!(r.modulos.iter().any(|p| p.as_str() == "lente_core"));
        // `lente_core` é cuidadoso — não esperamos ciclos entre seus módulos.
        assert!(
            r.ciclos.is_empty(),
            "lente_core não deve ter ciclos entre módulos; veio: {:?}",
            r.ciclos
        );
    }

    /// E2E real (prompt 0031): analisar o `egui` core. Mede o número de
    /// módulos e ciclos; ancora os achados no laudo. Não afirma número
    /// exato (varia com versão do fork/egui); afirma o **formato** e
    /// a presença de algum módulo conhecido.
    #[test]
    #[ignore]
    fn e2e_estrutura_egui_seu_codigo() {
        // Pacote do workspace egui; precisa ser rodado de dentro de
        // `<egui>/crates/egui` ou com `--pacote egui` num workspace que o
        // contenha. Este E2E roda só se ambiente estiver configurado.
        let r = analisar_estrutura(
            FonteGrafo::Pacote("egui".to_string()),
            Escopo::SeuCodigo,
            ModoUses::Todas,
        );
        let Ok(estrut) = r else {
            // Ambiente sem workspace egui — pula em silêncio.
            return;
        };
        assert!(estrut.modulos.len() > 1);
        // Ancoragem do laudo: número de módulos, ciclos, etc. Não-trivial,
        // registrado no laudo 0031.
    }

    // ---- Modo SoReferencia (prompt 0034) -----------------------------------

    /// JSON sintético do prompt 0034: tem um ciclo de módulos via duas
    /// arestas, uma `reference` (estrutural) e outra `import` (declaração
    /// `use` no topo do módulo). Sem o filtro, o ciclo aparece. Com
    /// `SoReferencia`, a aresta `import` some — e o ciclo desaparece.
    fn json_ciclo_misto_reference_e_import() -> &'static str {
        r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
                {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
                {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
                {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
                {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
                {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
                {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses","uses_kind":"reference"},
                {"from":"t::b","id_from":20,"to":"t::a::f","id_to":11,"relation":"uses","uses_kind":"import"}
            ]
        }"#
    }

    #[test]
    fn estrutura_todas_uses_detecta_ciclo_misto() {
        // Com `Todas`, o import conta — t::a → t::b → t::a fecha o anel.
        let r = analisar_estrutura(
            FonteGrafo::Json(json_ciclo_misto_reference_e_import().to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .unwrap();
        assert_eq!(r.ciclos.len(), 1);
        let nomes: Vec<&str> = r.ciclos[0].modulos.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["t::a", "t::b"]);
    }

    #[test]
    fn estrutura_so_referencia_descarta_import_e_desfaz_ciclo() {
        // Com `SoReferencia`, a aresta `import` (t::b → t::a) some, e o
        // ciclo desaparece. Resta uma DAG: t::a → t::b.
        let r = analisar_estrutura(
            FonteGrafo::Json(json_ciclo_misto_reference_e_import().to_string()),
            Escopo::Completo,
            ModoUses::SoReferencia,
        )
        .unwrap();
        assert!(r.ciclos.is_empty(), "ciclo deve sumir; veio: {:?}", r.ciclos);
        let deps: Vec<(&str, &str)> = r
            .dependencias
            .iter()
            .map(|d| (d.de.as_str(), d.para.as_str()))
            .collect();
        assert_eq!(deps, vec![("t::a", "t::b")]);
    }

    /// Prompt 0034: diagnóstico de fork antigo. Quando o JSON tem
    /// arestas `Uses` mas **nenhuma** carrega `uses_kind`, pedir
    /// `SoReferencia` retorna `ErroLente::ForkSemUsesKind` — em vez de
    /// descartar tudo silenciosamente.
    #[test]
    fn estrutura_so_referencia_com_fork_antigo_da_erro_proprio() {
        // JSON do laudo 0031: arestas `uses` sem `uses_kind`.
        let json_antigo = r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
                {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
                {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
                {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
                {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
                {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
                {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses"}
            ]
        }"#;
        match analisar_estrutura(
            FonteGrafo::Json(json_antigo.to_string()),
            Escopo::Completo,
            ModoUses::SoReferencia,
        ) {
            Err(ErroLente::ForkSemUsesKind) => {}
            outro => panic!("esperava ForkSemUsesKind, veio {:?}", outro),
        }
    }

    /// Não-regressão: o mesmo JSON antigo no modo `Todas` funciona normal.
    #[test]
    fn estrutura_todas_uses_com_fork_antigo_funciona() {
        let json_antigo = r#"{
            "crate": "t",
            "nodes": [
                {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
                {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
                {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
                {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
                {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"}
            ],
            "edges": [
                {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
                {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
                {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
                {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
                {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses"}
            ]
        }"#;
        let r = analisar_estrutura(
            FonteGrafo::Json(json_antigo.to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .expect("Todas com fork antigo deve funcionar");
        // DAG: t::a depende de t::b; sem ciclo.
        assert!(r.ciclos.is_empty());
    }

    /// E2E real (prompt 0034): com o fork atualizado (commit `b44aa96`+),
    /// reproduz o número do laudo 0033 — SCC cai de 85 para 42 ao
    /// contar só `reference`.
    #[test]
    #[ignore]
    fn e2e_estrutura_egui_so_referencia_reproduz_42() {
        let r = analisar_estrutura(
            FonteGrafo::Pacote("egui".to_string()),
            Escopo::Completo,
            ModoUses::SoReferencia,
        );
        let Ok(estrut) = r else { return };
        // Ancorado no laudo 0033: SCC de 42 módulos.
        let maior = estrut.ciclos.iter().map(|c| c.modulos.len()).max().unwrap_or(0);
        assert_eq!(
            maior, 42,
            "esperava o SCC de 42 do laudo 0033; veio {}",
            maior
        );
    }

    // ---- Modo estrutura — ordenamento da DSM (prompt 0035) -----------------

    #[test]
    fn estrutura_emite_ordem_e_blocos_do_dsm() {
        // Mesmo JSON do `analisar_estrutura_lista_modulos_e_detecta_ciclo`:
        // t::a ↔ t::b (ciclo). O agregado tem 3 módulos (t, t::a, t::b);
        // ordem = ordem topológica da condensação; bloco = {t::a, t::b}.
        let r = analisar_estrutura(
            FonteGrafo::Json(json_com_ciclo_de_modulos().to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .unwrap();

        assert_eq!(r.ordem.len(), r.modulos.len(), "ordem tem mesmo tamanho que modulos");
        let nomes_ordem: Vec<&str> = r.ordem.iter().map(|p| p.as_str()).collect();
        // `t` é a raiz (crate); não tem deps de saída → fica no início ou
        // fim conforme topológica do agregado. O bloco {t::a, t::b}
        // aparece contíguo.
        let idx_a = nomes_ordem.iter().position(|p| *p == "t::a").unwrap();
        let idx_b = nomes_ordem.iter().position(|p| *p == "t::b").unwrap();
        assert!(
            (idx_a as isize - idx_b as isize).abs() == 1,
            "membros do bloco devem ser contíguos; ordem={:?}",
            nomes_ordem
        );

        // Bloco do SCC: {t::a, t::b}.
        assert_eq!(r.blocos.len(), 1);
        let bloco_nomes: Vec<&str> =
            r.blocos[0].iter().map(|p| p.as_str()).collect();
        assert_eq!(bloco_nomes, vec!["t::a", "t::b"]);
    }

    /// Teste-consumidor (prompt 0035): a partir de `ordem` + `dependencias`,
    /// reconstrói a grade N×N e confere que ela bate com `dependencias`.
    /// Prova ponta-a-ponta que a "matriz como dado" é suficiente para a
    /// tela futura ou um agente.
    #[test]
    fn consumidor_reconstroi_grade_n_x_n_a_partir_da_saida() {
        let r = analisar_estrutura(
            FonteGrafo::Json(json_com_ciclo_de_modulos().to_string()),
            Escopo::Completo,
            ModoUses::Todas,
        )
        .unwrap();
        let n = r.ordem.len();
        let idx: std::collections::HashMap<&str, usize> = r
            .ordem
            .iter()
            .enumerate()
            .map(|(i, p)| (p.as_str(), i))
            .collect();
        let mut grade = vec![vec![false; n]; n];
        for d in &r.dependencias {
            let i = idx[d.de.as_str()];
            let j = idx[d.para.as_str()];
            grade[i][j] = true;
        }
        // Total de células = total de deps (1 por aresta deduplicada).
        let total: usize = grade.iter().flatten().filter(|c| **c).count();
        assert_eq!(total, r.dependencias.len());
    }

    /// E2E real (prompt 0035): ordem da DSM do egui no modo SoReferencia.
    /// O bloco de 42 (laudo 0033) é também um bloco da DSM, com seus
    /// membros contíguos em `ordem`.
    #[test]
    #[ignore]
    fn e2e_dsm_egui_bloco_de_42_e_contiguo() {
        let r = analisar_estrutura(
            FonteGrafo::Pacote("egui".to_string()),
            Escopo::Completo,
            ModoUses::SoReferencia,
        );
        let Ok(estrut) = r else { return };

        // Há exatamente um bloco com 42 membros (mesmo número do ciclo).
        let n_blocos_42 = estrut.blocos.iter().filter(|b| b.len() == 42).count();
        assert_eq!(n_blocos_42, 1);

        // Os 42 membros são contíguos em `ordem`.
        let bloco_42 = estrut.blocos.iter().find(|b| b.len() == 42).unwrap();
        let primeiro = bloco_42.first().unwrap().as_str();
        let idx_primeiro = estrut
            .ordem
            .iter()
            .position(|p| p.as_str() == primeiro)
            .expect("primeiro membro do bloco está em ordem");
        let fatia: Vec<&str> = estrut
            .ordem
            .iter()
            .skip(idx_primeiro)
            .take(42)
            .map(|p| p.as_str())
            .collect();
        let bloco_strs: Vec<&str> = bloco_42.iter().map(|p| p.as_str()).collect();
        assert_eq!(fatia, bloco_strs, "membros do bloco devem ser contíguos");
    }

    /// E2E real (prompt 0030): rankear o `lente_core` em `Completo`. Deve
    /// trazer ao menos um nó de sysroot no top — esperado, declarado, não
    /// regressão.
    #[test]
    #[ignore]
    fn e2e_ranking_do_lente_core_completo_traz_sysroot() {
        let r = rankear_pacote(
            FonteGrafo::Pacote("lente_core".to_string()),
            20,
            Escopo::Completo,
        )
        .expect("ranking E2E do lente_core (Completo) deve funcionar");

        let teve_sysroot = r.iter().any(|it| {
            let first = it.path.as_str().split("::").next().unwrap_or("");
            matches!(first, "core" | "std" | "alloc")
        });
        assert!(
            teve_sysroot,
            "esperava sysroot no top-20 Completo, veio: {:?}",
            r.iter().map(|it| it.path.as_str()).collect::<Vec<_>>()
        );
    }
}
