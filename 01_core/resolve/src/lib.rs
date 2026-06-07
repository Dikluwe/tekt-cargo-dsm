//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/resolve.md
//! @prompt-hash aa3a1a65
//! @layer L1
//! @updated 2026-06-07
//!          ampliado por prompt 00_nucleo/prompt/0042-resolve_escada_trait_ref.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0004-resolucao-colisoes-path.md
//!          00_nucleo/adr/0005-validacao-pos-medicao.md
//!          00_nucleo/adr/0006-nomeacao-trait-padrao.md (ampliado em laudo 0042)
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O.
//!
//! Aplicação da resolução de colisões de path. Recebe o `Veredito` que o
//! `lente_investiga` produziu e o materializa num `Grafo` novo (operação
//! pura — o grafo de entrada permanece intacto).
//!
//! Nomeação (ADR-0006, escada do prompt 0042):
//! degrau 1 — `<trait_>` antes do último segmento;
//! degrau 2 — se 2+ nós ficaram com o mesmo nome no degrau 1 (mesmo
//!            `trait_`), reescrever **esses** por `<trait_ref>`
//!            (a referência com argumentos: `From<&str>`);
//! degrau 3 — contador `#N` por ordem de id é o **piso** que sempre
//!            garante unicidade (id é único no grafo). Cai aqui o nó
//!            sem `trait_`, o nó cujo `trait_ref` ainda colide no
//!            degrau 2, e o nó com `trait_` colidindo mas `trait_ref =
//!            None`.
//!
//! Razão da escada: a regra original (só `trait_`) violava em silêncio
//! o invariante "paths únicos" para impls genéricos do mesmo trait
//! (4× `From<T>` viravam 4× `<From>::from`). Achado pela Arena no
//! laudo 0041; corrigido aqui (laudo 0042).
//!
//! A redistribuição de arestas é **determinística** graças à
//! identidade-por-nó (`id_from`/`id_to`): cada aresta sabe a qual cópia
//! pertence pelo id, sem ambiguidade.

#![forbid(unsafe_code)]

use core::error::Error;
use core::fmt;
use std::collections::{HashMap, HashSet};

use lente_core::entities::grafo::{Aresta, Grafo, No, Path, Relation};
use lente_core::entities::veredito::Veredito;

/// Modos de falha da resolução.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroResolve {
    /// O veredito foi `NaoDeterminado`; o diagnóstico é repassado intacto.
    ColisaoNaoResolvida(String),
    /// O `path` passado não tem cópias colidentes no grafo (menos de 2 nós).
    ColisaoInexistente,
    /// A evidência referencia ids que não correspondem aos nós colidentes.
    /// Defensiva — as evidências atuais não carregam ids, então não há
    /// caminho ativo que a dispare; mantida por completude do contrato.
    IdInconsistente,
}

impl fmt::Display for ErroResolve {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroResolve::ColisaoNaoResolvida(d) => {
                write!(f, "colisão não resolvida: {}", d)
            }
            ErroResolve::ColisaoInexistente => {
                f.write_str("colisão inexistente: path tem menos de 2 nós")
            }
            ErroResolve::IdInconsistente => {
                f.write_str("evidência referencia ids inconsistentes com os nós colidentes")
            }
        }
    }
}

impl Error for ErroResolve {}

/// Aplica a resolução de uma colisão de path, devolvendo um `Grafo` novo.
pub fn aplicar(
    grafo: &Grafo,
    colisao: &Path,
    veredito: &Veredito,
) -> Result<Grafo, ErroResolve> {
    let ids_colidentes: Vec<usize> = grafo
        .nodes
        .iter()
        .filter(|n| &n.path == colisao)
        .map(|n| n.id)
        .collect();

    if ids_colidentes.len() < 2 {
        return Err(ErroResolve::ColisaoInexistente);
    }

    match veredito {
        Veredito::NaoDeterminado { diagnostico } => {
            Err(ErroResolve::ColisaoNaoResolvida(diagnostico.clone()))
        }
        // A evidência (`VizinhancaDisjunta` ou `ImplDeTraitsDiferentes`) decide
        // SE as cópias são distintas; o NOME de cada uma vem do `trait_` do
        // próprio nó (ADR-0006), não da evidência. Por isso ignoramos o
        // conteúdo da evidência aqui.
        Veredito::Distintos { .. } => {
            Ok(aplicar_distintos(grafo, colisao, &ids_colidentes))
        }
        Veredito::MesmoItem => {
            Ok(aplicar_mesmo_item(grafo, &ids_colidentes))
        }
    }
}

