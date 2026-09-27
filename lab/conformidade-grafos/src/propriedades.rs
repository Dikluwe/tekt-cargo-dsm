//! Crystalline Lineage
//! @prompt lab/conformidade-grafos/CONTRATO.md
//! @layer Lab
//! @updated 2026-09-26
//! Somente testes: gerador v1 e oráculo cúbico para no máximo nove nós.

use super::*;
use lente_core::entities::grafo::{Aresta, Relation, UsesKind};

struct SplitMix64(u64);
impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn index(&mut self, len: usize) -> usize {
        (self.next() % len as u64) as usize
    }
    fn shuffle<T>(&mut self, values: &mut [T]) {
        for i in (1..values.len()).rev() {
            let j = self.index(i + 1);
            values.swap(i, j);
        }
    }
}
fn seeds() -> Vec<u64> {
    match std::env::var("LENTE_SEMENTE") {
        Ok(s) => vec![s.parse().expect("LENTE_SEMENTE deve ser u64 decimal")],
        Err(std::env::VarError::NotPresent) => corpus(),
        Err(e) => panic!("LENTE_SEMENTE inválida: {e}"),
    }
}
fn corpus() -> Vec<u64> {
    serde_json::from_str(include_str!("../sementes.json")).unwrap()
}
fn edge(g: &Grafo, from: usize, to: usize, relation: Relation, kind: Option<UsesKind>) -> Aresta {
    Aresta {
        id_from: g.nodes[from].id,
        from: g.nodes[from].path.clone(),
        id_to: g.nodes[to].id,
        to: g.nodes[to].path.clone(),
        relation,
        uses_kind: kind,
    }
}
fn generate(seed: u64) -> Grafo {
    let n = (seed % 9) as usize;
    let mode = (seed / 9) % 4;
    let nodes:Vec<_>=(0..n).map(|i|json!({"id":10+i*7,"path":format!("arquivo/{i:02}.mod"),"name":format!("n{i}"),"kind":"mod","visibility":"priv"})).collect();
    let mut g = lente_infra::desserializar_grafo(
        &json!({"crate":"gerador-v1","nodes":nodes,"edges":[]}).to_string(),
    )
    .unwrap();
    let mut rng = SplitMix64(seed);
    for a in 0..n {
        for b in 0..n {
            let sample = rng.next();
            let enabled = match mode {
                0 => sample.is_multiple_of(5),
                1 => !sample.is_multiple_of(3),
                2 => (a < n / 2) == (b < n / 2) && sample.is_multiple_of(3),
                _ => a + 1 < n && b + 1 < n && (b == (a + 1) % (n - 1) || a == b),
            };
            if enabled {
                let kind = if sample & 1 == 0 {
                    UsesKind::Import
                } else {
                    UsesKind::Reference
                };
                g.edges.push(edge(&g, a, b, Relation::Uses, Some(kind)));
            }
        }
    }
    // Pai sempre anterior ao filho: nenhum múltiplo pai/ciclo de contenção.
    for child in 1..n {
        if rng.next().is_multiple_of(3) {
            let parent = rng.index(child);
            g.edges.push(edge(&g, parent, child, Relation::Owns, None));
        }
    }
    g
}
fn graph_json(g: &Grafo) -> Value {
    json!({"crate":g.crate_name,
        "nodes":g.nodes.iter().map(|n|json!({"id":n.id,"path":n.path.as_str(),"name":n.name,"kind":"mod","visibility":"priv"})).collect::<Vec<_>>(),
        "edges":g.edges.iter().map(|e|json!({"id_from":e.id_from,"from":e.from.as_str(),"id_to":e.id_to,"to":e.to.as_str(),"relation":match e.relation {Relation::Uses=>"uses",Relation::Owns=>"owns"},"uses_kind":e.uses_kind.map(|k|match k {UsesKind::Import=>"import",UsesKind::Reference=>"reference"})})).collect::<Vec<_>>()})
}
fn identity(g: &Grafo) -> BTreeMap<usize, usize> {
    g.nodes.iter().map(|n| (n.id, n.id)).collect()
}

