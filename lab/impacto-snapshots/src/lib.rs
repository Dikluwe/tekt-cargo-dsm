//! Crystalline Lineage
//! @prompt lab/impacto-snapshots/CONTRATO.md
//! @updated 2026-09-26
//! Comparação pura de impacto. IDs são locais, identidade entre lados é explícita.
#![forbid(unsafe_code)]

use lente_core::{
    domain::raio::{Classificacao, calcular_raio},
    entities::grafo::{Grafo, Path, Relation, UsesKind},
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lado {
    Antes,
    Depois,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Completude {
    Completa,
    Parcial,
    Desconhecida,
}
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub versao: String,
    pub grafo: Grafo,
    pub completude: Completude,
    pub granularidade: String,
    pub natureza: String,
    pub diagnosticos: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Correspondencia {
    pub identidade: String,
    pub antes: usize,
    pub depois: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mudanca {
    Remocao {
        antes: usize,
    },
    Adicao {
        depois: usize,
    },
    Pareada {
        identidade: String,
        alteracao_declarada: bool,
        renomeacao_declarada: bool,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Origem {
    pub lado: Lado,
    pub versao: String,
    pub completude: Completude,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescricaoSnapshot {
    pub origem: Origem,
    pub granularidade: String,
    pub natureza: String,
    pub diagnosticos: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Elemento {
    pub origem: Origem,
    pub id_local: usize,
    pub path: Path,
    pub identidade: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Alcance {
    pub elemento: Elemento,
    pub profundidade: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImpactoSnapshot {
    pub alvo: Elemento,
    pub classificacao: Classificacao,
    pub entradas_diretas: usize,
    pub saidas_diretas: usize,
    pub consumidores: Vec<Alcance>,
    pub dependencias: Vec<Alcance>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoOutroLado {
    SemCorrespondencia,
    AlvoNaoComparavel,
    NaoAlcancadoNoGrafoCompleto,
    NaoObservadoComCoberturaLimitada,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservacaoUnilateral {
    pub consumidor: Alcance,
    pub contraparte: Option<Elemento>,
    pub estado_outro_lado: EstadoOutroLado,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumidorNosDois {
    pub identidade: String,
    pub antes: Alcance,
    pub depois: Alcance,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConjuntosConsumidores {
    pub comparacao_bilateral: bool,
    pub somente_antes: Vec<ObservacaoUnilateral>,
    pub ambos: Vec<ConsumidorNosDois>,
    pub somente_depois: Vec<ObservacaoUnilateral>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Subtipo {
    Desconhecido,
    Import,
    Reference,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelacaoObservada {
    pub de: Elemento,
    pub para: Elemento,
    pub subtipo: Subtipo,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelacaoNosDois {
    pub antes: RelacaoObservada,
    pub depois: RelacaoObservada,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusTopologia {
    MesmasRelacoesObservadas,
    DiferencasNasRelacoesObservadas,
    ComparacaoLimitada,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologiaIncidente {
    pub comuns: Vec<RelacaoNosDois>,
    pub somente_antes: Vec<RelacaoObservada>,
    pub somente_depois: Vec<RelacaoObservada>,
    pub cobertura_limitada: bool,
    pub endpoints_sem_par: bool,
    pub status: StatusTopologia,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultadoMudanca {
    pub mudanca: Mudanca,
    pub antes: Option<ImpactoSnapshot>,
    pub depois: Option<ImpactoSnapshot>,
    pub consumidores: ConjuntosConsumidores,
    pub topologia_incidente: Option<TopologiaIncidente>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparacaoImpacto {
    pub antes: DescricaoSnapshot,
    pub depois: DescricaoSnapshot,
    pub correspondencias: Vec<Correspondencia>,
    pub sem_par_antes: Vec<Elemento>,
    pub sem_par_depois: Vec<Elemento>,
    pub mudancas: Vec<ResultadoMudanca>,
    pub limites: Vec<&'static str>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Erro {
    RotuloInvalido {
        lado: Lado,
        campo: &'static str,
    },
    EscoposIncompativeis,
    IdDuplicado {
        lado: Lado,
        id: usize,
    },
    ReferenciaInexistente {
        lado: Lado,
        id: usize,
        contexto: &'static str,
    },
    EndpointDivergente {
        lado: Lado,
        id: usize,
        path: Path,
    },
    CorrespondenciaDuplicada(Correspondencia),
    IdentidadeReutilizada(String),
    EndpointReutilizado {
        lado: Lado,
        id: usize,
    },
    MudancaEmEndpointPareado {
        lado: Lado,
        id: usize,
    },
    IdentidadeNaoDeclarada(String),
    MudancaSemDeclaracao(String),
    MudancasConflitantes(String),
    FalhaRaio {
        lado: Lado,
        id: usize,
        motivo: String,
    },
}
impl std::fmt::Display for Erro {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Erro {}

fn validar_grafo(s: &Snapshot, lado: Lado, erros: &mut BTreeSet<Erro>) {
    for (campo, valor) in [
        ("versao", &s.versao),
        ("granularidade", &s.granularidade),
        ("natureza", &s.natureza),
    ] {
        if valor.trim().is_empty()
            || (campo != "versao" && valor.trim().eq_ignore_ascii_case("unknown"))
        {
            erros.insert(Erro::RotuloInvalido { lado, campo });
        }
    }
    let mut grupos = BTreeMap::<usize, Vec<_>>::new();
    for n in &s.grafo.nodes {
        grupos.entry(n.id).or_default().push(n);
    }
    for (&id, ns) in &grupos {
        if ns.len() > 1 {
            erros.insert(Erro::IdDuplicado { lado, id });
        }
    }
    for e in &s.grafo.edges {
        for (id, path) in [(e.id_from, &e.from), (e.id_to, &e.to)] {
            match grupos.get(&id) {
                None => {
                    erros.insert(Erro::ReferenciaInexistente {
                        lado,
                        id,
                        contexto: "aresta",
                    });
                }
                Some(ns) if ns.len() == 1 && &ns[0].path != path => {
                    erros.insert(Erro::EndpointDivergente {
                        lado,
                        id,
                        path: path.clone(),
                    });
                }
                _ => (),
            }
        }
    }
}
fn validar(
    a: &Snapshot,
    b: &Snapshot,
    pares: &[Correspondencia],
    mudancas: &[Mudanca],
) -> Vec<Erro> {
    let mut erros = BTreeSet::new();
    validar_grafo(a, Lado::Antes, &mut erros);
    validar_grafo(b, Lado::Depois, &mut erros);
    if a.granularidade != b.granularidade || a.natureza != b.natureza {
        erros.insert(Erro::EscoposIncompativeis);
    }
    let ids_a: BTreeSet<_> = a.grafo.nodes.iter().map(|n| n.id).collect();
    let ids_b: BTreeSet<_> = b.grafo.nodes.iter().map(|n| n.id).collect();
    let mut contagens = BTreeMap::<&Correspondencia, usize>::new();
    let mut identidades = BTreeMap::<&str, BTreeSet<&Correspondencia>>::new();
    let mut por_endpoint = BTreeMap::<(Lado, usize), BTreeSet<&Correspondencia>>::new();
    for p in pares {
        *contagens.entry(p).or_default() += 1;
        identidades.entry(&p.identidade).or_default().insert(p);
        for (lado, id, ids) in [
            (Lado::Antes, p.antes, &ids_a),
            (Lado::Depois, p.depois, &ids_b),
        ] {
            por_endpoint.entry((lado, id)).or_default().insert(p);
            if !ids.contains(&id) {
                erros.insert(Erro::ReferenciaInexistente {
                    lado,
                    id,
                    contexto: "correspondencia",
                });
            }
        }
    }
    for (p, count) in contagens {
        if count > 1 {
            erros.insert(Erro::CorrespondenciaDuplicada(p.clone()));
        }
    }
    for (identidade, ps) in &identidades {
        if ps.len() > 1 {
            erros.insert(Erro::IdentidadeReutilizada((*identidade).into()));
        }
    }
    for (&(lado, id), ps) in &por_endpoint {
        if ps.len() > 1 {
            erros.insert(Erro::EndpointReutilizado { lado, id });
        }
    }
    let mut por_mudanca = BTreeMap::<&str, BTreeSet<&Mudanca>>::new();
    for m in mudancas {
        match m {
            Mudanca::Remocao { antes: id } | Mudanca::Adicao { depois: id } => {
                let (lado, ids) = if matches!(m, Mudanca::Remocao { .. }) {
                    (Lado::Antes, &ids_a)
                } else {
                    (Lado::Depois, &ids_b)
                };
                if !ids.contains(id) {
                    erros.insert(Erro::ReferenciaInexistente {
                        lado,
                        id: *id,
                        contexto: "mudanca",
                    });
                }
                if por_endpoint.contains_key(&(lado, *id)) {
                    erros.insert(Erro::MudancaEmEndpointPareado { lado, id: *id });
                }
            }
            Mudanca::Pareada {
                identidade,
                alteracao_declarada,
                renomeacao_declarada,
            } => {
                if !identidades.contains_key(identidade.as_str()) {
                    erros.insert(Erro::IdentidadeNaoDeclarada(identidade.clone()));
                }
                if !alteracao_declarada && !renomeacao_declarada {
                    erros.insert(Erro::MudancaSemDeclaracao(identidade.clone()));
                }
                por_mudanca.entry(identidade).or_default().insert(m);
            }
        }
    }
    for (id, ms) in por_mudanca {
        if ms.len() > 1 {
            erros.insert(Erro::MudancasConflitantes(id.into()));
        }
    }
    erros.into_iter().collect()
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Chave {
    Pareada(String),
    SemPar(Lado, usize),
}
fn chave(e: &Elemento) -> Chave {
    match &e.identidade {
        Some(id) => Chave::Pareada(id.clone()),
        None => Chave::SemPar(e.origem.lado, e.id_local),
    }
}
struct Contexto<'a> {
    snapshot: &'a Snapshot,
    origem: Origem,
    elementos: BTreeMap<usize, Elemento>,
    por_identidade: BTreeMap<String, usize>,
    projecao: Grafo,
    ids_por_path: BTreeMap<Path, usize>,
}
impl<'a> Contexto<'a> {
    fn new(snapshot: &'a Snapshot, lado: Lado, pares: &[Correspondencia]) -> Self {
        let origem = Origem {
            lado,
            versao: snapshot.versao.clone(),
            completude: snapshot.completude,
        };
        let ids: BTreeMap<_, _> = pares
            .iter()
            .map(|p| {
                (
                    if lado == Lado::Antes {
                        p.antes
                    } else {
                        p.depois
                    },
                    p.identidade.clone(),
                )
            })
            .collect();
        let elementos = snapshot
            .grafo
            .nodes
            .iter()
            .map(|n| {
                (
                    n.id,
                    Elemento {
                        origem: origem.clone(),
                        id_local: n.id,
                        path: n.path.clone(),
                        identidade: ids.get(&n.id).cloned(),
                    },
                )
            })
            .collect();
        let por_identidade = ids.iter().map(|(id, key)| (key.clone(), *id)).collect();
        let mut projecao = snapshot.grafo.clone();
        let paths: BTreeMap<_, _> = projecao
            .nodes
            .iter()
            .map(|n| (n.id, Path::new(format!("local-snapshot-node-{}", n.id))))
            .collect();
        for n in &mut projecao.nodes {
            n.path = paths[&n.id].clone();
        }
        for e in &mut projecao.edges {
            e.from = paths[&e.id_from].clone();
            e.to = paths[&e.id_to].clone();
        }
        let ids_por_path = paths.into_iter().map(|(id, p)| (p, id)).collect();
        Self {
            snapshot,
            origem,
            elementos,
            por_identidade,
            projecao,
            ids_por_path,
        }
    }
    fn descricao(&self) -> DescricaoSnapshot {
        DescricaoSnapshot {
            origem: self.origem.clone(),
            granularidade: self.snapshot.granularidade.clone(),
            natureza: self.snapshot.natureza.clone(),
            diagnosticos: self
                .snapshot
                .diagnosticos
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }
    fn raio(&self, id: usize) -> Result<ImpactoSnapshot, Erro> {
        let raio = calcular_raio(
            &self.projecao,
            &Path::new(format!("local-snapshot-node-{id}")),
        )
        .map_err(|e| Erro::FalhaRaio {
            lado: self.origem.lado,
            id,
            motivo: e.to_string(),
        })?;
        let alcances = |map: BTreeMap<Path, usize>| {
            let mut result: Vec<_> = map
                .into_iter()
                .map(|(path, profundidade)| Alcance {
                    elemento: self.elementos[&self.ids_por_path[&path]].clone(),
                    profundidade,
                })
                .collect();
            result.sort_by_key(|a| chave(&a.elemento));
            result
        };
        Ok(ImpactoSnapshot {
            alvo: self.elementos[&id].clone(),
            classificacao: raio.classificacao,
            entradas_diretas: raio.uses_entrada,
            saidas_diretas: raio.uses_saida,
            consumidores: alcances(raio.montante.into_iter().collect()),
            dependencias: alcances(raio.jusante.into_iter().collect()),
        })
    }
    fn contraparte(&self, e: &Elemento) -> Option<Elemento> {
        e.identidade
            .as_ref()
            .and_then(|key| self.por_identidade.get(key))
            .map(|id| self.elementos[id].clone())
    }
    fn sem_par(&self) -> Vec<Elemento> {
        self.elementos
            .values()
            .filter(|e| e.identidade.is_none())
            .cloned()
            .collect()
    }
}
fn unilateral(
    c: &Alcance,
    outro: &Contexto<'_>,
    outro_raio: Option<&ImpactoSnapshot>,
) -> ObservacaoUnilateral {
    let contraparte = outro.contraparte(&c.elemento);
    let estado_outro_lado = if contraparte.is_none() {
        EstadoOutroLado::SemCorrespondencia
    } else if outro_raio.is_none() {
        EstadoOutroLado::AlvoNaoComparavel
    } else if outro.snapshot.completude != Completude::Completa {
        EstadoOutroLado::NaoObservadoComCoberturaLimitada
    } else {
        EstadoOutroLado::NaoAlcancadoNoGrafoCompleto
    };
    ObservacaoUnilateral {
        consumidor: c.clone(),
        contraparte,
        estado_outro_lado,
    }
}
fn consumidores(
    a: Option<&ImpactoSnapshot>,
    b: Option<&ImpactoSnapshot>,
    ca: &Contexto<'_>,
    cb: &Contexto<'_>,
) -> ConjuntosConsumidores {
    let mapa = |r: Option<&ImpactoSnapshot>| -> BTreeMap<Chave, Alcance> {
        r.into_iter()
            .flat_map(|r| r.consumidores.iter())
            .map(|c| (chave(&c.elemento), c.clone()))
            .collect()
    };
    let ma = mapa(a);
    let mb = mapa(b);
    let mut ambos = Vec::new();
    let mut somente_antes = Vec::new();
    let mut somente_depois = Vec::new();
    for (key, c) in &ma {
        if let Some(d) = mb.get(key) {
            if let Chave::Pareada(identidade) = key {
                ambos.push(ConsumidorNosDois {
                    identidade: identidade.clone(),
                    antes: c.clone(),
                    depois: d.clone(),
                });
            }
        } else {
            somente_antes.push(unilateral(c, cb, b));
        }
    }
    for (key, c) in &mb {
        if !ma.contains_key(key) {
            somente_depois.push(unilateral(c, ca, a));
        }
    }
    ConjuntosConsumidores {
        comparacao_bilateral: a.is_some() && b.is_some(),
        somente_antes,
        ambos,
        somente_depois,
    }
}
type ChaveRelacao = (Chave, Chave, Subtipo);
fn incidentes(c: &Contexto<'_>, alvo: usize) -> BTreeMap<ChaveRelacao, RelacaoObservada> {
    c.snapshot
        .grafo
        .edges
        .iter()
        .filter(|e| e.relation == Relation::Uses && (e.id_from == alvo || e.id_to == alvo))
        .map(|e| {
            let de = c.elementos[&e.id_from].clone();
            let para = c.elementos[&e.id_to].clone();
            let subtipo = match e.uses_kind {
                None => Subtipo::Desconhecido,
                Some(UsesKind::Import) => Subtipo::Import,
                Some(UsesKind::Reference) => Subtipo::Reference,
            };
            (
                (chave(&de), chave(&para), subtipo),
                RelacaoObservada { de, para, subtipo },
            )
        })
        .collect()
}
fn topologia(ca: &Contexto<'_>, cb: &Contexto<'_>, a: usize, b: usize) -> TopologiaIncidente {
    let ma = incidentes(ca, a);
    let mb = incidentes(cb, b);
    let endpoints_sem_par = ma
        .values()
        .chain(mb.values())
        .any(|e| e.de.identidade.is_none() || e.para.identidade.is_none());
    let cobertura_limitada = ca.snapshot.completude != Completude::Completa
        || cb.snapshot.completude != Completude::Completa;
    let mut comuns = Vec::new();
    let mut somente_antes = Vec::new();
    let mut somente_depois = Vec::new();
    for (key, e) in &ma {
        match mb.get(key) {
            Some(d) => comuns.push(RelacaoNosDois {
                antes: e.clone(),
                depois: d.clone(),
            }),
            None => somente_antes.push(e.clone()),
        }
    }
    for (key, e) in &mb {
        if !ma.contains_key(key) {
            somente_depois.push(e.clone());
        }
    }
    let status = if cobertura_limitada || endpoints_sem_par {
        StatusTopologia::ComparacaoLimitada
    } else if somente_antes.is_empty() && somente_depois.is_empty() {
        StatusTopologia::MesmasRelacoesObservadas
    } else {
        StatusTopologia::DiferencasNasRelacoesObservadas
    };
    TopologiaIncidente {
        comuns,
        somente_antes,
        somente_depois,
        cobertura_limitada,
        endpoints_sem_par,
        status,
    }
}
/// Compara somente mudanças declaradas. Não descobre renomes ou consulta Git.
///
/// Valida os dois grafos e TODO o mapa antes de calcular. Cada raio usa apenas
/// seu próprio snapshot. Ausência em extrações parciais permanece inconclusiva.
/// Veja CONTRATO.md para escopos, tipos de mudança e significado das categorias.
pub fn comparar_impacto(
    antes: &Snapshot,
    depois: &Snapshot,
    correspondencias: &[Correspondencia],
    mudancas: &[Mudanca],
) -> Result<ComparacaoImpacto, Vec<Erro>> {
    let erros = validar(antes, depois, correspondencias, mudancas);
    if !erros.is_empty() {
        return Err(erros);
    }
    let mut pares = correspondencias.to_vec();
    pares.sort();
    let ca = Contexto::new(antes, Lado::Antes, &pares);
    let cb = Contexto::new(depois, Lado::Depois, &pares);
    let mut resultados = Vec::new();
    for m in mudancas.iter().cloned().collect::<BTreeSet<_>>() {
        let (a, b) = match &m {
            Mudanca::Remocao { antes } => (Some(*antes), None),
            Mudanca::Adicao { depois } => (None, Some(*depois)),
            Mudanca::Pareada { identidade, .. } => (
                Some(ca.por_identidade[identidade]),
                Some(cb.por_identidade[identidade]),
            ),
        };
        let ra = a.map(|id| ca.raio(id)).transpose().map_err(|e| vec![e])?;
        let rb = b.map(|id| cb.raio(id)).transpose().map_err(|e| vec![e])?;
        let consumidores = consumidores(ra.as_ref(), rb.as_ref(), &ca, &cb);
        let topologia_incidente = match (a, b) {
            (Some(a), Some(b)) => Some(topologia(&ca, &cb, a, b)),
            _ => None,
        };
        resultados.push(ResultadoMudanca {
            mudanca: m,
            antes: ra,
            depois: rb,
            consumidores,
            topologia_incidente,
        });
    }
    Ok(ComparacaoImpacto {
        antes: ca.descricao(),
        depois: cb.descricao(),
        correspondencias: pares,
        sem_par_antes: ca.sem_par(),
        sem_par_depois: cb.sem_par(),
        mudancas: resultados,
        limites: vec![
            "Impacto é alcance estrutural observado; não prova quebra nem preservação comportamental.",
            "Somente antes/depois refere-se aos raios, não à existência dos consumidores nos snapshots.",
            "Ausência de observação em snapshot parcial ou desconhecido não prova ausência de dependência.",
            "Sem correspondência significa não pareado; IDs e paths iguais não implicam identidade entre versões.",
            "Renomeação declarada é independente das relações observadas. Topologia incidente não é topologia global.",
            "Versão, escopo, completude, mudanças e correspondências são declarações do chamador.",
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{Aresta, Kind, Modificadores, No, Visibility};
    fn snapshot(versao: &str, nos: &[(usize, &str)], arestas: &[(usize, usize)]) -> Snapshot {
        let nodes: Vec<_> = nos
            .iter()
            .map(|&(id, path)| No {
                id,
                path: Path::new(path),
                name: path.into(),
                kind: Kind::Fn,
                modificadores: Modificadores::default(),
                visibility: Visibility::Pub,
                crate_name: "example".into(),
                trait_: None,
                trait_ref: None,
                cfg: None,
                macro_kind: None,
                is_non_exhaustive: false,
                position: None,
            })
            .collect();
        let paths: BTreeMap<_, _> = nodes.iter().map(|n| (n.id, n.path.clone())).collect();
        let edges = arestas
            .iter()
            .map(|&(from, to)| Aresta {
                from: paths[&from].clone(),
                id_from: from,
                to: paths[&to].clone(),
                id_to: to,
                relation: Relation::Uses,
                uses_kind: Some(UsesKind::Reference),
            })
            .collect();
        Snapshot {
            versao: versao.into(),
            grafo: Grafo {
                crate_name: "example".into(),
                nodes,
                edges,
            },
            completude: Completude::Completa,
            granularidade: "item".into(),
            natureza: "resolved-reference".into(),
            diagnosticos: vec![],
        }
    }
    fn par(id: &str, a: usize, b: usize) -> Correspondencia {
        Correspondencia {
            identidade: id.into(),
            antes: a,
            depois: b,
        }
    }
    fn alteracao(id: &str) -> Mudanca {
        Mudanca::Pareada {
            identidade: id.into(),
            alteracao_declarada: true,
            renomeacao_declarada: false,
        }
    }
    fn rename(id: &str) -> Mudanca {
        Mudanca::Pareada {
            identidade: id.into(),
            alteracao_declarada: false,
            renomeacao_declarada: true,
        }
    }
    #[test]
    fn remocao_preserva_consumidores_diretos_e_transitivos_no_antes() {
        let a = snapshot(
            "v1",
            &[(0, "removed"), (1, "consumer"), (2, "transitive")],
            &[(1, 0), (2, 1)],
        );
        let b = snapshot("v2", &[(50, "consumer"), (90, "transitive")], &[(90, 50)]);
        let r = comparar_impacto(
            &a,
            &b,
            &[par("consumer", 1, 50), par("transitive", 2, 90)],
            &[Mudanca::Remocao { antes: 0 }],
        )
        .unwrap();
        let m = &r.mudancas[0];
        assert!(m.depois.is_none());
        assert!(!m.consumidores.comparacao_bilateral);
        let cs = &m.antes.as_ref().unwrap().consumidores;
        assert_eq!(cs.len(), 2);
        assert_eq!(cs[0].profundidade, 1);
        assert_eq!(cs[1].profundidade, 2);
        assert_eq!(cs[1].elemento.origem.versao, "v1");
        assert_eq!(cs[1].elemento.origem.lado, Lado::Antes);
        assert!(
            m.consumidores
                .somente_antes
                .iter()
                .all(|c| c.estado_outro_lado == EstadoOutroLado::AlvoNaoComparavel)
        );
        assert!(m.topologia_incidente.is_none());
    }
    #[test]
    fn adicao_calcula_somente_o_snapshot_posterior() {
        let a = snapshot("v1", &[(1, "consumer")], &[]);
        let b = snapshot("v2", &[(1, "new-target"), (2, "consumer")], &[(2, 1)]);
        let r = comparar_impacto(
            &a,
            &b,
            &[par("consumer", 1, 2)],
            &[Mudanca::Adicao { depois: 1 }],
        )
        .unwrap();
        let m = &r.mudancas[0];
        assert!(m.antes.is_none());
        assert_eq!(m.depois.as_ref().unwrap().consumidores.len(), 1);
        assert_eq!(m.depois.as_ref().unwrap().alvo.origem.versao, "v2");
        assert_eq!(
            m.consumidores.somente_depois[0].estado_outro_lado,
            EstadoOutroLado::AlvoNaoComparavel
        );
    }
    #[test]
    fn alteracao_separa_consumidores_antes_ambos_depois_por_identidade() {
        let a = snapshot(
            "old",
            &[
                (0, "target"),
                (1, "removed-consumer"),
                (2, "common"),
                (3, "added-consumer"),
            ],
            &[(1, 0), (2, 0)],
        );
        let b = snapshot(
            "new",
            &[
                (90, "target"),
                (11, "removed-consumer"),
                (12, "common"),
                (13, "added-consumer"),
            ],
            &[(12, 90), (13, 90)],
        );
        let ps = [
            par("target", 0, 90),
            par("removed", 1, 11),
            par("common", 2, 12),
            par("added", 3, 13),
        ];
        let r = comparar_impacto(&a, &b, &ps, &[alteracao("target")]).unwrap();
        let cs = &r.mudancas[0].consumidores;
        assert_eq!(cs.ambos[0].identidade, "common");
        assert_eq!(cs.ambos[0].antes.elemento.id_local, 2);
        assert_eq!(cs.ambos[0].depois.elemento.id_local, 12);
        assert_eq!(
            cs.somente_antes[0]
                .consumidor
                .elemento
                .identidade
                .as_deref(),
            Some("removed")
        );
        assert_eq!(
            cs.somente_depois[0]
                .consumidor
                .elemento
                .identidade
                .as_deref(),
            Some("added")
        );
        assert_eq!(
            cs.somente_antes[0].estado_outro_lado,
            EstadoOutroLado::NaoAlcancadoNoGrafoCompleto
        );
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::DiferencasNasRelacoesObservadas
        );
    }
    #[test]
    fn renomeacao_declarada_nao_inventa_mudanca_de_dependencias() {
        let a = snapshot("before", &[(4, "old/path"), (7, "caller")], &[(7, 4)]);
        let b = snapshot(
            "after",
            &[(200, "new.module"), (3, "renamed-caller")],
            &[(3, 200)],
        );
        let r = comparar_impacto(
            &a,
            &b,
            &[par("target", 4, 200), par("caller", 7, 3)],
            &[rename("target")],
        )
        .unwrap();
        let m = &r.mudancas[0];
        assert_eq!(m.mudanca, rename("target"));
        assert_eq!(m.consumidores.ambos.len(), 1);
        let t = m.topologia_incidente.as_ref().unwrap();
        assert_eq!(t.status, StatusTopologia::MesmasRelacoesObservadas);
        assert_eq!(t.comuns.len(), 1);
        assert_eq!(t.comuns[0].antes.para.path.as_str(), "old/path");
        assert_eq!(t.comuns[0].depois.para.path.as_str(), "new.module");
    }
    #[test]
    fn renomeacao_pode_coexistir_com_diferenca_estrutural() {
        let a = snapshot("a", &[(0, "old"), (1, "consumer")], &[(1, 0)]);
        let b = snapshot("b", &[(0, "new"), (1, "consumer")], &[]);
        let m = Mudanca::Pareada {
            identidade: "t".into(),
            alteracao_declarada: true,
            renomeacao_declarada: true,
        };
        let r = comparar_impacto(
            &a,
            &b,
            &[par("t", 0, 0), par("c", 1, 1)],
            std::slice::from_ref(&m),
        )
        .unwrap();
        assert_eq!(r.mudancas[0].mudanca, m);
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::DiferencasNasRelacoesObservadas
        );
    }
    #[test]
    fn sem_correspondencia_permanece_nao_pareado_mesmo_com_mesmo_id_e_path() {
        let a = snapshot("a", &[(0, "t"), (1, "same"), (2, "isolated")], &[(1, 0)]);
        let b = snapshot("b", &[(0, "t"), (1, "same"), (2, "isolated")], &[(1, 0)]);
        let r = comparar_impacto(&a, &b, &[par("target", 0, 0)], &[alteracao("target")]).unwrap();
        assert_eq!(r.sem_par_antes.len(), 2);
        assert_eq!(r.sem_par_depois.len(), 2);
        let cs = &r.mudancas[0].consumidores;
        assert!(cs.ambos.is_empty());
        assert_eq!(cs.somente_antes.len(), 1);
        assert_eq!(cs.somente_depois.len(), 1);
        assert_eq!(
            cs.somente_antes[0].estado_outro_lado,
            EstadoOutroLado::SemCorrespondencia
        );
        assert!(cs.somente_antes[0].contraparte.is_none());
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::ComparacaoLimitada
        );
    }
    #[test]
    fn ids_iguais_com_significados_diferentes_nao_decidem_pareamento() {
        let a = snapshot("a", &[(0, "target"), (1, "consumer")], &[(1, 0)]);
        let b = snapshot(
            "b",
            &[(0, "consumer-renamed"), (1, "target-renamed")],
            &[(0, 1)],
        );
        let r = comparar_impacto(
            &a,
            &b,
            &[par("target", 0, 1), par("consumer", 1, 0)],
            &[alteracao("target")],
        )
        .unwrap();
        let c = &r.mudancas[0].consumidores.ambos[0];
        assert_eq!(c.antes.elemento.id_local, 1);
        assert_eq!(c.depois.elemento.id_local, 0);
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::MesmasRelacoesObservadas
        );
    }
    #[test]
    fn parcial_limita_ausencia_sem_apagar_observacoes_positivas() {
        let a = snapshot("a", &[(0, "t"), (1, "c")], &[(1, 0)]);
        let mut b = snapshot("b", &[(7, "t"), (8, "c")], &[]);
        b.completude = Completude::Parcial;
        b.diagnosticos = vec!["falha no extrator".into()];
        let ps = [par("t", 0, 7), par("c", 1, 8)];
        let r = comparar_impacto(&a, &b, &ps, &[alteracao("t")]).unwrap();
        assert_eq!(
            r.mudancas[0].consumidores.somente_antes[0].estado_outro_lado,
            EstadoOutroLado::NaoObservadoComCoberturaLimitada
        );
        assert_eq!(r.depois.diagnosticos, vec!["falha no extrator"]);
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::ComparacaoLimitada
        );
        b.grafo.edges.push(Aresta {
            from: Path::new("c"),
            id_from: 8,
            to: Path::new("t"),
            id_to: 7,
            relation: Relation::Uses,
            uses_kind: Some(UsesKind::Reference),
        });
        let r = comparar_impacto(&a, &b, &ps, &[alteracao("t")]).unwrap();
        assert_eq!(r.mudancas[0].consumidores.ambos.len(), 1);
    }
    #[test]
    fn correspondencias_duplicadas_conflitantes_e_inexistentes_falham() {
        let a = snapshot("a", &[(0, "t"), (1, "c")], &[]);
        let b = a.clone();
        let p = par("t", 0, 0);
        let e = comparar_impacto(&a, &b, &[p.clone(), p.clone()], &[]).unwrap_err();
        assert!(e.contains(&Erro::CorrespondenciaDuplicada(p.clone())));
        let e = comparar_impacto(
            &a,
            &b,
            &[
                p.clone(),
                par("t", 1, 1),
                par("other", 0, 1),
                par("missing", 99, 98),
            ],
            &[],
        )
        .unwrap_err();
        assert!(e.contains(&Erro::IdentidadeReutilizada("t".into())));
        assert!(e.contains(&Erro::EndpointReutilizado {
            lado: Lado::Antes,
            id: 0
        }));
        assert!(e.contains(&Erro::EndpointReutilizado {
            lado: Lado::Depois,
            id: 1
        }));
        assert!(e.contains(&Erro::ReferenciaInexistente {
            lado: Lado::Antes,
            id: 99,
            contexto: "correspondencia"
        }));
        assert!(e.contains(&Erro::ReferenciaInexistente {
            lado: Lado::Depois,
            id: 98,
            contexto: "correspondencia"
        }));
    }
    #[test]
    fn ordem_das_entradas_nao_afeta_resultado_nem_erros() {
        let mut a = snapshot(
            "a",
            &[(0, "t"), (1, "c"), (2, "removed")],
            &[(1, 0), (1, 2)],
        );
        let mut b = snapshot("b", &[(7, "t"), (8, "c"), (9, "added")], &[(8, 7), (8, 9)]);
        let mut ps = vec![par("t", 0, 7), par("c", 1, 8)];
        let mut ms = vec![
            alteracao("t"),
            Mudanca::Remocao { antes: 2 },
            Mudanca::Adicao { depois: 9 },
        ];
        a.diagnosticos = vec!["z".into(), "a".into(), "z".into()];
        let expected = comparar_impacto(&a, &b, &ps, &ms).unwrap();
        a.grafo.nodes.reverse();
        a.grafo.edges.reverse();
        a.diagnosticos.reverse();
        b.grafo.nodes.reverse();
        b.grafo.edges.reverse();
        ps.reverse();
        ms.reverse();
        ms.push(ms[0].clone());
        assert_eq!(comparar_impacto(&a, &b, &ps, &ms).unwrap(), expected);
        ps.push(par("t", 1, 9));
        ps.push(par("missing", 99, 99));
        let expected = comparar_impacto(&a, &b, &ps, &ms).unwrap_err();
        ps.reverse();
        ms.reverse();
        assert_eq!(comparar_impacto(&a, &b, &ps, &ms).unwrap_err(), expected);
    }
    #[test]
    fn paths_colidentes_sao_preservados_por_id_sem_confundir_raio() {
        let a = snapshot(
            "a",
            &[(0, "collision"), (1, "collision"), (2, "caller")],
            &[(2, 0)],
        );
        let b = snapshot("b", &[], &[]);
        let r = comparar_impacto(
            &a,
            &b,
            &[],
            &[Mudanca::Remocao { antes: 0 }, Mudanca::Remocao { antes: 1 }],
        )
        .unwrap();
        assert_eq!(r.mudancas[0].antes.as_ref().unwrap().consumidores.len(), 1);
        assert!(
            r.mudancas[1]
                .antes
                .as_ref()
                .unwrap()
                .consumidores
                .is_empty()
        );
    }
    #[test]
    fn grafo_invalido_e_referencias_de_mudancas_sao_validados_antes_do_raio() {
        let mut a = snapshot("a", &[(0, "t"), (1, "c")], &[(1, 0)]);
        let b = a.clone();
        a.grafo.nodes.push(a.grafo.nodes[0].clone());
        a.grafo.edges[0].from = Path::new("wrong");
        a.grafo.edges[0].id_to = 99;
        let ms = [
            Mudanca::Remocao { antes: 88 },
            Mudanca::Adicao { depois: 77 },
            alteracao("undeclared"),
        ];
        let e = comparar_impacto(&a, &b, &[], &ms).unwrap_err();
        assert!(e.contains(&Erro::IdDuplicado {
            lado: Lado::Antes,
            id: 0
        }));
        assert!(e.contains(&Erro::EndpointDivergente {
            lado: Lado::Antes,
            id: 1,
            path: Path::new("wrong")
        }));
        assert!(e.contains(&Erro::ReferenciaInexistente {
            lado: Lado::Antes,
            id: 99,
            contexto: "aresta"
        }));
        assert!(e.contains(&Erro::IdentidadeNaoDeclarada("undeclared".into())));
        a.grafo.nodes.reverse();
        a.grafo.edges.reverse();
        assert_eq!(comparar_impacto(&a, &b, &[], &ms).unwrap_err(), e);
    }
    #[test]
    fn remocao_adicao_pareadas_e_declaracoes_conflitantes_falham() {
        let a = snapshot("a", &[(0, "t")], &[]);
        let b = a.clone();
        let ps = [par("t", 0, 0)];
        let ms = [
            Mudanca::Remocao { antes: 0 },
            Mudanca::Adicao { depois: 0 },
            alteracao("t"),
            rename("t"),
        ];
        let e = comparar_impacto(&a, &b, &ps, &ms).unwrap_err();
        assert!(e.contains(&Erro::MudancaEmEndpointPareado {
            lado: Lado::Antes,
            id: 0
        }));
        assert!(e.contains(&Erro::MudancaEmEndpointPareado {
            lado: Lado::Depois,
            id: 0
        }));
        assert!(e.contains(&Erro::MudancasConflitantes("t".into())));
        let m = Mudanca::Pareada {
            identidade: "t".into(),
            alteracao_declarada: false,
            renomeacao_declarada: false,
        };
        assert!(
            comparar_impacto(&a, &b, &ps, &[m])
                .unwrap_err()
                .contains(&Erro::MudancaSemDeclaracao("t".into()))
        );
    }
    #[test]
    fn topologia_incidente_nao_oculta_alteracao_de_consumidores_transitivos() {
        let a = snapshot(
            "a",
            &[(0, "t"), (1, "middle"), (2, "consumer")],
            &[(1, 0), (2, 1)],
        );
        let b = snapshot("b", &[(0, "t"), (1, "middle"), (2, "consumer")], &[(1, 0)]);
        let ps = [par("t", 0, 0), par("middle", 1, 1), par("consumer", 2, 2)];
        let r = comparar_impacto(&a, &b, &ps, &[alteracao("t")]).unwrap();
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::MesmasRelacoesObservadas
        );
        assert_eq!(
            r.mudancas[0].consumidores.somente_antes[0]
                .consumidor
                .elemento
                .identidade
                .as_deref(),
            Some("consumer")
        );
        assert_eq!(
            r.mudancas[0].consumidores.somente_antes[0]
                .consumidor
                .profundidade,
            2
        );
    }
    #[test]
    fn profundidades_mudam_sem_perder_o_pareamento_de_consumidores() {
        let a = snapshot(
            "a",
            &[(0, "t"), (1, "middle"), (2, "consumer")],
            &[(1, 0), (2, 1)],
        );
        let b = snapshot(
            "b",
            &[(0, "t"), (1, "middle"), (2, "consumer")],
            &[(1, 0), (2, 0)],
        );
        let r = comparar_impacto(
            &a,
            &b,
            &[par("t", 0, 0), par("m", 1, 1), par("c", 2, 2)],
            &[alteracao("t")],
        )
        .unwrap();
        let c = r.mudancas[0]
            .consumidores
            .ambos
            .iter()
            .find(|c| c.identidade == "c")
            .unwrap();
        assert_eq!(c.antes.profundidade, 2);
        assert_eq!(c.depois.profundidade, 1);
    }
    #[test]
    fn ciclos_paralelas_e_owns_preservam_as_regras_do_raio_existente() {
        let mut a = snapshot(
            "a",
            &[(0, "t"), (1, "consumer"), (2, "owner")],
            &[(1, 0), (0, 1), (1, 0)],
        );
        a.grafo.edges.push(Aresta {
            from: Path::new("owner"),
            id_from: 2,
            to: Path::new("t"),
            id_to: 0,
            relation: Relation::Owns,
            uses_kind: None,
        });
        let b = snapshot("b", &[], &[]);
        let r = comparar_impacto(&a, &b, &[], &[Mudanca::Remocao { antes: 0 }]).unwrap();
        let raio = r.mudancas[0].antes.as_ref().unwrap();
        assert_eq!(raio.consumidores.len(), 1);
        assert_eq!(raio.entradas_diretas, 1);
        assert_eq!(raio.dependencias.len(), 1);
        assert_eq!(raio.consumidores[0].elemento.id_local, 1);
    }
    #[test]
    fn subtipo_direto_nao_se_perde_em_conjuntos_de_consumidores_iguais() {
        let a = snapshot("a", &[(0, "t"), (1, "c")], &[(1, 0)]);
        let mut b = a.clone();
        b.versao = "b".into();
        b.grafo.edges[0].uses_kind = Some(UsesKind::Import);
        let r =
            comparar_impacto(&a, &b, &[par("t", 0, 0), par("c", 1, 1)], &[alteracao("t")]).unwrap();
        assert_eq!(r.mudancas[0].consumidores.ambos.len(), 1);
        let t = r.mudancas[0].topologia_incidente.as_ref().unwrap();
        assert_eq!(t.status, StatusTopologia::DiferencasNasRelacoesObservadas);
        assert_eq!(t.somente_antes[0].subtipo, Subtipo::Reference);
        assert_eq!(t.somente_depois[0].subtipo, Subtipo::Import);
    }
    #[test]
    fn identidades_opacas_inclusive_vazia_e_renome_nao_inferido() {
        let a = snapshot("a", &[(0, "old"), (1, "caller")], &[(1, 0)]);
        let b = snapshot("b", &[(0, "new"), (1, "different")], &[(1, 0)]);
        let r = comparar_impacto(
            &a,
            &b,
            &[par("", 0, 0), par("/α::opaque", 1, 1)],
            &[alteracao("")],
        )
        .unwrap();
        assert_eq!(r.mudancas[0].mudanca, alteracao(""));
        assert_eq!(r.mudancas[0].consumidores.ambos[0].identidade, "/α::opaque");
    }
    #[test]
    fn sem_mudancas_nao_infere_adicoes_remocoes_nem_renomes() {
        let a = snapshot("a", &[(0, "old")], &[]);
        let b = snapshot("b", &[(0, "new")], &[]);
        let r = comparar_impacto(&a, &b, &[], &[]).unwrap();
        assert!(r.mudancas.is_empty());
        assert_eq!(r.sem_par_antes.len(), 1);
        assert_eq!(r.sem_par_depois.len(), 1);
    }
    #[test]
    fn nao_cria_caminho_que_so_existiria_na_uniao_dos_snapshots() {
        let a = snapshot(
            "a",
            &[(0, "target"), (1, "middle"), (2, "consumer")],
            &[(2, 1)],
        );
        let b = snapshot(
            "b",
            &[(0, "target"), (1, "middle"), (2, "consumer")],
            &[(1, 0)],
        );
        let ps = [par("t", 0, 0), par("m", 1, 1), par("c", 2, 2)];
        let r = comparar_impacto(&a, &b, &ps, &[alteracao("t")]).unwrap();
        assert!(
            r.mudancas[0]
                .antes
                .as_ref()
                .unwrap()
                .consumidores
                .is_empty()
        );
        let novos = &r.mudancas[0].depois.as_ref().unwrap().consumidores;
        assert_eq!(novos.len(), 1);
        assert_eq!(novos[0].elemento.identidade.as_deref(), Some("m"));
    }
    #[test]
    fn cobertura_limitada_e_simetrica_e_inclui_desconhecida() {
        let mut a = snapshot("a", &[(0, "t"), (1, "c")], &[]);
        a.completude = Completude::Desconhecida;
        let b = snapshot("b", &[(0, "t"), (1, "c")], &[(1, 0)]);
        let r =
            comparar_impacto(&a, &b, &[par("t", 0, 0), par("c", 1, 1)], &[alteracao("t")]).unwrap();
        assert_eq!(
            r.mudancas[0].consumidores.somente_depois[0].estado_outro_lado,
            EstadoOutroLado::NaoObservadoComCoberturaLimitada
        );
        assert_eq!(
            r.mudancas[0].topologia_incidente.as_ref().unwrap().status,
            StatusTopologia::ComparacaoLimitada
        );
    }
    #[test]
    fn snapshots_incompativeis_e_rotulos_vazios_sao_rejeitados() {
        let a = snapshot("a", &[], &[]);
        let mut b = snapshot("b", &[], &[]);
        b.granularidade = "file".into();
        assert!(
            comparar_impacto(&a, &b, &[], &[])
                .unwrap_err()
                .contains(&Erro::EscoposIncompativeis)
        );
        b = a.clone();
        b.versao.clear();
        assert!(
            comparar_impacto(&a, &b, &[], &[])
                .unwrap_err()
                .contains(&Erro::RotuloInvalido {
                    lado: Lado::Depois,
                    campo: "versao"
                })
        );
    }
}