/// Insere `<Trait>` antes do último segmento do path:
/// `a::b::Tipo::metodo` + `Display` → `a::b::Tipo::<Display>::metodo`.
fn path_com_trait(path: &str, nome_trait: &str) -> String {
    match path.rsplit_once("::") {
        Some((prefixo, ultimo)) => {
            format!("{}::<{}>::{}", prefixo, nome_trait, ultimo)
        }
        None => format!("<{}>::{}", nome_trait, path),
    }
}

fn aplicar_distintos(grafo: &Grafo, colisao: &Path, ids_colidentes: &[usize]) -> Grafo {
    let mut ids: Vec<usize> = ids_colidentes.to_vec();
    ids.sort_unstable();

    // `trait_` e `trait_ref` de cada nó colidente — a fonte do nome
    // (ADR-0006, escada do prompt 0042). Vêm do próprio nó (id correto),
    // encerrando a D4: nada de adivinhar por ordem.
    let trait_por_id: HashMap<usize, Option<String>> = grafo
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .map(|n| (n.id, n.trait_.clone()))
        .collect();
    let trait_ref_por_id: HashMap<usize, Option<String>> = grafo
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .map(|n| (n.id, n.trait_ref.clone()))
        .collect();

    // Degrau 1 — tenta `<trait_>`. Nó sem `trait_` é separado para o
    // contador desde já (não dá pra nomear por trait sem trait).
    let mut nome_d1: HashMap<usize, String> = HashMap::new();
    let mut sem_trait: Vec<usize> = Vec::new();
    for id in &ids {
        match trait_por_id.get(id).and_then(|t| t.as_deref()) {
            Some(t) => {
                nome_d1.insert(*id, path_com_trait(colisao.as_str(), t));
            }
            None => sem_trait.push(*id),
        }
    }

    // Contagem dos nomes do degrau 1 para detectar grupos colidentes.
    let mut grupos_d1: HashMap<String, Vec<usize>> = HashMap::new();
    for (id, nome) in &nome_d1 {
        grupos_d1.entry(nome.clone()).or_default().push(*id);
    }

    // Para cada grupo do degrau 1:
    //   - tamanho 1 → fica com o nome do degrau 1 (sem regressão do
    //     caso `Display + Debug`);
    //   - tamanho ≥ 2 → escala para o degrau 2 (`<trait_ref>`).
    let mut nomes_finais: HashMap<usize, String> = HashMap::new();
    // `pendentes_contador` recebe os ids que não tiveram nome único nem no
    // degrau 1 nem no degrau 2.
    let mut pendentes_contador: Vec<usize> = sem_trait;
    for (_nome_d1_str, ids_grupo) in grupos_d1 {
        if ids_grupo.len() == 1 {
            let id = ids_grupo[0];
            nomes_finais.insert(id, nome_d1.get(&id).expect("nome_d1[id]").clone());
            continue;
        }
        // Degrau 2 — tenta `<trait_ref>`. Conta dentro do grupo.
        let mut nome_d2: HashMap<usize, String> = HashMap::new();
        let mut sem_ref: Vec<usize> = Vec::new();
        for id in &ids_grupo {
            match trait_ref_por_id.get(id).and_then(|t| t.as_deref()) {
                Some(tr) => {
                    nome_d2.insert(*id, path_com_trait(colisao.as_str(), tr));
                }
                None => sem_ref.push(*id),
            }
        }
        pendentes_contador.extend(sem_ref);
        // Verificar unicidade entre os do grupo (degrau 2).
        let mut grupos_d2: HashMap<String, Vec<usize>> = HashMap::new();
        for (id, nome) in &nome_d2 {
            grupos_d2.entry(nome.clone()).or_default().push(*id);
        }
        for (_nome_d2_str, ids_g2) in grupos_d2 {
            if ids_g2.len() == 1 {
                let id = ids_g2[0];
                nomes_finais.insert(id, nome_d2.get(&id).expect("nome_d2[id]").clone());
            } else {
                // Degrau 2 ainda colide (mesmo `trait_ref`, patológico) →
                // contador.
                pendentes_contador.extend(ids_g2);
            }
        }
    }

    // Degrau 3 — contador `#N` por ordem de id no conjunto original
    // (`ids`, já sortido). O índice **global** (posição em `ids`) preserva
    // o comportamento do laudo 0010 D9: ids = [1, 2], id=2 sem trait →
    // `#2` (não `#1`), mesmo quando id=1 ganha nome por trait.
    pendentes_contador.sort_unstable();
    pendentes_contador.dedup();
    for id in &pendentes_contador {
        let i = ids.iter().position(|x| x == id).expect("id ∈ ids");
        nomes_finais.insert(*id, format!("{}#{}", colisao.as_str(), i + 1));
    }

    let novo = nomes_finais;
    let nodes: Vec<No> = grafo
        .nodes
        .iter()
        .map(|n| {
            let mut nn = n.clone();
            if let Some(p) = novo.get(&n.id) {
                nn.path = Path::from(p.as_str());
            }
            nn
        })
        .collect();

    let edges: Vec<Aresta> = grafo
        .edges
        .iter()
        .map(|a| {
            let mut na = a.clone();
            if let Some(p) = novo.get(&a.id_from) {
                na.from = Path::from(p.as_str());
            }
            if let Some(p) = novo.get(&a.id_to) {
                na.to = Path::from(p.as_str());
            }
            na
        })
        .collect();

    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes,
        edges,
    }
}