// Floyd–Warshall: nenhuma fila, BFS, cálculo de raio ou SCC da produção.
// Diagonal zero modela distância reflexiva; o alvo é removido ao projetar o raio.
fn oracle(g: &Grafo, inverse: &BTreeMap<usize, usize>) -> Value {
    let n = g.nodes.len();
    let infinity = n + 1;
    let index: BTreeMap<_, _> = g
        .nodes
        .iter()
        .enumerate()
        .map(|(i, node)| (node.id, i))
        .collect();
    let mut distance = vec![vec![infinity; n]; n];
    let mut incoming = vec![BTreeSet::new(); n];
    let mut outgoing = incoming.clone();
    let mut parents = vec![None; n];
    let mut children = incoming.clone();
    for (i, row) in distance.iter_mut().enumerate() {
        row[i] = 0;
    }
    for e in &g.edges {
        let a = index[&e.id_from];
        let b = index[&e.id_to];
        match e.relation {
            Relation::Uses => {
                distance[a][b] = distance[a][b].min(1);
                outgoing[a].insert(b);
                incoming[b].insert(a);
            }
            Relation::Owns => {
                parents[b] = Some(inverse[&e.id_from]);
                children[a].insert(inverse[&e.id_to]);
            }
        }
    }
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                distance[i][j] = distance[i][j].min(distance[i][k] + distance[k][j]);
            }
        }
    }
    let mut nodes = BTreeMap::new();
    let mut ranking = BTreeMap::new();
    for i in 0..n {
        let upstream: BTreeMap<_, _> = (0..n)
            .filter(|&j| j != i && distance[j][i] < infinity)
            .map(|j| (inverse[&g.nodes[j].id], distance[j][i]))
            .collect();
        let downstream: BTreeMap<_, _> = (0..n)
            .filter(|&j| j != i && distance[i][j] < infinity)
            .map(|j| (inverse[&g.nodes[j].id], distance[i][j]))
            .collect();
        let class = match (incoming[i].is_empty(), outgoing[i].is_empty()) {
            (true, true) => "Isolado",
            (true, false) => "Folha",
            (false, true) => "Base",
            (false, false) => "Intermediario",
        };
        let id = inverse[&g.nodes[i].id];
        ranking.insert(id, (upstream.len(), class));
        nodes.insert(id,json!({"classificacao":class,"entrada":incoming[i].len(),"saida":outgoing[i].len(),
            "profundidade_montante":upstream.values().copied().max().unwrap_or(0),"profundidade_jusante":downstream.values().copied().max().unwrap_or(0),
            "montante":upstream,"jusante":downstream,"pai":parents[i],"filhos":children[i]}));
    }
    // SCC por equivalência de alcançabilidade mútua; não é Tarjan.
    let mut cycles = BTreeSet::new();
    for (i, row) in distance.iter().enumerate() {
        let group: BTreeSet<_> = (0..n)
            .filter(|&j| row[j] < infinity && distance[j][i] < infinity)
            .map(|j| inverse[&g.nodes[j].id])
            .collect();
        if group.len() > 1 {
            cycles.insert(group);
        }
    }
    json!({"nos":nodes,"ranking":ranking,"ciclos":cycles})
}

struct Case<'a> {
    seed: u64,
    property: &'a str,
    original: &'a Grafo,
    changed: &'a Grafo,
    inverse: &'a BTreeMap<usize, usize>,
}
impl Case<'_> {
    fn report(&self, detail: &str) -> String {
        format!(
            "gerador=v1 semente={} propriedade={}\nRepetir: LENTE_SEMENTE={} cargo test --offline --locked --manifest-path lab/conformidade-grafos/Cargo.toml propriedades::{} -- --exact --nocapture\nORIGINAL:\n{}\nTRANSFORMAÇÃO:\n{}\nMAPA INVERSO:\n{:?}\nDIVERGÊNCIA:\n{}",
            self.seed,
            self.property,
            self.seed,
            self.property,
            serde_json::to_string_pretty(&graph_json(self.original)).unwrap(),
            serde_json::to_string_pretty(&graph_json(self.changed)).unwrap(),
            self.inverse,
            detail
        )
    }
    fn fail(&self, detail: &str) -> ! {
        panic!("{}", self.report(detail));
    }
    fn equal(&self, label: &str, expected: &Value, actual: &Value) {
        if expected != actual {
            self.fail(&format!("{label}\nesperado={expected}\nobtido={actual}"));
        }
    }
    fn actual(&self, g: &Grafo, inverse: &BTreeMap<usize, usize>) -> Value {
        if let Err(e) = lente_infra::desserializar_grafo(&graph_json(g).to_string()) {
            self.fail(&format!("gerador produziu grafo inválido: {e}"));
        }
        let actual = std::panic::catch_unwind(|| assinatura(g, inverse))
            .unwrap_or_else(|payload| {
                let detail = payload
                    .downcast_ref::<String>()
                    .map(String::as_str)
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("panic sem texto");
                self.fail(&format!("panic da API de produção: {detail}"))
            })
            .unwrap_or_else(|e| self.fail(&format!("API: {e}")));
        let expected = oracle(g, inverse);
        for field in ["nos", "ranking", "ciclos"] {
            self.equal(
                &format!("oráculo/{field}"),
                &expected[field],
                &actual[field],
            );
        }
        actual
    }
}

