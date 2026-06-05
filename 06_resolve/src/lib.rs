//! Lineage: prompt 00_nucleo/prompt/0010-lente_resolve-v2.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0004-resolucao-colisoes-path.md
//!          00_nucleo/adr/0005-validacao-pos-medicao.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O.
//!
//! Aplicação da resolução de colisões de path. Recebe o `Veredito` que o
//! `lente_investiga` produziu e o materializa num `Grafo` novo (operação
//! pura — o grafo de entrada permanece intacto).
//!
//! Nomeação (ADR-0005 Ajuste 2):
//! - Padrão: contador por ordem de id (`path#1`, `path#2`, ...).
//! - Enriquecida: quando a evidência traz traits (`ImplDeTraitsDiferentes`)
//!   e há exatamente 2 cópias, usa `Tipo::<Trait>::metodo`.
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

    // `trait_` de cada nó colidente — a fonte do nome (ADR-0006). Vem do
    // próprio nó (id correto), encerrando a D4: nada de adivinhar por ordem.
    let trait_por_id: HashMap<usize, Option<String>> = grafo
        .nodes
        .iter()
        .filter(|n| ids.contains(&n.id))
        .map(|n| (n.id, n.trait_.clone()))
        .collect();

    // id -> novo path. Regra única (ADR-0006), aplicada nó a nó:
    // - tem `trait_` → nomeia por trait (`Tipo::<Trait>::metodo`);
    // - não tem → contador `#N` por ordem de id (piso, laudo 0010).
    let mut novo: HashMap<usize, String> = HashMap::new();
    for (i, id) in ids.iter().enumerate() {
        let nome = match trait_por_id.get(id).and_then(|t| t.as_deref()) {
            Some(t) => path_com_trait(colisao.as_str(), t),
            None => format!("{}#{}", colisao.as_str(), i + 1),
        };
        novo.insert(*id, nome);
    }

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
        }
    }

    fn no_com_trait(id: usize, path: &str, trait_: &str) -> No {
        let mut n = no(id, path);
        n.trait_ = Some(trait_.to_string());
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
}