fn aplicar_mesmo_item(grafo: &Grafo, ids_colidentes: &[usize]) -> Grafo {
    let mut ids: Vec<usize> = ids_colidentes.to_vec();
    ids.sort_unstable();
    let canonico = ids[0];
    // Cópias divergentes em name/kind/visibility são resolvidas a favor do
    // menor id (canônico), silenciosamente — L1 puro não tem canal de aviso.
    let outros: HashSet<usize> = ids[1..].iter().copied().collect();

    // Nós: descartar as cópias não-canônicas.
    let nodes: Vec<No> = grafo
        .nodes
        .iter()
        .filter(|n| !outros.contains(&n.id))
        .cloned()
        .collect();

    // Arestas: redirecionar referências às cópias para o canônico.
    let mut edges: Vec<Aresta> = grafo
        .edges
        .iter()
        .map(|a| {
            let mut na = a.clone();
            if outros.contains(&na.id_from) {
                na.id_from = canonico;
            }
            if outros.contains(&na.id_to) {
                na.id_to = canonico;
            }
            na
        })
        .collect();

    // Dedup de arestas que ficaram idênticas após o redirecionamento.
    let mut vistas: HashSet<(usize, usize, Relation)> = HashSet::new();
    edges.retain(|a| vistas.insert((a.id_from, a.id_to, a.relation)));

    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes,
        edges,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{Kind, Modificadores, Visibility};
    use lente_core::entities::veredito::Evidencia;

    fn no(id: usize, path: &str) -> No {
        No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "c".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    fn no_com_trait(id: usize, path: &str, trait_: &str) -> No {
        let mut n = no(id, path);
        n.trait_ = Some(trait_.to_string());
        n
    }

    /// Auxiliar do prompt 0042: nó com `trait_` E `trait_ref` (impls
    /// genéricos do mesmo trait, ex.: `From<&str>`, `From<String>`).
    fn no_com_trait_e_ref(id: usize, path: &str, trait_: &str, trait_ref: &str) -> No {
        let mut n = no_com_trait(id, path, trait_);
        n.trait_ref = Some(trait_ref.to_string());
        n
    }

    fn aresta(from: &str, id_from: usize, to: &str, id_to: usize, r: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from,
            to: Path::from(to),
            id_to,
            relation: r,
            uses_kind: None,
        }
    }

    /// Verifica os invariantes que todo grafo de saída deve satisfazer.
    fn checar_invariantes(g: &Grafo) {
        // ids únicos
        let mut ids: Vec<usize> = g.nodes.iter().map(|n| n.id).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "ids únicos");
        // integridade referencial: id_from/id_to referenciam nós existentes
        let conjunto: HashSet<usize> = g.nodes.iter().map(|n| n.id).collect();
        for a in &g.edges {
            assert!(conjunto.contains(&a.id_from), "id_from {} existe", a.id_from);
            assert!(conjunto.contains(&a.id_to), "id_to {} existe", a.id_to);
        }
    }

    fn paths_unicos(g: &Grafo) -> bool {
        let mut ps: Vec<&str> = g.nodes.iter().map(|n| n.path.as_str()).collect();
        let total = ps.len();
        ps.sort_unstable();
        ps.dedup();
        ps.len() == total
    }

    #[test]
    fn distintos_contador_renomeia_e_redistribui() {
        // Dois nós "M::T::fmt" (ids 1, 2). Aresta Owns de M(0) para cada cópia.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no(1, "M::T::fmt"),
                no(2, "M::T::fmt"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();

        // Nós renomeados por contador (ordem de id).
        assert_eq!(r.nodes[1].path.as_str(), "M::T::fmt#1"); // id 1
        assert_eq!(r.nodes[2].path.as_str(), "M::T::fmt#2"); // id 2
        // Aresta para id 1 aponta para #1; para id 2 aponta para #2.
        let a1 = r.edges.iter().find(|a| a.id_to == 1).unwrap();
        let a2 = r.edges.iter().find(|a| a.id_to == 2).unwrap();
        assert_eq!(a1.to.as_str(), "M::T::fmt#1");
        assert_eq!(a2.to.as_str(), "M::T::fmt#2");
        assert!(paths_unicos(&r), "sem colisão de path após resolução");
        checar_invariantes(&r);
    }

    #[test]
    fn distintos_com_trait_no_no_nomeia_por_trait() {
        // O trait vem do PRÓPRIO nó (ADR-0006), não da evidência.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait(1, "M::T::fmt", "Display"),
                no_com_trait(2, "M::T::fmt", "Debug"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 2, Relation::Owns),
            ],
        };
        // Evidência topológica (o caso comum de 97%); o nome vem do nó.
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        assert_eq!(r.nodes[1].path.as_str(), "M::T::<Display>::fmt"); // id 1
        assert_eq!(r.nodes[2].path.as_str(), "M::T::<Debug>::fmt"); // id 2
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    #[test]
    fn distintos_trait_atribuicao_exata_mata_d4() {
        // O teste que encerra a D4: o nó de MENOR id carrega "Debug" e o de
        // MAIOR id carrega "Display" — ordem INVERSA do alfabeto/posição.
        // A adivinhação antiga (menor id = primeiro trait) erraria; ler do nó
        // acerta. Cada nó pega EXATAMENTE o seu trait.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait(36, "M::T::fmt", "Debug"),   // menor id → Debug
                no_com_trait(47, "M::T::fmt", "Display"), // maior id → Display
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 36, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 47, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        let n36 = r.nodes.iter().find(|n| n.id == 36).unwrap();
        let n47 = r.nodes.iter().find(|n| n.id == 47).unwrap();
        assert_eq!(n36.path.as_str(), "M::T::<Debug>::fmt");
        assert_eq!(n47.path.as_str(), "M::T::<Display>::fmt");
        checar_invariantes(&r);
    }

    #[test]
    fn distintos_mistura_trait_e_contador() {
        // Um nó com trait (vira <Trait>), outro sem (vira #N). Cada um por sua
        // regra. Caso raro mas a regra trata nó a nó.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait(1, "M::T::f", "Display"),
                no(2, "M::T::f"), // sem trait
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::f", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::f", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::f"), &v).unwrap();
        assert_eq!(r.nodes.iter().find(|n| n.id == 1).unwrap().path.as_str(), "M::T::<Display>::f");
        assert_eq!(r.nodes.iter().find(|n| n.id == 2).unwrap().path.as_str(), "M::T::f#2");
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    #[test]
    fn distintos_tres_copias_sem_trait_usam_contador() {
        // 3 cópias SEM trait → contador #1/#2/#3 (piso, laudo 0010).
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no(5, "M::T::f"),
                no(7, "M::T::f"),
                no(9, "M::T::f"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::f", 5, Relation::Owns),
                aresta("M::T", 0, "M::T::f", 7, Relation::Owns),
                aresta("M::T", 0, "M::T::f", 9, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::f"), &v).unwrap();
        // ids 5,7,9 ordenados → #1,#2,#3
        assert_eq!(r.nodes.iter().find(|n| n.id == 5).unwrap().path.as_str(), "M::T::f#1");
        assert_eq!(r.nodes.iter().find(|n| n.id == 7).unwrap().path.as_str(), "M::T::f#2");
        assert_eq!(r.nodes.iter().find(|n| n.id == 9).unwrap().path.as_str(), "M::T::f#3");
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    #[test]
    fn mesmo_item_unifica() {
        // Duas cópias "M::T::f" (ids 1,2). Arestas de dois usuários distintos.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(1, "M::T::f"),
                no(2, "M::T::f"),
                no(3, "M::user_a"),
                no(4, "M::user_b"),
            ],
            edges: vec![
                aresta("M::user_a", 3, "M::T::f", 1, Relation::Uses),
                aresta("M::user_b", 4, "M::T::f", 2, Relation::Uses),
            ],
        };
        let r = aplicar(&g, &Path::from("M::T::f"), &Veredito::MesmoItem).unwrap();
        // Só um nó "M::T::f" (o canônico, id 1).
        let copias: Vec<_> = r.nodes.iter().filter(|n| n.path.as_str() == "M::T::f").collect();
        assert_eq!(copias.len(), 1);
        assert_eq!(copias[0].id, 1);
        // Ambos os usuários agora apontam para id 1.
        assert!(r.edges.iter().all(|a| a.id_to == 1));
        assert_eq!(r.edges.len(), 2);
        checar_invariantes(&r);
    }

    #[test]
    fn mesmo_item_dedup_arestas_identicas() {
        // Dois nós colidentes recebem a MESMA aresta de entrada → após unificar,
        // viram uma só.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![no(1, "T::f"), no(2, "T::f"), no(3, "user")],
            edges: vec![
                aresta("user", 3, "T::f", 1, Relation::Uses),
                aresta("user", 3, "T::f", 2, Relation::Uses),
            ],
        };
        let r = aplicar(&g, &Path::from("T::f"), &Veredito::MesmoItem).unwrap();
        // As duas arestas viram (user→3, T::f→1, Uses) — dedup para uma.
        assert_eq!(r.edges.len(), 1);
        checar_invariantes(&r);
    }

    #[test]
    fn nao_determinado_propaga_erro_sem_modificar() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![no(1, "T::f"), no(2, "T::f")],
            edges: vec![],
        };
        let v = Veredito::NaoDeterminado {
            diagnostico: "macro-gerado".to_string(),
        };
        match aplicar(&g, &Path::from("T::f"), &v).unwrap_err() {
            ErroResolve::ColisaoNaoResolvida(d) => assert_eq!(d, "macro-gerado"),
            outro => panic!("erro inesperado: {:?}", outro),
        }
    }

    #[test]
    fn path_sem_colisao_e_erro() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![no(1, "T::f"), no(2, "T::g")],
            edges: vec![],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 0,
                exclusivas_b: 0,
            },
        };
        assert_eq!(
            aplicar(&g, &Path::from("T::f"), &v).unwrap_err(),
            ErroResolve::ColisaoInexistente
        );
    }

    #[test]
    fn determinismo_aplicar_duas_vezes_da_mesmo_resultado() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![no(0, "M::T"), no(1, "M::T::fmt"), no(2, "M::T::fmt")],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r1 = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        let r2 = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn grafo_original_nao_e_modificado() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![no(1, "T::f"), no(2, "T::f")],
            edges: vec![aresta("x", 9, "T::f", 1, Relation::Uses)],
        };
        // Adicionar nó x para integridade do grafo de entrada.
        let mut g = g;
        g.nodes.push(no(9, "x"));
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 0,
            },
        };
        let _ = aplicar(&g, &Path::from("T::f"), &v).unwrap();
        // O grafo de entrada continua com as cópias colidentes intactas.
        let copias: Vec<_> = g.nodes.iter().filter(|n| n.path.as_str() == "T::f").collect();
        assert_eq!(copias.len(), 2);
    }

    // ---- Escada `trait_` → `trait_ref` → contador (prompt 0042) -------------

    /// **Degrau 2 (2 cópias)**: o caso real do laudo 0041 — `Path::from`
    /// com duas cópias, ambas `trait_ = "From"`, `trait_ref` distintos
    /// (`From<&str>`, `From<String>`). Antes do prompt 0042, viravam
    /// `Path::<From>::from` ×2 (colidem). Agora viram
    /// `Path::<From<&str>>::from` e `Path::<From<String>>::from`.
    #[test]
    fn escada_d2_path_from_dois_impl_genericos_se_distinguem_por_trait_ref() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::Path"),
                no_com_trait_e_ref(1, "M::Path::from", "From", "From<&str>"),
                no_com_trait_e_ref(2, "M::Path::from", "From", "From<String>"),
            ],
            edges: vec![
                aresta("M::Path", 0, "M::Path::from", 1, Relation::Owns),
                aresta("M::Path", 0, "M::Path::from", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::Path::from"), &v).unwrap();
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 1).unwrap().path.as_str(),
            "M::Path::<From<&str>>::from"
        );
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 2).unwrap().path.as_str(),
            "M::Path::<From<String>>::from"
        );
        assert!(paths_unicos(&r), "paths únicos após resolução (invariante laudo 0010)");
        checar_invariantes(&r);
    }

    /// **Degrau 2 (4 cópias)**: o caso real do laudo 0041 —
    /// `ErroLente::from` com 4 cópias, todas `trait_ = "From"`, `trait_ref`
    /// distintos por argumento (`From<ErroFork>`, `From<ErroAdaptador>`,
    /// `From<ErroResolve>`, `From<ErroRaio>`). 4 paths únicos via
    /// `<trait_ref>`.
    #[test]
    fn escada_d2_erro_lente_from_quatro_impl_genericos_se_distinguem() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "W::ErroLente"),
                no_com_trait_e_ref(10, "W::ErroLente::from", "From", "From<ErroFork>"),
                no_com_trait_e_ref(11, "W::ErroLente::from", "From", "From<ErroAdaptador>"),
                no_com_trait_e_ref(12, "W::ErroLente::from", "From", "From<ErroResolve>"),
                no_com_trait_e_ref(13, "W::ErroLente::from", "From", "From<ErroRaio>"),
            ],
            edges: vec![
                aresta("W::ErroLente", 0, "W::ErroLente::from", 10, Relation::Owns),
                aresta("W::ErroLente", 0, "W::ErroLente::from", 11, Relation::Owns),
                aresta("W::ErroLente", 0, "W::ErroLente::from", 12, Relation::Owns),
                aresta("W::ErroLente", 0, "W::ErroLente::from", 13, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 2,
                exclusivas_b: 2,
            },
        };
        let r = aplicar(&g, &Path::from("W::ErroLente::from"), &v).unwrap();
        let p = |id: usize| {
            r.nodes
                .iter()
                .find(|n| n.id == id)
                .unwrap()
                .path
                .as_str()
                .to_string()
        };
        assert_eq!(p(10), "W::ErroLente::<From<ErroFork>>::from");
        assert_eq!(p(11), "W::ErroLente::<From<ErroAdaptador>>::from");
        assert_eq!(p(12), "W::ErroLente::<From<ErroResolve>>::from");
        assert_eq!(p(13), "W::ErroLente::<From<ErroRaio>>::from");
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    /// **Degrau 1 não-regressão**: `Display + Debug` em `T::fmt` continua
    /// virando `T::<Display>::fmt` / `T::<Debug>::fmt` — o `trait_` já
    /// distingue, a escada não escala. Garante zero regressão do caso
    /// canônico que dominava os testes pré-0042.
    #[test]
    fn escada_d1_display_debug_nao_escala_para_d2_nem_d3() {
        // Cópias com `trait_ref` preenchido também (mais realista — o
        // fork emite ambos). O `trait_` já basta, então `<Display>` e
        // `<Debug>`, NÃO `<Display>::for-…>::fmt` ou similar.
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait_e_ref(1, "M::T::fmt", "Display", "Display"),
                no_com_trait_e_ref(2, "M::T::fmt", "Debug", "Debug"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 1).unwrap().path.as_str(),
            "M::T::<Display>::fmt"
        );
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 2).unwrap().path.as_str(),
            "M::T::<Debug>::fmt"
        );
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    /// **Degrau 3 — `trait_ref = None` num conjunto que colide no d1**.
    /// Duas cópias, ambas `trait_ = "From"`, mas **sem** `trait_ref`
    /// (fork hipoteticamente velho, ou nó-referência sem o subcampo).
    /// Ambas escalam para d2, ambas têm `trait_ref = None` → caem no
    /// contador.
    #[test]
    fn escada_d3_trait_ref_ausente_no_grupo_colidindo_cai_no_contador() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait(1, "M::T::from", "From"), // sem trait_ref
                no_com_trait(2, "M::T::from", "From"), // sem trait_ref
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::from", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::from", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::from"), &v).unwrap();
        // ids = [1, 2] (sortidos); pendentes → posição 0 e 1 → #1 e #2.
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 1).unwrap().path.as_str(),
            "M::T::from#1"
        );
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 2).unwrap().path.as_str(),
            "M::T::from#2"
        );
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    /// **Degrau 3 — patológico: `trait_ref` idênticos**. Não deveria
    /// acontecer no Rust real, mas o piso protege: duas cópias com
    /// `trait_ref` iguais (impossível na linguagem; teste construído)
    /// caem no contador. Garante que o piso pega 100% dos casos.
    #[test]
    fn escada_d3_patologico_trait_ref_identicos_cai_no_contador() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait_e_ref(1, "M::T::from", "From", "From<X>"),
                no_com_trait_e_ref(2, "M::T::from", "From", "From<X>"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::from", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::from", 2, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::from"), &v).unwrap();
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 1).unwrap().path.as_str(),
            "M::T::from#1"
        );
        assert_eq!(
            r.nodes.iter().find(|n| n.id == 2).unwrap().path.as_str(),
            "M::T::from#2"
        );
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    /// **Mistura d1 ok + d2 colide**: 3 cópias — uma `Display` (d1 sai
    /// limpo), duas `From<T>` (d1 colide entre essas duas, escalam para
    /// d2 — ambas com `trait_ref` distintos, d2 resolve).
    #[test]
    fn escada_mistura_d1_ok_e_d2_resolvendo() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait_e_ref(1, "M::T::fmt", "Display", "Display"),
                no_com_trait_e_ref(2, "M::T::fmt", "From", "From<&str>"),
                no_com_trait_e_ref(3, "M::T::fmt", "From", "From<u32>"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::fmt", 1, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 2, Relation::Owns),
                aresta("M::T", 0, "M::T::fmt", 3, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r = aplicar(&g, &Path::from("M::T::fmt"), &v).unwrap();
        let p = |id: usize| r.nodes.iter().find(|n| n.id == id).unwrap().path.as_str().to_string();
        assert_eq!(p(1), "M::T::<Display>::fmt"); // d1 limpo
        assert_eq!(p(2), "M::T::<From<&str>>::fmt"); // d2
        assert_eq!(p(3), "M::T::<From<u32>>::fmt"); // d2
        assert!(paths_unicos(&r));
        checar_invariantes(&r);
    }

    /// **Determinismo da escada**: o conjunto de saídas é estável entre
    /// execuções (HashMap interno não vaza ordem para o resultado;
    /// pendentes_contador é ordenado por id). Aplicar duas vezes dá o
    /// mesmo grafo.
    #[test]
    fn escada_determinismo_aplicar_duas_vezes() {
        let g = Grafo {
            crate_name: "c".to_string(),
            nodes: vec![
                no(0, "M::T"),
                no_com_trait_e_ref(7, "M::T::from", "From", "From<A>"),
                no_com_trait_e_ref(3, "M::T::from", "From", "From<B>"),
                no_com_trait_e_ref(11, "M::T::from", "From", "From<C>"),
            ],
            edges: vec![
                aresta("M::T", 0, "M::T::from", 7, Relation::Owns),
                aresta("M::T", 0, "M::T::from", 3, Relation::Owns),
                aresta("M::T", 0, "M::T::from", 11, Relation::Owns),
            ],
        };
        let v = Veredito::Distintos {
            evidencia: Evidencia::VizinhancaDisjunta {
                exclusivas_a: 1,
                exclusivas_b: 1,
            },
        };
        let r1 = aplicar(&g, &Path::from("M::T::from"), &v).unwrap();
        let r2 = aplicar(&g, &Path::from("M::T::from"), &v).unwrap();
        assert_eq!(r1, r2);
        assert!(paths_unicos(&r1));
    }
}