fn transform(g: &Grafo, seed: u64, property: usize) -> (Grafo, BTreeMap<usize, usize>) {
    let mut changed = g.clone();
    let mut inverse = identity(g);
    let mut rng = SplitMix64(seed ^ 0xc0ffee);
    match property {
        1 => {
            rng.shuffle(&mut changed.nodes);
            rng.shuffle(&mut changed.edges);
        }
        2 => {
            let mut slots: Vec<_> = (0..g.nodes.len()).collect();
            rng.shuffle(&mut slots);
            let map: BTreeMap<_, _> = g
                .nodes
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    (
                        n.id,
                        (
                            1009 + slots[i] * 17,
                            Path::from(format!("renomeado/{:02}.mod", g.nodes.len() - slots[i])),
                        ),
                    )
                })
                .collect();
            inverse = map.iter().map(|(old, (new, _))| (*new, *old)).collect();
            for node in &mut changed.nodes {
                let (id, path) = &map[&node.id];
                node.id = *id;
                node.path = path.clone();
                node.name = path.to_string();
            }
            for e in &mut changed.edges {
                let (id, path) = &map[&e.id_from];
                e.id_from = *id;
                e.from = path.clone();
                let (id, path) = &map[&e.id_to];
                e.id_to = *id;
                e.to = path.clone();
            }
            rng.shuffle(&mut changed.nodes);
            rng.shuffle(&mut changed.edges);
        }
        3 => changed.edges.extend(g.edges.iter().cloned()),
        4 => {
            let node=lente_infra::desserializar_grafo(r#"{"crate":"gerador-v1","nodes":[{"id":99999,"path":"isolado/novo.mod","name":"novo","kind":"mod","visibility":"priv"}],"edges":[]}"#).unwrap().nodes.remove(0);
            inverse.insert(node.id, node.id);
            changed.nodes.push(node);
        }
        5 | 6 => {
            let candidates: Vec<_> = (0..g.nodes.len())
                .flat_map(|a| (0..g.nodes.len()).map(move |b| (a, b)))
                .filter(|(a, b)| {
                    !g.edges.iter().any(|e| {
                        e.relation == Relation::Uses
                            && e.id_from == g.nodes[*a].id
                            && e.id_to == g.nodes[*b].id
                    })
                })
                .collect();
            if !candidates.is_empty() {
                let (a, b) = candidates[rng.index(candidates.len())];
                changed
                    .edges
                    .push(edge(g, a, b, Relation::Uses, Some(UsesKind::Import)));
            }
        }
        7 => {
            changed
                .edges
                .retain(|e| e.relation != Relation::Uses || rng.next().is_multiple_of(2));
        }
        8 => {
            for e in &mut changed.edges {
                if e.relation == Relation::Uses {
                    std::mem::swap(&mut e.from, &mut e.to);
                    std::mem::swap(&mut e.id_from, &mut e.id_to);
                }
            }
        }
        9 => {
            for child in 1..g.nodes.len() {
                if !g
                    .edges
                    .iter()
                    .any(|e| e.relation == Relation::Owns && e.id_to == g.nodes[child].id)
                {
                    changed
                        .edges
                        .push(edge(g, child - 1, child, Relation::Owns, None));
                }
            }
        }
        _ => unreachable!(),
    }
    (changed, inverse)
}

