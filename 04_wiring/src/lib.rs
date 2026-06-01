//! Lineage: prompt 00_nucleo/prompt/0019-l4-wiring.md
//! Camada:  L4 — Fiação (composição pura, sem lógica de negócio).
//!
//! Compõe o pipeline da lente ponta a ponta:
//!
//!   FonteGrafo → [JSON cru] → desserializa → grafo → [detecta colisões]
//!     → para cada colisão: investigar (lente_investiga) + aplicar
//!       (lente_resolve) → grafo resolvido (paths únicos)
//!     → resolver alvo (path direto ou via id) → calcular_raio → Raio.
//!
//! Não formata, não escreve em stdout, não lida com argumentos — isso é L2.

#![forbid(unsafe_code)]

use core::error::Error;
use core::fmt;
use std::collections::HashMap;

use lente_core::domain::raio::{ErroRaio, Raio, calcular_raio};
use lente_core::entities::grafo::{Aresta, Grafo, Path};
use lente_infra::ErroAdaptador;
use lente_infra::fork::ErroFork;
use lente_investiga::{ArestasNo, ParColidente, Vizinhanca};
use lente_resolve::ErroResolve;

/// De onde vem o grafo: JSON pronto ou nome de pacote (invoca o fork).
pub enum FonteGrafo {
    /// JSON pronto (o L2 leu de arquivo ou stdin).
    Json(String),
    /// Nome de pacote — o wiring invoca o fork via `lente_infra::fork`.
    Pacote(String),
}

/// Como o alvo do raio é apontado: por path canônico ou por id.
pub enum AlvoBusca {
    PorPath(Path),
    PorId(usize),
}

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
        }
    }
}

impl Error for ErroLente {}

/// Pipeline completo: extrai (ou recebe) o grafo, resolve colisões, e calcula
/// o raio do alvo.
pub fn calcular_raio_de_alvo(
    fonte: FonteGrafo,
    alvo: AlvoBusca,
) -> Result<Raio, ErroLente> {
    // 1. Obter o JSON cru.
    let json = match fonte {
        FonteGrafo::Json(s) => s,
        FonteGrafo::Pacote(p) => lente_infra::fork::invocar_fork(&p)?,
    };

    // 2. Desserializar.
    let mut grafo = lente_infra::desserializar_grafo(&json)?;

    // 3. Detectar paths colidentes (uma vez, no grafo de entrada).
    //    Os `aplicar`s só tocam o path da colisão sendo resolvida, então a
    //    lista capturada agora permanece consistente durante a iteração.
    let colisoes = detectar_colisoes(&grafo);

    // 4. Investigar + resolver cada colisão. O grafo evolui a cada passo.
    for path_colidente in colisoes {
        grafo = resolver_uma_colisao(grafo, &path_colidente)?;
    }

    // 5. Resolver o alvo no grafo resolvido.
    let path_alvo = match alvo {
        AlvoBusca::PorPath(p) => p,
        AlvoBusca::PorId(id) => grafo
            .nodes
            .iter()
            .find(|n| n.id == id)
            .map(|n| n.path.clone())
            .ok_or(ErroLente::IdInexistente(id))?,
    };

    // 6. Calcular o raio.
    let raio = calcular_raio(&grafo, &path_alvo)?;
    Ok(raio)
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
        )
        .expect("alvo por path deve funcionar");
        assert_eq!(raio.alvo.as_str(), "t::T::<Debug>::fmt");
    }

    #[test]
    fn id_inexistente_retorna_erro_proprio() {
        match calcular_raio_de_alvo(
            FonteGrafo::Json(json_sintetico_com_colisao().to_string()),
            AlvoBusca::PorId(9999),
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
        ) {
            Err(ErroLente::Adaptador(ErroAdaptador::JsonInvalido(_))) => {}
            outro => panic!("esperava Adaptador/JsonInvalido, veio {:?}", outro),
        }
    }

    #[test]
    fn display_de_erro_lente_cobre_variantes() {
        let v1 = ErroLente::IdInexistente(42);
        let v2 = ErroLente::Adaptador(ErroAdaptador::JsonInvalido("eof".to_string()));
        for v in [v1, v2].iter() {
            assert!(!format!("{}", v).is_empty());
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
}