fn check(property: usize, name: &str) {
    for seed in seeds() {
        let g = generate(seed);
        let (changed, inverse) = transform(&g, seed, property);
        let case = Case {
            seed,
            property: name,
            original: &g,
            changed: &changed,
            inverse: &inverse,
        };
        let before = case.actual(&g, &identity(&g));
        let after = case.actual(&changed, &inverse);
        match property {
            1 | 2 => case.equal("assinatura semântica", &before, &after),
            3 => {
                for field in ["nos", "ranking", "ciclos"] {
                    case.equal(field, &before[field], &after[field]);
                }
            }
            4 => {
                for n in &g.nodes {
                    let id = n.id.to_string();
                    for field in ["nos", "ranking"] {
                        case.equal(
                            &format!("isolado/{field}/{id}"),
                            &before[field][&id],
                            &after[field][&id],
                        );
                    }
                }
                case.equal("isolado/ciclos", &before["ciclos"], &after["ciclos"]);
            }
            5..=7 => {
                for n in &g.nodes {
                    let id = n.id.to_string();
                    for direction in ["montante", "jusante"] {
                        let (subset, superset) = if property == 7 {
                            (
                                &after["nos"][&id][direction],
                                &before["nos"][&id][direction],
                            )
                        } else {
                            (
                                &before["nos"][&id][direction],
                                &after["nos"][&id][direction],
                            )
                        };
                        for (target, distance) in subset.as_object().unwrap() {
                            let found = &superset[target];
                            if found.is_null()
                                || property == 6
                                    && found.as_u64().unwrap() > distance.as_u64().unwrap()
                            {
                                case.fail(&format!("nó={id} direção={direction} destino={target} antes={} depois={}",before["nos"][&id][direction],after["nos"][&id][direction]));
                            }
                        }
                    }
                }
            }
            8 => {
                for n in &g.nodes {
                    let id = n.id.to_string();
                    for (a, b) in [
                        ("montante", "jusante"),
                        ("jusante", "montante"),
                        ("entrada", "saida"),
                        ("saida", "entrada"),
                        ("profundidade_montante", "profundidade_jusante"),
                        ("profundidade_jusante", "profundidade_montante"),
                    ] {
                        case.equal(
                            &format!("inversão/{id}/{a}"),
                            &before["nos"][&id][a],
                            &after["nos"][&id][b],
                        );
                    }
                }
            }
            9 => {
                for n in &g.nodes {
                    let id = n.id.to_string();
                    for field in [
                        "montante",
                        "jusante",
                        "entrada",
                        "saida",
                        "classificacao",
                        "profundidade_montante",
                        "profundidade_jusante",
                    ] {
                        case.equal(
                            &format!("Owns/{id}/{field}"),
                            &before["nos"][&id][field],
                            &after["nos"][&id][field],
                        );
                    }
                }
                for field in ["ranking", "ciclos"] {
                    case.equal(field, &before[field], &after[field]);
                }
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn permutacao() {
    check(1, "permutacao");
}
#[test]
fn renomeacao_bijetiva() {
    check(2, "renomeacao_bijetiva");
}
#[test]
fn duplicacao() {
    check(3, "duplicacao");
}
#[test]
fn isolado() {
    check(4, "isolado");
}
#[test]
fn adicao_preserva_alcance() {
    check(5, "adicao_preserva_alcance");
}
#[test]
fn adicao_nao_aumenta_distancias() {
    check(6, "adicao_nao_aumenta_distancias");
}
#[test]
fn remocao_nao_cria_caminhos() {
    check(7, "remocao_nao_cria_caminhos");
}
#[test]
fn inversao() {
    check(8, "inversao");
}
#[test]
fn owns_nao_propaga() {
    check(9, "owns_nao_propaga");
}

#[test]
fn oraculo_confere_cadeia_atalho_ciclo_e_autoaresta() {
    let mut chain =
        lente_infra::desserializar_grafo(include_str!("../fixtures/cadeia.json")).unwrap();
    let original = oracle(&chain, &identity(&chain));
    assert_eq!(original["nos"]["1"]["jusante"], json!({"2":1,"3":2}));
    assert_eq!(original["nos"]["3"]["montante"], json!({"1":2,"2":1}));
    chain.edges.push(edge(&chain, 0, 2, Relation::Uses, None));
    assert_eq!(
        oracle(&chain, &identity(&chain))["nos"]["1"]["jusante"]["3"],
        1
    );
    let cycle = lente_infra::desserializar_grafo(include_str!("../fixtures/ciclo.json")).unwrap();
    let expected = oracle(&cycle, &identity(&cycle));
    assert_eq!(expected["ciclos"], json!([[1, 2, 3]]));
    assert_eq!(expected["nos"]["1"]["montante"], json!({"2":2,"3":1}));
    assert_eq!(expected["nos"]["1"]["jusante"], json!({"2":1,"3":2}));
    let mut single =
        lente_infra::desserializar_grafo(include_str!("../fixtures/isolado.json")).unwrap();
    single.edges.push(edge(&single, 0, 0, Relation::Uses, None));
    let expected = oracle(&single, &identity(&single));
    let id = single.nodes[0].id.to_string();
    assert_eq!(expected["nos"][&id]["entrada"], 1);
    assert_eq!(expected["nos"][&id]["saida"], 1);
    assert_eq!(expected["nos"][&id]["montante"], json!({}));
    assert_eq!(expected["nos"][&id]["jusante"], json!({}));
    assert_eq!(expected["nos"][&id]["classificacao"], "Intermediario");
    assert_eq!(expected["ciclos"], json!([]));
}

#[test]
fn gerador_v1_e_corpus_cobrem_casos_nao_vazios() {
    let mut rng = SplitMix64(0);
    assert_eq!(rng.next(), 0xe220a8397b1dcdaf);
    assert_eq!(rng.next(), 0x6e789e6aa1b965f4);
    let seeds = corpus();
    assert_eq!(seeds.len(), 64);
    assert_eq!(
        seeds.iter().copied().collect::<BTreeSet<_>>().len(),
        seeds.len()
    );
    let mut sizes = BTreeSet::new();
    let mut changed_count = [0usize; 9];
    let (mut loops, mut cycles, mut disconnected, mut grown, mut shortened, mut shrunk) =
        (0, 0, 0, 0, 0, 0);
    for seed in seeds {
        let g = generate(seed);
        sizes.insert(g.nodes.len());
        assert_eq!(
            graph_json(&g),
            graph_json(&generate(seed)),
            "semente {seed}"
        );
        loops += usize::from(
            g.edges
                .iter()
                .any(|e| e.relation == Relation::Uses && e.id_from == e.id_to),
        );
        let before = oracle(&g, &identity(&g));
        cycles += usize::from(!before["ciclos"].as_array().unwrap().is_empty());
        if (seed / 9) % 4 == 2 && g.nodes.len() >= 4 {
            let half = g.nodes.len() / 2;
            assert!(!g.edges.iter().any(|e| e.relation == Relation::Uses
                && ((e.id_from - 10) / 7 < half) != ((e.id_to - 10) / 7 < half)));
            disconnected += 1;
        }
        for property in 1..=9 {
            let (changed, inverse) = transform(&g, seed, property);
            changed_count[property - 1] += usize::from(graph_json(&g) != graph_json(&changed));
            if ![5, 6, 7].contains(&property) {
                continue;
            }
            let after = oracle(&changed, &inverse);
            for n in &g.nodes {
                let id = n.id.to_string();
                let a = before["nos"][&id]["jusante"].as_object().unwrap();
                let b = after["nos"][&id]["jusante"].as_object().unwrap();
                if property == 5 && b.len() > a.len() {
                    grown += 1;
                }
                if property == 7 && b.len() < a.len() {
                    shrunk += 1;
                }
                if property == 6 {
                    for (key, d) in a {
                        if b.get(key).is_some_and(|v| v.as_u64() < d.as_u64()) {
                            shortened += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(sizes, (0..=8).collect());
    assert!(changed_count.iter().all(|n| *n > 0), "{changed_count:?}");
    assert!(
        loops > 0 && cycles > 0 && disconnected > 0 && grown > 0 && shortened > 0 && shrunk > 0
    );
    println!(
        "corpus v1: autoarestas={loops}, ciclos={cycles}, partições desconectadas={disconnected}; transformações efetivas={changed_count:?}; alcances ampliados={grown}, distâncias reduzidas={shortened}, alcances reduzidos={shrunk}"
    );
}

#[test]
fn diagnostico_contem_reproducao_grafos_e_divergencia() {
    let g = generate(42);
    let (changed, inverse) = transform(&g, 42, 2);
    let case = Case {
        seed: 42,
        property: "renomeacao_bijetiva",
        original: &g,
        changed: &changed,
        inverse: &inverse,
    };
    let report = case.report("esperado=1 obtido=2");
    for text in [
        "semente=42",
        "LENTE_SEMENTE=42",
        "propriedades::renomeacao_bijetiva -- --exact --nocapture",
        "ORIGINAL:",
        "TRANSFORMAÇÃO:",
        "MAPA INVERSO:",
        "DIVERGÊNCIA:",
        "esperado=1 obtido=2",
    ] {
        assert!(report.contains(text), "{text}");
    }
    assert!(report.contains(&serde_json::to_string_pretty(&graph_json(&g)).unwrap()));
    assert!(report.contains(&serde_json::to_string_pretty(&graph_json(&changed)).unwrap()));
}
