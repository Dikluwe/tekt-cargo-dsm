//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/estrutura.md
//! @prompt-hash 7a8a7c30
//! @layer L1
//! @updated 2026-11-26
//! Spec:    00_nucleo/specs/forma-organizada.md
//! Camada:  L1 — Núcleo. Pureza: stdlib + `lente_core`. Zero externas.
//!
//! Primeiro tijolo da **vista global** (estilo Lattix LDM / Structure101): a
//! `lente_estrutura` agrega o grafo de itens ao nível de **módulo** e
//! detecta **ciclos** (componentes fortemente conexos ≥ 2) entre módulos.
//!
//! Duas operações puras, ambas sobre `Grafo`:
//!
//! - [`agregar_por_modulo`]: grafo de itens → grafo de módulos (pelo
//!   contenedor `Owns`); arestas `Uses` viram dependências módulo→módulo
//!   (uses intra-módulo são absorvidos — não viram aresta).
//! - [`detectar_ciclos`]: SCC à mão (Tarjan, sem `petgraph`) sobre as
//!   arestas `Uses` do grafo recebido; devolve os SCCs de tamanho ≥ 2.
//!   **Genérico**: funciona sobre qualquer `Grafo` (item, módulo, crate);
//!   o nível em que se aplica é decisão do chamador.
//!
//! ## Sobre o fractal (horizonte)
//!
//! Em qualquer nível o grafo tem as mesmas duas relações (contém e usa); a
//! agregação produz um `Grafo` e a detecção opera sobre qualquer `Grafo`,
//! então a mesma peça serve **crate-a-crate** (workspace) e **item** quando
//! a próxima trilha aparecer. Este crate não constrói navegação multi-nível
//! agora — fica como aplicação posterior das mesmas duas funções. (Prompt
//! 0031, "não estruturar antes do uso pedir" aplicado ao zoom.)

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap, HashSet};

use lente_core::entities::grafo::{Aresta, Grafo, Kind, No, Path, Relation};

/// Um ciclo entre módulos: o conjunto de módulos (paths) que formam o SCC.
///
/// **Determinismo**: `modulos` é uma lista ordenada lexicograficamente. A
/// ordem entre ciclos diferentes (no `Vec<Ciclo>` devolvido por
/// [`detectar_ciclos`]) também é determinística — ordenada pelo primeiro
/// membro de cada ciclo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ciclo {
    pub modulos: Vec<Path>,
}

/// Dependência módulo→módulo no resultado do modo estrutura.
///
/// Formato pensado para o JSON DSM-friendly (prompt 0031): pares
/// `{de, para}` deduplicados, ordenados deterministicamente. A
/// representação em si é apenas dois paths — a forma que uma DSM
/// futura consome (linhas e colunas).
///
/// Movido do `lente_wiring` (L4) para cá no Estágio 2 (refactor V3+V12, 0056):
/// é dado de estrutura puro e mora junto do [`Ciclo`] que o [`EstruturaModulos`]
/// referencia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenciaModulo {
    pub de: Path,
    pub para: Path,
    /// **Peso de acoplamento** (prompt 0071; Achado 1 do laudo 0036): quantas
    /// arestas-de-item `Uses` colapsaram nesta aresta módulo→módulo. `1` significa
    /// um único uso; valores altos, acoplamento forte. Contagem que
    /// [`agregar_por_modulo`] descartava — agora emitida por
    /// [`pesos_modulo_a_modulo`]. Densidade ≠ presença.
    pub peso: usize,
}

/// **Raio de impacto de um módulo** (prompt 0073): montante e jusante
/// **transitivos**, na convenção do [`crate::Raio`] por item:
///
/// - `montante`: módulos que **dependem deste** (quem sente — alcançabilidade
///   reversa sobre `Uses` de item, projetada a módulos).
/// - `jusante`: módulos **de que este depende** (alcançabilidade direta).
///
/// **Semântica exata** (definição 2 do prompt 0073): BFS no grafo de **itens**,
/// projetado a módulos — **não** o fecho do grafo agregado, que superestima
/// (`a∈A→b∈B`, `b'∈B→c∈C` sem caminho `a⇝c` daria `A⇝C` no agregado, mas a
/// exata não). O próprio módulo nunca aparece nas suas listas. Determinístico
/// (paths ordenados).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaioModulo {
    pub modulo: Path,
    pub montante: Vec<Path>,
    pub jusante: Vec<Path>,
}

/// Resultado do modo estrutura (prompt 0031, ampliado pelo 0035): a lista
/// de **módulos** do crate, as **dependências** módulo→módulo agregadas,
/// os **ciclos** detectados (SCCs ≥ 2), e o **ordenamento** da DSM
/// (`ordem` + `blocos` — prompt 0035). Todos os campos determinísticos.
///
/// `modulos` mantém a ordem **alfabética** (compatibilidade com clientes
/// pré-0035); `ordem` traz a **ordem topológica da condensação dos SCCs**
/// — é a sequência em que linhas/colunas da DSM aparecem. Ambas são
/// emitidas em paralelo.
///
/// Movido do `lente_wiring` (L4) no Estágio 2 (0056) — dado puro de estrutura.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EstruturaModulos {
    pub modulos: Vec<Path>,
    pub dependencias: Vec<DependenciaModulo>,
    pub ciclos: Vec<Ciclo>,
    /// Módulos na ordem da DSM (prompt 0035). Ordem topológica da
    /// condensação; SCCs ≥ 2 expandidos com membros agrupados.
    pub ordem: Vec<Path>,
    /// SCCs ≥ 2 na ordem em que aparecem em `ordem` (prompt 0035). Cada
    /// bloco é um intervalo contíguo de `ordem`.
    pub blocos: Vec<Vec<Path>>,
    /// Raio (montante/jusante transitivos) de **cada módulo** (prompt 0073),
    /// na mesma ordem de `modulos`. Para a interação "clicar um módulo e ver o
    /// que ele alcança" na vista HTML. Vazio se não computado.
    pub raios: Vec<RaioModulo>,
}

/// Ordenamento dos módulos para a DSM (prompt 0035): a sequência em que os
/// nós aparecem nas linhas/colunas da matriz, mais os **blocos** (SCCs ≥ 2)
/// na ordem em que aparecem em `ordem`.
///
/// Propriedade central: na grade que um consumidor monta a partir de
/// `ordem` + `dependencias`, **quase toda dependência aponta para o mesmo
/// lado da diagonal** (resultado da ordem topológica da condensação dos
/// SCCs); o que sobra do "lado errado" fica **dentro** dos blocos —
/// densas as células dos ciclos, claras as camadas entre eles. É a
/// "DSM como dado".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdemDsm {
    /// Módulos na ordem da DSM (linhas/colunas da grade). Determinística.
    pub ordem: Vec<Path>,
    /// SCCs ≥ 2 (cada um uma lista de módulos), na ordem em que aparecem
    /// dentro de `ordem`. Cada bloco corresponde a um intervalo contíguo
    /// de `ordem` — os membros de um SCC aparecem agrupados, e a ordem
    /// interna de cada SCC é por `path` ascendente (membros são
    /// mutuamente cíclicos; ordenação interna é convenção).
    pub blocos: Vec<Vec<Path>>,
}

/// Agrega o grafo de itens num grafo de **módulos**.
///
/// Política:
/// - **Nós** do resultado: apenas os com `Kind::Mod` ou `Kind::Crate`. `id`
///   e `path` preservados (idêntico ao do nó original).
/// - **Arestas `Uses`** do resultado: para cada aresta `Uses item_x →
///   item_y` no grafo original, achar `mod(x)` e `mod(y)` pela cadeia
///   `Owns`. Se `mod(x) != mod(y)`, emite aresta `Uses mod(x) → mod(y)` no
///   resultado (deduplicada). Uses intra-módulo (mod(x) == mod(y)) são
///   **absorvidos** — não viram aresta. Itens sem módulo contenedor (raros)
///   são ignorados.
/// - **Arestas `Owns`** do resultado: preservadas entre módulos (crate
///   "possui" módulos filhos diretos, módulo "possui" submódulos). Útil
///   para a hierarquia da DSM e para a navegação fractal futura.
///
/// Saída determinística: nós ordenados por `path`, arestas por `(from,
/// to, relation)`.
pub fn agregar_por_modulo(grafo: &Grafo) -> Grafo {
    let modulo_de = mapa_modulo_contenedor(grafo);

    // Nós: só módulos e crates, na ordem original (preserva `id` e `path`).
    let mut nodes: Vec<No> = grafo
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
        .cloned()
        .collect();
    nodes.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));

    // Arestas: deduplicadas em conjunto, depois ordenadas.
    let mut chaves: HashSet<(usize, usize, Relation)> = HashSet::new();
    let mut arestas: Vec<Aresta> = Vec::new();

    for a in &grafo.edges {
        match a.relation {
            Relation::Uses => {
                let Some(from_mod) = modulo_de.get(&a.id_from).copied() else {
                    continue;
                };
                let Some(to_mod) = modulo_de.get(&a.id_to).copied() else {
                    continue;
                };
                if from_mod == to_mod {
                    continue; // uses intra-módulo é absorvido
                }
                if chaves.insert((from_mod, to_mod, Relation::Uses)) {
                    let from_no = grafo.nodes.iter().find(|n| n.id == from_mod);
                    let to_no = grafo.nodes.iter().find(|n| n.id == to_mod);
                    if let (Some(f), Some(t)) = (from_no, to_no) {
                        // Agregado intencionalmente perde a granularidade do
                        // subtipo (uma aresta módulo→módulo deriva de N
                        // arestas-de-item, possivelmente de subtipos
                        // diferentes). `uses_kind = None` no agregado.
                        arestas.push(Aresta {
                            from: f.path.clone(),
                            id_from: from_mod,
                            to: t.path.clone(),
                            id_to: to_mod,
                            relation: Relation::Uses,
                            uses_kind: None,
                        });
                    }
                }
            }
            Relation::Owns => {
                // Hierarquia entre módulos: mantém só se ambas as pontas
                // forem módulos/crates.
                let from_e_mod = grafo
                    .nodes
                    .iter()
                    .find(|n| n.id == a.id_from)
                    .map(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
                    .unwrap_or(false);
                let to_e_mod = grafo
                    .nodes
                    .iter()
                    .find(|n| n.id == a.id_to)
                    .map(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
                    .unwrap_or(false);
                if from_e_mod && to_e_mod && chaves.insert((a.id_from, a.id_to, Relation::Owns)) {
                    arestas.push(a.clone());
                }
            }
        }
    }

    arestas.sort_by(|a, b| {
        a.from
            .as_str()
            .cmp(b.from.as_str())
            .then_with(|| a.to.as_str().cmp(b.to.as_str()))
            .then_with(|| format!("{:?}", a.relation).cmp(&format!("{:?}", b.relation)))
    });

    Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes,
        edges: arestas,
    }
}

/// **Peso de acoplamento** por aresta módulo→módulo (prompt 0071; Achado 1 do
/// laudo 0036): para cada par `(mod_from, mod_to)`, quantas arestas-de-item
/// `Uses` colapsam nela. É a contagem que [`agregar_por_modulo`] descarta no
/// `chaves.insert` (a 1ª aresta cria a aresta-de-módulo; as demais somem) —
/// aqui ela é preservada.
///
/// Mesma política do agregador: itens sem módulo contenedor e `uses`
/// intra-módulo (`mod_from == mod_to`) **não** contam. Chave por
/// `(id_mod_from, id_mod_to)` — casa com `id_from`/`id_to` das arestas do
/// agregado. Pura: só stdlib, determinística.
pub fn pesos_modulo_a_modulo(grafo: &Grafo) -> HashMap<(usize, usize), usize> {
    let modulo_de = mapa_modulo_contenedor(grafo);
    let mut pesos: HashMap<(usize, usize), usize> = HashMap::new();
    for a in &grafo.edges {
        if a.relation != Relation::Uses {
            continue;
        }
        let (from, to) = match (modulo_de.get(&a.id_from), modulo_de.get(&a.id_to)) {
            (Some(&f), Some(&t)) => (f, t),
            _ => continue,
        };
        if from == to {
            continue; // uses intra-módulo é absorvido (não vira aresta nem peso)
        }
        *pesos.entry((from, to)).or_insert(0) += 1;
    }
    pesos
}

/// **Raio por módulo** (prompt 0073) — montante/jusante transitivos de cada
/// módulo, na **definição exata** (alcançabilidade no grafo de itens projetada
/// a módulos; ver [`RaioModulo`]). Determinístico: módulos por `path`, listas
/// por `path`.
///
/// Custo: por módulo, dois BFS no grafo de itens — O(módulos · (itens +
/// arestas)). Linear; medido barato (laudo 0073).
pub fn raios_por_modulo(grafo: &Grafo) -> Vec<RaioModulo> {
    let modulo_de = mapa_modulo_contenedor(grafo);

    // Adjacência item→item sobre `Uses` (direta e reversa).
    let mut fwd: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut rev: HashMap<usize, Vec<usize>> = HashMap::new();
    for a in &grafo.edges {
        if a.relation == Relation::Uses {
            fwd.entry(a.id_from).or_default().push(a.id_to);
            rev.entry(a.id_to).or_default().push(a.id_from);
        }
    }

    // Itens (todos os nós) de cada módulo, pela cadeia `Owns` (mapa_modulo_contenedor).
    let mut itens_de: HashMap<usize, Vec<usize>> = HashMap::new();
    for n in &grafo.nodes {
        if let Some(&m) = modulo_de.get(&n.id) {
            itens_de.entry(m).or_default().push(n.id);
        }
    }
    let path_de_mod: HashMap<usize, Path> = grafo
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
        .map(|n| (n.id, n.path.clone()))
        .collect();

    let mut modulos: Vec<&No> = grafo
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, Kind::Mod | Kind::Crate))
        .collect();
    modulos.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));

    modulos
        .into_iter()
        .map(|m| {
            let fontes = itens_de.get(&m.id).cloned().unwrap_or_default();
            RaioModulo {
                modulo: m.path.clone(),
                montante: projetar(&alcanca(&fontes, &rev, &modulo_de, m.id), &path_de_mod),
                jusante: projetar(&alcanca(&fontes, &fwd, &modulo_de, m.id), &path_de_mod),
            }
        })
        .collect()
}

/// BFS multi-fonte sobre uma adjacência de itens; devolve o conjunto de
/// **módulos** dos itens alcançados, exceto o próprio (`self_mod`).
fn alcanca(
    fontes: &[usize],
    adj: &HashMap<usize, Vec<usize>>,
    modulo_de: &HashMap<usize, usize>,
    self_mod: usize,
) -> BTreeSet<usize> {
    let mut visto: HashSet<usize> = fontes.iter().copied().collect();
    let mut fila: std::collections::VecDeque<usize> = fontes.iter().copied().collect();
    let mut mods: BTreeSet<usize> = BTreeSet::new();
    while let Some(x) = fila.pop_front() {
        if let Some(vizinhos) = adj.get(&x) {
            for &y in vizinhos {
                if let Some(&m) = modulo_de.get(&y)
                    && m != self_mod
                {
                    mods.insert(m);
                }
                if visto.insert(y) {
                    fila.push_back(y);
                }
            }
        }
    }
    mods
}

/// Converte um conjunto de ids-de-módulo em paths ordenados.
fn projetar(mods: &BTreeSet<usize>, path_de_mod: &HashMap<usize, Path>) -> Vec<Path> {
    let mut v: Vec<Path> = mods
        .iter()
        .filter_map(|id| path_de_mod.get(id).cloned())
        .collect();
    v.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    v
}

/// Detecta os ciclos (SCCs de tamanho ≥ 2) no grafo, **sobre as arestas
/// `Uses`**. Genérico: funciona em qualquer `Grafo` (item, módulo, crate).
///
/// Algoritmo: Tarjan iterativo (sem recursão para evitar stack overflow em
/// grafos profundos). Saída determinística:
/// - Cada `Ciclo.modulos` é ordenado lexicograficamente.
/// - O `Vec<Ciclo>` é ordenado pelo primeiro path de cada ciclo.
pub fn detectar_ciclos(grafo: &Grafo) -> Vec<Ciclo> {
    let path_por_id: HashMap<usize, Path> =
        grafo.nodes.iter().map(|n| (n.id, n.path.clone())).collect();
    let sccs = tarjan_sccs(grafo, &path_por_id);

    // Filtra SCCs ≥ 2 e ordena deterministicamente.
    let mut ciclos: Vec<Ciclo> = sccs
        .into_iter()
        .filter(|s| s.len() >= 2)
        .map(|s| {
            let mut paths: Vec<Path> = s
                .into_iter()
                .filter_map(|id| path_por_id.get(&id).cloned())
                .collect();
            paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            Ciclo { modulos: paths }
        })
        .collect();
    ciclos.sort_by(|a, b| {
        let pa = a.modulos.first().map(|p| p.as_str()).unwrap_or("");
        let pb = b.modulos.first().map(|p| p.as_str()).unwrap_or("");
        pa.cmp(pb)
    });
    ciclos
}

/// Ordena os módulos para a DSM: condensação dos SCCs (cada SCC vira um
/// ponto) + ordem topológica do DAG resultante, com empate por menor
/// `path` do SCC. Os membros de cada SCC ficam **agrupados** em `ordem`,
/// e os SCCs ≥ 2 (os ciclos) são listados em `blocos` na ordem em que
/// aparecem.
///
/// Genérico sobre qualquer `Grafo` (item, módulo, crate) — prompt 0035 usa
/// no nível módulo, mas a peça reusa para o fractal.
///
/// Pureza L1: sem `petgraph`. Algoritmo:
/// 1. `tarjan_sccs` devolve a partição completa (cada nó num SCC, possivelmente
///    singleton).
/// 2. Constrói o DAG da condensação: aresta de SCC(a) → SCC(b) se existe
///    aresta `Uses` no grafo original com `from ∈ a, to ∈ b, a ≠ b`.
/// 3. Ordem topológica via Kahn iterativo (fila ordenada por menor `path`
///    do SCC para empate determinístico).
/// 4. Expande os SCCs na ordem topológica; membros internos ordenados por
///    `path` ascendente.
pub fn ordenar_dsm(grafo: &Grafo) -> OrdemDsm {
    let path_por_id: HashMap<usize, Path> =
        grafo.nodes.iter().map(|n| (n.id, n.path.clone())).collect();
    let sccs = tarjan_sccs(grafo, &path_por_id);

    // Membros de cada SCC ordenados por path; e descobre o "menor path"
    // de cada SCC para empate na topológica.
    let mut sccs_paths: Vec<Vec<Path>> = sccs
        .iter()
        .map(|s| {
            let mut paths: Vec<Path> = s
                .iter()
                .filter_map(|id| path_por_id.get(id).cloned())
                .collect();
            paths.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            paths
        })
        .collect();

    // Mapa id_de_no → índice de SCC.
    let mut scc_de_no: HashMap<usize, usize> = HashMap::new();
    for (i, s) in sccs.iter().enumerate() {
        for &id in s {
            scc_de_no.insert(id, i);
        }
    }

    // Construir o DAG da condensação: arestas SCC→SCC (deduplicadas).
    let mut adj_cond: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); sccs.len()];
    let mut grau_entrada: Vec<usize> = vec![0; sccs.len()];
    let mut arestas_visitadas: HashSet<(usize, usize)> = HashSet::new();
    for a in &grafo.edges {
        if a.relation != Relation::Uses {
            continue;
        }
        let Some(&ia) = scc_de_no.get(&a.id_from) else {
            continue;
        };
        let Some(&ib) = scc_de_no.get(&a.id_to) else {
            continue;
        };
        if ia == ib {
            continue; // aresta interna ao SCC — não vira aresta da condensação
        }
        if arestas_visitadas.insert((ia, ib)) {
            adj_cond[ia].insert(ib);
            grau_entrada[ib] += 1;
        }
    }

    // Kahn determinístico: a fila é uma BTreeSet de (menor_path_do_scc,
    // índice_scc) para ordenar empates por path ascendente.
    let chave = |i: usize| -> String {
        sccs_paths[i]
            .first()
            .map(|p| p.as_str().to_string())
            .unwrap_or_default()
    };
    let mut prontos: BTreeSet<(String, usize)> = BTreeSet::new();
    for (i, grau) in grau_entrada.iter().enumerate() {
        if *grau == 0 {
            prontos.insert((chave(i), i));
        }
    }

    let mut ordem_sccs: Vec<usize> = Vec::with_capacity(sccs.len());
    while let Some(entrada) = prontos.iter().next().cloned() {
        prontos.remove(&entrada);
        let (_, i) = entrada;
        ordem_sccs.push(i);
        // Para cada vizinho na condensação, decrementa o grau de entrada;
        // adiciona à fila quando chegar a zero.
        let vizinhos: Vec<usize> = adj_cond[i].iter().copied().collect();
        for j in vizinhos {
            grau_entrada[j] -= 1;
            if grau_entrada[j] == 0 {
                prontos.insert((chave(j), j));
            }
        }
    }

    // Expandir: para cada SCC na ordem topológica, emitir seus membros.
    let mut ordem: Vec<Path> = Vec::with_capacity(grafo.nodes.len());
    let mut blocos: Vec<Vec<Path>> = Vec::new();
    for &i in &ordem_sccs {
        let membros = std::mem::take(&mut sccs_paths[i]);
        if membros.len() >= 2 {
            blocos.push(membros.clone());
        }
        ordem.extend(membros);
    }

    OrdemDsm { ordem, blocos }
}

/// Tarjan iterativo: devolve **todos** os SCCs do grafo sobre as arestas
/// `Uses`, **incluindo singletons** (nós sem ciclo formam SCC de tamanho 1).
/// É a **partição completa** dos nós — pré-requisito da condensação
/// (prompt 0035): cada SCC vira um ponto, e a condensação fica DAG.
///
/// Os SCCs vêm na ordem em que o Tarjan os **fecha** — naturalmente a
/// inversa de uma ordem topológica da condensação. O chamador decide
/// como aplicar essa propriedade (filtrar como [`detectar_ciclos`] faz,
/// ou usar para ordenar como [`ordenar_dsm`] faz).
///
/// Determinismo: a ordem de visita das raízes do DFS é por `path`
/// ascendente, e a adjacência é ordenada por id alcançado. Mesmo
/// `Grafo` → mesma sequência de SCCs.
fn tarjan_sccs(grafo: &Grafo, path_por_id: &HashMap<usize, Path>) -> Vec<Vec<usize>> {
    // Adjacência: id → ids alcançáveis via Uses.
    let mut adj: HashMap<usize, Vec<usize>> = HashMap::new();
    for n in &grafo.nodes {
        adj.entry(n.id).or_default();
    }
    for a in &grafo.edges {
        if a.relation == Relation::Uses {
            adj.entry(a.id_from).or_default().push(a.id_to);
        }
    }
    // Determinismo: ordenar adjacência por id alcançado.
    for v in adj.values_mut() {
        v.sort_unstable();
        v.dedup();
    }

    // Ordem dos nós-raiz da DFS: por path ascendente — saída determinística.
    let mut ordem: Vec<usize> = grafo.nodes.iter().map(|n| n.id).collect();
    ordem.sort_by(|a, b| {
        path_por_id
            .get(a)
            .map(|p| p.as_str())
            .unwrap_or("")
            .cmp(path_por_id.get(b).map(|p| p.as_str()).unwrap_or(""))
    });

    let mut index: HashMap<usize, usize> = HashMap::new();
    let mut lowlink: HashMap<usize, usize> = HashMap::new();
    let mut on_stack: HashSet<usize> = HashSet::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut idx: usize = 0;
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for &raiz in &ordem {
        if index.contains_key(&raiz) {
            continue;
        }
        let mut dfs: Vec<(usize, usize)> = Vec::new();
        index.insert(raiz, idx);
        lowlink.insert(raiz, idx);
        idx += 1;
        stack.push(raiz);
        on_stack.insert(raiz);
        dfs.push((raiz, 0));

        while let Some(&(v, i)) = dfs.last() {
            let viz = adj.get(&v).map(|v| v.as_slice()).unwrap_or(&[]);
            if i < viz.len() {
                let w = viz[i];
                let last = dfs.last_mut().unwrap();
                last.1 = i + 1;

                if let std::collections::hash_map::Entry::Vacant(e) = index.entry(w) {
                    e.insert(idx);
                    lowlink.insert(w, idx);
                    idx += 1;
                    stack.push(w);
                    on_stack.insert(w);
                    dfs.push((w, 0));
                } else if on_stack.contains(&w) {
                    let lv = *lowlink.get(&v).unwrap();
                    let iw = *index.get(&w).unwrap();
                    lowlink.insert(v, lv.min(iw));
                }
            } else {
                let v_idx = *index.get(&v).unwrap();
                let v_low = *lowlink.get(&v).unwrap();
                dfs.pop();
                if v_low == v_idx {
                    let mut scc = Vec::new();
                    while let Some(w) = stack.pop() {
                        on_stack.remove(&w);
                        scc.push(w);
                        if w == v {
                            break;
                        }
                    }
                    sccs.push(scc);
                }
                if let Some(&(parent, _)) = dfs.last() {
                    let lp = *lowlink.get(&parent).unwrap();
                    lowlink.insert(parent, lp.min(v_low));
                }
            }
        }
    }
    sccs
}

/// Para cada nó do grafo, qual é o seu **módulo contenedor** (id). Subida
/// pela cadeia de `Owns` até o primeiro `Kind::Mod` ou `Kind::Crate`. Um
/// nó que **é** módulo/crate aponta para si mesmo. Itens sem cadeia até
/// um módulo (órfãos, raros) ficam **ausentes** do mapa.
fn mapa_modulo_contenedor(grafo: &Grafo) -> HashMap<usize, usize> {
    let pai: HashMap<usize, usize> = grafo
        .edges
        .iter()
        .filter(|a| a.relation == Relation::Owns)
        .map(|a| (a.id_to, a.id_from))
        .collect();
    let kind_por_id: HashMap<usize, Kind> = grafo.nodes.iter().map(|n| (n.id, n.kind)).collect();

    let mut contenedor: HashMap<usize, usize> = HashMap::new();
    for n in &grafo.nodes {
        if matches!(n.kind, Kind::Mod | Kind::Crate) {
            contenedor.insert(n.id, n.id);
            continue;
        }
        // Subir até encontrar Mod/Crate. Protege contra ciclos teóricos em
        // `Owns` com `BTreeSet` de visitados (não ocorrem em dado real,
        // mas o custo é nulo).
        let mut visitados: BTreeSet<usize> = BTreeSet::new();
        let mut atual = n.id;
        loop {
            if !visitados.insert(atual) {
                break; // ciclo em Owns — abandona
            }
            match pai.get(&atual) {
                None => break, // órfão
                Some(&p) => {
                    if matches!(kind_por_id.get(&p), Some(Kind::Mod) | Some(Kind::Crate)) {
                        contenedor.insert(n.id, p);
                        break;
                    }
                    atual = p;
                }
            }
        }
    }
    contenedor
}

/// Obstrução na cadeia de contenção. IDs só identificam este snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemaContencao {
    SemModulo,
    NoAusente { id: usize },
    MultiplosPais { item: usize, pais: Vec<usize> },
    Ciclo { membros: Vec<usize> },
}

/// Um nó cuja contenção não pôde ser estabelecida com segurança.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemSemContenedor {
    pub id: usize,
    pub path: Path,
    pub problemas: Vec<ProblemaContencao>,
}

/// Motivo explícito para uma relação não contribuir para uma célula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MotivoNaoAgregavel {
    /// Owns é hierarquia, não dependência Uses. Não contribui para o peso.
    ContencaoOwns,
    /// Problemas de cada endpoint, separadamente; vazio significa ponta válida.
    ContencaoInvalida {
        origem: Vec<ProblemaContencao>,
        destino: Vec<ProblemaContencao>,
    },
}

/// Partição exaustiva dos registros de arestas do snapshot recebido.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DestinoRelacao {
    Agregada {
        id_modulo_de: usize,
        id_modulo_para: usize,
    },
    InternaAbsorvida {
        id_modulo: usize,
    },
    NaoAgregavel(MotivoNaoAgregavel),
}

/// Evidência fiel. `indice_snapshot` endereça `grafo.edges[indice_snapshot]`,
/// inclusive quando há registros idênticos. Não é identidade entre extrações.
/// A posição de um nó não é convertida em posição da referência.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelacaoDetalhada {
    pub indice_snapshot: usize,
    pub aresta: Aresta,
    pub destino: DestinoRelacao,
}

/// Célula explicada da DSM. Peso = quantidade de registros Uses no snapshot,
/// incluindo paralelas e todos os subtipos; não são ocorrências no código nem
/// pares distintos. `evidencias` contém índices em `grafo.edges`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenciaDetalhada {
    pub id_de: usize,
    pub de: Path,
    pub id_para: usize,
    pub para: Path,
    pub peso: usize,
    pub evidencias: Vec<usize>,
}

/// Agregação com contabilidade completa: cada aresta aparece uma vez em
/// `relacoes`; cada contribuição entre módulos pertence a uma única célula.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgregacaoDetalhada {
    pub dependencias: Vec<DependenciaDetalhada>,
    pub relacoes: Vec<RelacaoDetalhada>,
    pub itens_sem_contenedor: Vec<ItemSemContenedor>,
}

/// Agrega com evidências sem modificar a política das APIs legadas.
///
/// Exige IDs únicos, conforme o contrato de Grafo. Usa IDs para contenção e
/// preserva paths verbatim. Valida toda a ancestralidade Owns, inclusive acima
/// de módulos: ciclo ou múltiplos pais tornam o nó e seus descendentes não
/// agregáveis, mesmo se os pais levarem ao mesmo módulo. Owns paralelos para
/// o mesmo pai não criam ambiguidade. Nenhum pai é escolhido arbitrariamente.
///
/// Detecta SCCs Owns (inclusive auto-ciclos); registra as obstruções locais da
/// primeira ancestralidade inválida alcançada. Nós fora dessas cadeias seguem
/// agregáveis. Ausência de módulo é reportada também para nós sem Uses.
/// Ordena por paths/IDs e discriminantes explícitos, nunca por Debug de enums.
/// Permutar a entrada preserva a semântica, mas muda os índices das evidências.
///
/// Consulta das evidências de uma célula, inteiramente em memória:
/// ```
/// use lente_core::entities::grafo::Grafo;
/// use lente_estrutura::agregar_por_modulo_detalhado;
/// fn evidencias(grafo: &Grafo, de: usize, para: usize) {
///     let detalhe = agregar_por_modulo_detalhado(grafo);
///     if let Some(celula) = detalhe.dependencias.iter()
///         .find(|d| d.id_de == de && d.id_para == para) {
///         assert_eq!(celula.peso, celula.evidencias.len());
///         for &indice in &celula.evidencias {
///             let original = &grafo.edges[indice];
///             assert_eq!(original.relation,
///                 lente_core::entities::grafo::Relation::Uses);
///             // original preserva IDs, paths, relation e uses_kind.
///         }
///     }
/// }
/// evidencias(&Grafo::new("exemplo"), 1, 2);
/// ```
pub fn agregar_por_modulo_detalhado(grafo: &Grafo) -> AgregacaoDetalhada {
    let nos: HashMap<usize, &No> = grafo.nodes.iter().map(|n| (n.id, n)).collect();
    let mut pais: HashMap<usize, BTreeSet<usize>> = HashMap::new();
    for a in &grafo.edges {
        if a.relation == Relation::Owns {
            pais.entry(a.id_to).or_default().insert(a.id_from);
        }
    }
    // Projeção local exclusivamente para reutilizar o Tarjan iterativo. As
    // evidências originais não são alteradas e continuam com Relation::Owns.
    let hierarquia = Grafo {
        crate_name: grafo.crate_name.clone(),
        nodes: grafo.nodes.clone(),
        edges: grafo
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Owns)
            .map(|a| Aresta {
                relation: Relation::Uses,
                ..a.clone()
            })
            .collect(),
    };
    let paths = nos.iter().map(|(&id, n)| (id, n.path.clone())).collect();
    let mut problemas: HashMap<usize, Vec<ProblemaContencao>> = HashMap::new();
    for (&id, ps) in &pais {
        if ps.len() > 1 {
            problemas
                .entry(id)
                .or_default()
                .push(ProblemaContencao::MultiplosPais {
                    item: id,
                    pais: ps.iter().copied().collect(),
                });
        }
    }
    for mut membros in tarjan_sccs(&hierarquia, &paths) {
        if membros.len() > 1
            || membros
                .first()
                .is_some_and(|id| pais.get(id).is_some_and(|ps| ps.contains(id)))
        {
            membros.sort_unstable();
            for &id in &membros {
                problemas
                    .entry(id)
                    .or_default()
                    .push(ProblemaContencao::Ciclo {
                        membros: membros.clone(),
                    });
            }
        }
    }
    let mut resolvidos: HashMap<usize, Result<Option<usize>, Vec<ProblemaContencao>>> =
        HashMap::new();
    let ids: BTreeSet<usize> = nos
        .keys()
        .copied()
        .chain(grafo.edges.iter().flat_map(|a| [a.id_from, a.id_to]))
        .collect();
    for id in ids {
        let mut atual = id;
        let mut cadeia = Vec::new();
        let mut resultado = loop {
            if let Some(r) = resolvidos.get(&atual) {
                break r.clone();
            }
            if !nos.contains_key(&atual) {
                break Err(vec![ProblemaContencao::NoAusente { id: atual }]);
            }
            if let Some(p) = problemas.get(&atual) {
                break Err(p.clone());
            }
            cadeia.push(atual);
            match pais.get(&atual).and_then(|ps| ps.first()) {
                Some(&pai) => atual = pai,
                None => break Ok(None),
            }
        };
        resolvidos.entry(atual).or_insert_with(|| resultado.clone());
        for item in cadeia.into_iter().rev() {
            if resultado.is_ok() && matches!(nos[&item].kind, Kind::Mod | Kind::Crate) {
                resultado = Ok(Some(item));
            }
            resolvidos.insert(item, resultado.clone());
        }
    }
    let erros = |id: usize| match &resolvidos[&id] {
        Ok(Some(_)) => vec![],
        Ok(None) => vec![ProblemaContencao::SemModulo],
        Err(p) => p.clone(),
    };
    let mut itens_sem_contenedor: Vec<_> = grafo
        .nodes
        .iter()
        .filter_map(|n| {
            let problemas = erros(n.id);
            (!problemas.is_empty()).then(|| ItemSemContenedor {
                id: n.id,
                path: n.path.clone(),
                problemas,
            })
        })
        .collect();
    itens_sem_contenedor.sort_by(|a, b| (&a.path, a.id).cmp(&(&b.path, b.id)));
    let mut relacoes: Vec<_> = grafo
        .edges
        .iter()
        .enumerate()
        .map(|(indice_snapshot, a)| {
            let origem = erros(a.id_from);
            let destino = erros(a.id_to);
            let destino = if !origem.is_empty() || !destino.is_empty() {
                DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoInvalida {
                    origem,
                    destino,
                })
            } else if a.relation == Relation::Owns {
                DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoOwns)
            } else {
                let de = resolvidos[&a.id_from].as_ref().unwrap().unwrap();
                let para = resolvidos[&a.id_to].as_ref().unwrap().unwrap();
                if de == para {
                    DestinoRelacao::InternaAbsorvida { id_modulo: de }
                } else {
                    DestinoRelacao::Agregada {
                        id_modulo_de: de,
                        id_modulo_para: para,
                    }
                }
            };
            RelacaoDetalhada {
                indice_snapshot,
                aresta: a.clone(),
                destino,
            }
        })
        .collect();
    fn chave(a: &Aresta) -> (&Path, &Path, usize, usize, u8, u8) {
        use lente_core::entities::grafo::UsesKind;
        (
            &a.from,
            &a.to,
            a.id_from,
            a.id_to,
            match a.relation {
                Relation::Owns => 0,
                Relation::Uses => 1,
            },
            match a.uses_kind {
                Option::None => 0,
                Some(UsesKind::Reference) => 1,
                Some(UsesKind::Import) => 2,
            },
        )
    }
    relacoes.sort_by(|a, b| {
        chave(&a.aresta)
            .cmp(&chave(&b.aresta))
            .then(a.indice_snapshot.cmp(&b.indice_snapshot))
    });
    let mut celulas: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for r in &relacoes {
        if let DestinoRelacao::Agregada {
            id_modulo_de,
            id_modulo_para,
        } = r.destino
        {
            celulas
                .entry((id_modulo_de, id_modulo_para))
                .or_default()
                .push(r.indice_snapshot);
        }
    }
    let mut dependencias: Vec<_> = celulas
        .into_iter()
        .map(|((id_de, id_para), evidencias)| DependenciaDetalhada {
            id_de,
            de: nos[&id_de].path.clone(),
            id_para,
            para: nos[&id_para].path.clone(),
            peso: evidencias.len(),
            evidencias,
        })
        .collect();
    dependencias.sort_by(|a, b| {
        (&a.de, &a.para, a.id_de, a.id_para).cmp(&(&b.de, &b.para, b.id_de, b.id_para))
    });
    AgregacaoDetalhada {
        dependencias,
        relacoes,
        itens_sem_contenedor,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::entities::grafo::{Modificadores, Visibility};

    fn no(id: usize, path: &str, kind: Kind) -> No {
        No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: "k".to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: None,
        }
    }

    fn aresta(id_from: usize, from: &str, id_to: usize, to: &str, rel: Relation) -> Aresta {
        Aresta {
            from: Path::from(from),
            id_from,
            to: Path::from(to),
            id_to,
            relation: rel,
            uses_kind: None,
        }
    }

    /// Grafo-padrão com 2 módulos (a, b) e 1 item dentro de cada (a::f, b::g).
    /// Sem arestas de Uses; útil de ponto de partida.
    fn grafo_dois_modulos() -> Grafo {
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(1, "k", Kind::Crate),
            no(10, "k::a", Kind::Mod),
            no(11, "k::a::f", Kind::Fn),
            no(20, "k::b", Kind::Mod),
            no(21, "k::b::g", Kind::Fn),
        ];
        g.edges = vec![
            aresta(1, "k", 10, "k::a", Relation::Owns),
            aresta(10, "k::a", 11, "k::a::f", Relation::Owns),
            aresta(1, "k", 20, "k::b", Relation::Owns),
            aresta(20, "k::b", 21, "k::b::g", Relation::Owns),
        ];
        g
    }

    // ---- mapa_modulo_contenedor ---------------------------------------------

    #[test]
    fn modulo_contenedor_de_item_e_seu_modulo_direto() {
        let g = grafo_dois_modulos();
        let m = mapa_modulo_contenedor(&g);
        assert_eq!(m.get(&11), Some(&10), "f deve estar em a");
        assert_eq!(m.get(&21), Some(&20), "g deve estar em b");
    }

    #[test]
    fn modulo_contenedor_de_modulo_e_ele_mesmo() {
        let g = grafo_dois_modulos();
        let m = mapa_modulo_contenedor(&g);
        assert_eq!(m.get(&10), Some(&10));
        assert_eq!(m.get(&1), Some(&1)); // crate aponta para si
    }

    #[test]
    fn modulo_contenedor_sobe_atraves_de_struct() {
        // método sob struct sob módulo: a subida tem que passar pela struct
        // até alcançar o módulo. `T` é struct (não módulo), `f` é fn.
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(10, "k::a", Kind::Mod),
            no(11, "k::a::T", Kind::Struct),
            no(12, "k::a::T::f", Kind::Fn),
        ];
        g.edges = vec![
            aresta(10, "k::a", 11, "k::a::T", Relation::Owns),
            aresta(11, "k::a::T", 12, "k::a::T::f", Relation::Owns),
        ];
        let m = mapa_modulo_contenedor(&g);
        // método deve subir struct → módulo.
        assert_eq!(m.get(&12), Some(&10));
    }

    // ---- pesos_modulo_a_modulo (prompt 0071, Achado 1 do 0036) --------------

    #[test]
    fn peso_conta_arestas_de_item_por_par_de_modulo() {
        // a tem f(11) e g(13); b tem h(21). Três Uses cross-módulo e um
        // intra-módulo (absorvido): a→b = 2, b→a = 1.
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(10, "k::a", Kind::Mod),
            no(11, "k::a::f", Kind::Fn),
            no(13, "k::a::g", Kind::Fn),
            no(20, "k::b", Kind::Mod),
            no(21, "k::b::h", Kind::Fn),
        ];
        g.edges = vec![
            aresta(10, "k::a", 11, "k::a::f", Relation::Owns),
            aresta(10, "k::a", 13, "k::a::g", Relation::Owns),
            aresta(20, "k::b", 21, "k::b::h", Relation::Owns),
            // dois usos a→b (de itens diferentes de a para o mesmo item de b)
            aresta(11, "k::a::f", 21, "k::b::h", Relation::Uses),
            aresta(13, "k::a::g", 21, "k::b::h", Relation::Uses),
            // um uso b→a
            aresta(21, "k::b::h", 11, "k::a::f", Relation::Uses),
            // uso intra-módulo a→a: NÃO conta
            aresta(11, "k::a::f", 13, "k::a::g", Relation::Uses),
        ];
        let pesos = pesos_modulo_a_modulo(&g);
        assert_eq!(pesos.get(&(10, 20)), Some(&2), "a→b deve ter peso 2");
        assert_eq!(pesos.get(&(20, 10)), Some(&1), "b→a deve ter peso 1");
        assert_eq!(pesos.get(&(10, 10)), None, "intra-módulo não entra");
        assert_eq!(pesos.len(), 2);
    }

    // ---- raios_por_modulo (prompt 0073, semântica EXATA — teste-contrato) ----

    /// O contrato da definição 2: `a∈A → b∈B`, `b'∈B → c∈C`, **sem** caminho de
    /// item `a ⇝ c`. O fecho do grafo AGREGADO diria `A ⇝ C` (A→B→C); a exata
    /// **não** — não há item de A que alcance um item de C. Se alguém trocar a
    /// definição pela agregada, este teste grita.
    #[test]
    fn raio_exato_nao_superestima_pela_agregacao() {
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(100, "k::a", Kind::Mod),
            no(1, "k::a::f", Kind::Fn),
            no(200, "k::b", Kind::Mod),
            no(2, "k::b::g", Kind::Fn),
            no(3, "k::b::h", Kind::Fn),
            no(300, "k::c", Kind::Mod),
            no(4, "k::c::i", Kind::Fn),
        ];
        g.edges = vec![
            aresta(100, "k::a", 1, "k::a::f", Relation::Owns),
            aresta(200, "k::b", 2, "k::b::g", Relation::Owns),
            aresta(200, "k::b", 3, "k::b::h", Relation::Owns),
            aresta(300, "k::c", 4, "k::c::i", Relation::Owns),
            aresta(1, "k::a::f", 2, "k::b::g", Relation::Uses), // a → b
            aresta(3, "k::b::h", 4, "k::c::i", Relation::Uses), // b' → c
        ];
        let raios = raios_por_modulo(&g);
        let de = |p: &str| raios.iter().find(|r| r.modulo.as_str() == p).unwrap();

        let a = de("k::a");
        // jusante (do que A depende, forward): só B — C fica de fora (exata).
        assert_eq!(
            a.jusante,
            vec![Path::from("k::b")],
            "A depende só de B, não de C"
        );
        assert!(a.montante.is_empty(), "ninguém depende de A");

        let b = de("k::b");
        assert_eq!(b.jusante, vec![Path::from("k::c")], "B depende de C (b'→c)");
        assert_eq!(b.montante, vec![Path::from("k::a")], "A depende de B");

        let c = de("k::c");
        assert!(c.jusante.is_empty());
        assert_eq!(c.montante, vec![Path::from("k::b")], "B depende de C");
    }

    #[test]
    fn no_orfao_nao_tem_modulo_contenedor() {
        let mut g = Grafo::new("k");
        g.nodes = vec![no(99, "k::orfao", Kind::Fn)];
        g.edges = vec![]; // sem Owns
        let m = mapa_modulo_contenedor(&g);
        assert!(!m.contains_key(&99));
    }

    // ---- agregar_por_modulo --------------------------------------------------

    #[test]
    fn agregacao_uses_inter_modulo_vira_aresta_uma_vez() {
        let mut g = grafo_dois_modulos();
        // dois itens de a dependem do mesmo item de b — vira UMA aresta a→b.
        g.nodes.push(no(12, "k::a::f2", Kind::Fn));
        g.edges
            .push(aresta(10, "k::a", 12, "k::a::f2", Relation::Owns));
        g.edges
            .push(aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses));
        g.edges
            .push(aresta(12, "k::a::f2", 21, "k::b::g", Relation::Uses));
        let r = agregar_por_modulo(&g);
        let arestas_uses: Vec<_> = r
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Uses)
            .collect();
        assert_eq!(arestas_uses.len(), 1, "dois itens, uma aresta de módulo");
        assert_eq!(arestas_uses[0].from.as_str(), "k::a");
        assert_eq!(arestas_uses[0].to.as_str(), "k::b");
    }

    #[test]
    fn agregacao_ignora_uses_sem_modulo_na_origem_ou_no_destino() {
        let mut g = grafo_dois_modulos();
        g.nodes.push(no(99, "k::orfao", Kind::Fn));
        g.edges.extend([
            aresta(99, "k::orfao", 21, "k::b::g", Relation::Uses),
            aresta(11, "k::a::f", 99, "k::orfao", Relation::Uses),
            aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses),
        ]);
        let usos: Vec<_> = agregar_por_modulo(&g)
            .edges
            .into_iter()
            .filter(|a| a.relation == Relation::Uses)
            .map(|a| (a.id_from, a.id_to))
            .collect();
        assert_eq!(usos, [(10, 20)]);
    }

    #[test]
    fn agregacao_uses_intra_modulo_e_absorvido() {
        let mut g = grafo_dois_modulos();
        // dois itens do MESMO módulo se usam — nenhuma aresta no resultado.
        g.nodes.push(no(12, "k::a::f2", Kind::Fn));
        g.edges
            .push(aresta(10, "k::a", 12, "k::a::f2", Relation::Owns));
        g.edges
            .push(aresta(11, "k::a::f", 12, "k::a::f2", Relation::Uses));
        let r = agregar_por_modulo(&g);
        assert!(
            r.edges.iter().all(|a| a.relation != Relation::Uses),
            "uses intra-módulo não vira aresta"
        );
    }

    #[test]
    fn agregacao_so_inclui_nos_de_mod_e_crate() {
        let g = grafo_dois_modulos();
        let r = agregar_por_modulo(&g);
        for n in &r.nodes {
            assert!(matches!(n.kind, Kind::Mod | Kind::Crate));
        }
        // 1 crate + 2 mods = 3 nós no resultado.
        assert_eq!(r.nodes.len(), 3);
    }

    #[test]
    fn agregacao_preserva_id_e_path_dos_modulos() {
        let g = grafo_dois_modulos();
        let r = agregar_por_modulo(&g);
        let mod_a = r.nodes.iter().find(|n| n.path.as_str() == "k::a").unwrap();
        assert_eq!(mod_a.id, 10);
    }

    #[test]
    fn agregacao_preserva_owns_entre_modulos() {
        let g = grafo_dois_modulos();
        let r = agregar_por_modulo(&g);
        let owns: Vec<_> = r
            .edges
            .iter()
            .filter(|a| a.relation == Relation::Owns)
            .collect();
        // crate possui dois módulos.
        assert_eq!(owns.len(), 2);
    }

    #[test]
    fn agregacao_preserva_crate_name() {
        let g = grafo_dois_modulos();
        let r = agregar_por_modulo(&g);
        assert_eq!(r.crate_name, "k");
    }

    // ---- detectar_ciclos -----------------------------------------------------

    #[test]
    fn ciclo_de_dois_modulos_e_detectado() {
        let mut g = grafo_dois_modulos();
        // f (em a) usa g (em b); g (em b) usa f (em a) — agregação produz
        // a→b e b→a, formando um ciclo de tamanho 2.
        g.edges
            .push(aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses));
        g.edges
            .push(aresta(21, "k::b::g", 11, "k::a::f", Relation::Uses));
        let agg = agregar_por_modulo(&g);
        let ciclos = detectar_ciclos(&agg);
        assert_eq!(ciclos.len(), 1);
        let nomes: Vec<&str> = ciclos[0].modulos.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b"]);
    }

    #[test]
    fn ciclo_de_tres_modulos_e_detectado() {
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(1, "k", Kind::Crate),
            no(10, "k::a", Kind::Mod),
            no(11, "k::a::x", Kind::Fn),
            no(20, "k::b", Kind::Mod),
            no(21, "k::b::y", Kind::Fn),
            no(30, "k::c", Kind::Mod),
            no(31, "k::c::z", Kind::Fn),
        ];
        g.edges = vec![
            aresta(1, "k", 10, "k::a", Relation::Owns),
            aresta(10, "k::a", 11, "k::a::x", Relation::Owns),
            aresta(1, "k", 20, "k::b", Relation::Owns),
            aresta(20, "k::b", 21, "k::b::y", Relation::Owns),
            aresta(1, "k", 30, "k::c", Relation::Owns),
            aresta(30, "k::c", 31, "k::c::z", Relation::Owns),
            // A → B → C → A
            aresta(11, "k::a::x", 21, "k::b::y", Relation::Uses),
            aresta(21, "k::b::y", 31, "k::c::z", Relation::Uses),
            aresta(31, "k::c::z", 11, "k::a::x", Relation::Uses),
        ];
        let agg = agregar_por_modulo(&g);
        let ciclos = detectar_ciclos(&agg);
        assert_eq!(ciclos.len(), 1);
        let nomes: Vec<&str> = ciclos[0].modulos.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b", "k::c"]);
    }

    #[test]
    fn grafo_aciclico_nao_tem_ciclos() {
        let mut g = grafo_dois_modulos();
        // a → b, sem volta — DAG.
        g.edges
            .push(aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses));
        let agg = agregar_por_modulo(&g);
        let ciclos = detectar_ciclos(&agg);
        assert!(ciclos.is_empty());
    }

    #[test]
    fn ciclo_de_um_nodo_uses_de_si_mesmo_nao_conta_como_ciclo() {
        // SCC só conta como ciclo se tiver ≥ 2 elementos. Um auto-loop
        // (módulo que usa a si mesmo) não dispara — combina com a
        // política "ciclos de tamanho ≥ 2" do prompt 0031.
        let mut g = grafo_dois_modulos();
        // Tecnicamente impossível depois de agregar (uses intra-módulo é
        // absorvido), mas para garantir, exercito direto: detectar_ciclos
        // sobre um grafo com auto-loop em a→a.
        let mut g_loop = Grafo::new("k");
        g_loop.nodes = vec![no(10, "k::a", Kind::Mod)];
        g_loop.edges = vec![aresta(10, "k::a", 10, "k::a", Relation::Uses)];
        let ciclos = detectar_ciclos(&g_loop);
        assert!(ciclos.is_empty());

        // sanity: o caso normal de duas direções continua detectando.
        g.edges
            .push(aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses));
        g.edges
            .push(aresta(21, "k::b::g", 11, "k::a::f", Relation::Uses));
        let agg = agregar_por_modulo(&g);
        let ciclos = detectar_ciclos(&agg);
        assert_eq!(ciclos.len(), 1);
    }

    #[test]
    fn dois_ciclos_disjuntos_aparecem_em_ordem_deterministica() {
        // Dois ciclos: {a, b} e {c, d}; ordenados pelo primeiro path.
        let mut g = Grafo::new("k");
        g.nodes = vec![
            no(1, "k", Kind::Crate),
            no(10, "k::a", Kind::Mod),
            no(11, "k::a::x", Kind::Fn),
            no(20, "k::b", Kind::Mod),
            no(21, "k::b::y", Kind::Fn),
            no(30, "k::c", Kind::Mod),
            no(31, "k::c::z", Kind::Fn),
            no(40, "k::d", Kind::Mod),
            no(41, "k::d::w", Kind::Fn),
        ];
        g.edges = vec![
            aresta(1, "k", 10, "k::a", Relation::Owns),
            aresta(10, "k::a", 11, "k::a::x", Relation::Owns),
            aresta(1, "k", 20, "k::b", Relation::Owns),
            aresta(20, "k::b", 21, "k::b::y", Relation::Owns),
            aresta(1, "k", 30, "k::c", Relation::Owns),
            aresta(30, "k::c", 31, "k::c::z", Relation::Owns),
            aresta(1, "k", 40, "k::d", Relation::Owns),
            aresta(40, "k::d", 41, "k::d::w", Relation::Owns),
            // ciclo 1: a ↔ b
            aresta(11, "k::a::x", 21, "k::b::y", Relation::Uses),
            aresta(21, "k::b::y", 11, "k::a::x", Relation::Uses),
            // ciclo 2: c ↔ d
            aresta(31, "k::c::z", 41, "k::d::w", Relation::Uses),
            aresta(41, "k::d::w", 31, "k::c::z", Relation::Uses),
        ];
        let agg = agregar_por_modulo(&g);
        let ciclos = detectar_ciclos(&agg);
        assert_eq!(ciclos.len(), 2);
        assert_eq!(ciclos[0].modulos[0].as_str(), "k::a");
        assert_eq!(ciclos[1].modulos[0].as_str(), "k::c");
    }

    /// Genericidade: `detectar_ciclos` opera sobre qualquer Grafo,
    /// inclusive um de itens (sem agregação prévia). A "estrutura
    /// fractal" do prompt 0031 — mesma peça noutra escala — vive aqui.
    #[test]
    fn detectar_ciclos_funciona_sobre_grafo_de_itens_tambem() {
        let mut g = Grafo::new("k");
        g.nodes = vec![no(11, "k::a::x", Kind::Fn), no(21, "k::b::y", Kind::Fn)];
        g.edges = vec![
            aresta(11, "k::a::x", 21, "k::b::y", Relation::Uses),
            aresta(21, "k::b::y", 11, "k::a::x", Relation::Uses),
        ];
        let ciclos = detectar_ciclos(&g);
        assert_eq!(ciclos.len(), 1);
        assert_eq!(ciclos[0].modulos.len(), 2);
    }

    // ---- ordenar_dsm (prompt 0035) ------------------------------------------

    /// Helper que monta um grafo módulo→módulo já no formato que o
    /// `ordenar_dsm` espera. Não é o agregador (que sobe `Owns`); é direto.
    fn grafo_modulos(ms: &[(usize, &str)], deps: &[(usize, usize)]) -> Grafo {
        let mut g = Grafo::new("k");
        g.nodes = ms.iter().map(|&(id, p)| no(id, p, Kind::Mod)).collect();
        let path_de = |id: usize| -> &str {
            ms.iter()
                .find(|&&(i, _)| i == id)
                .map(|&(_, p)| p)
                .unwrap_or("")
        };
        g.edges = deps
            .iter()
            .map(|&(a, b)| aresta(a, path_de(a), b, path_de(b), Relation::Uses))
            .collect();
        g
    }

    #[test]
    fn ordenar_dag_linear_devolve_ordem_topologica_estavel() {
        // A→B→C. Sem ciclos. Ordem topológica: A, B, C. Blocos: vazio.
        let g = grafo_modulos(&[(1, "k::a"), (2, "k::b"), (3, "k::c")], &[(1, 2), (2, 3)]);
        let o = ordenar_dsm(&g);
        let nomes: Vec<&str> = o.ordem.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b", "k::c"]);
        assert!(o.blocos.is_empty());
    }

    #[test]
    fn ordenar_um_ciclo_de_dois_modulos_vira_bloco_unico() {
        // A↔B. Bloco {A,B}; só esse SCC. Ordem: A, B (membros internos
        // alfabéticos).
        let g = grafo_modulos(&[(1, "k::a"), (2, "k::b")], &[(1, 2), (2, 1)]);
        let o = ordenar_dsm(&g);
        let nomes: Vec<&str> = o.ordem.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b"]);
        assert_eq!(o.blocos.len(), 1);
        assert_eq!(o.blocos[0].len(), 2);
    }

    #[test]
    fn ordenar_ciclo_mais_depende_dele_e_bloco_vem_depois() {
        // A↔B (bloco), e C→A. C deve vir ANTES do bloco {A,B} na ordem
        // topológica — porque C depende do bloco.
        let g = grafo_modulos(
            &[(1, "k::a"), (2, "k::b"), (3, "k::c")],
            &[(1, 2), (2, 1), (3, 1)],
        );
        let o = ordenar_dsm(&g);
        let nomes: Vec<&str> = o.ordem.iter().map(|p| p.as_str()).collect();
        // c → {a,b}: c vem primeiro (fonte), depois o bloco.
        assert_eq!(nomes, vec!["k::c", "k::a", "k::b"]);
        assert_eq!(o.blocos.len(), 1);
    }

    #[test]
    fn ordenar_dois_ciclos_disjuntos_aparecem_em_ordem_estavel() {
        // {a,b} (bloco 1) e {c,d} (bloco 2). Sem deps entre eles → empate
        // por path: bloco {a,b} vem primeiro.
        let g = grafo_modulos(
            &[(1, "k::a"), (2, "k::b"), (3, "k::c"), (4, "k::d")],
            &[(1, 2), (2, 1), (3, 4), (4, 3)],
        );
        let o = ordenar_dsm(&g);
        let nomes: Vec<&str> = o.ordem.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b", "k::c", "k::d"]);
        assert_eq!(o.blocos.len(), 2);
    }

    #[test]
    fn ordenar_no_isolado_aparece_em_ordem_topologica_valida() {
        // c isolado; a→b. Sem ciclo. Kahn pop'a "k::a" (menor) → libera
        // b. Próximo pop pela chave "k::b" < "k::c". Resultado: a, b, c.
        // (Qualquer ordem topológica é DSM-correta; o algoritmo escolhe
        // a determinística via fila ordenada por path.)
        let g = grafo_modulos(&[(1, "k::a"), (2, "k::b"), (3, "k::c")], &[(1, 2)]);
        let o = ordenar_dsm(&g);
        let nomes: Vec<&str> = o.ordem.iter().map(|p| p.as_str()).collect();
        assert_eq!(nomes, vec!["k::a", "k::b", "k::c"]);
        // Propriedade topológica: o índice de "k::a" < índice de "k::b"
        // (a→b é satisfeita).
        let idx_a = nomes.iter().position(|p| *p == "k::a").unwrap();
        let idx_b = nomes.iter().position(|p| *p == "k::b").unwrap();
        assert!(idx_a < idx_b, "a→b deve apontar 'para frente' na DSM");
        assert!(o.blocos.is_empty());
    }

    #[test]
    fn ordenar_e_deterministico_entre_extracoes() {
        // Duas extrações do mesmo grafo → mesma ordem (sem aleatoriedade).
        let g = grafo_modulos(&[(1, "k::a"), (2, "k::b"), (3, "k::c")], &[(1, 2), (2, 3)]);
        let o1 = ordenar_dsm(&g);
        let o2 = ordenar_dsm(&g);
        assert_eq!(o1, o2);
    }

    /// Teste-consumidor (prompt 0035): a partir de `ordem` + as deps do
    /// grafo, reconstrói a grade N×N e confere que reflete as
    /// dependências originais. Prova que a "matriz como dado" é
    /// suficiente.
    #[test]
    fn consumidor_reconstroi_grade_a_partir_de_ordem_e_deps() {
        let g = grafo_modulos(
            &[(1, "k::a"), (2, "k::b"), (3, "k::c")],
            &[(1, 2), (2, 3), (3, 1)], // a→b→c→a (ciclo de 3)
        );
        let o = ordenar_dsm(&g);

        // Reconstrução: índice de cada path em `ordem`; grade N×N booleana.
        let n = o.ordem.len();
        let mut idx: HashMap<&str, usize> = HashMap::new();
        for (i, p) in o.ordem.iter().enumerate() {
            idx.insert(p.as_str(), i);
        }
        let mut grade = vec![vec![false; n]; n];
        for a in &g.edges {
            if a.relation == Relation::Uses
                && let (Some(&i), Some(&j)) = (idx.get(a.from.as_str()), idx.get(a.to.as_str()))
            {
                grade[i][j] = true;
            }
        }
        // Cada dependência original corresponde a uma célula `true`.
        let total_marcadas: usize = grade.iter().flatten().filter(|c| **c).count();
        assert_eq!(total_marcadas, 3);
        // E o bloco do ciclo cobre os 3 módulos.
        assert_eq!(o.blocos.len(), 1);
        assert_eq!(o.blocos[0].len(), 3);
    }

    fn detalhado_fixture() -> Grafo {
        use lente_core::entities::grafo::UsesKind;
        let mut g = grafo_dois_modulos();
        g.nodes.push(no(12, "k::a::f2", Kind::Fn));
        g.nodes.push(no(99, "orfao", Kind::Fn));
        g.edges
            .push(aresta(10, "k::a", 12, "k::a::f2", Relation::Owns));
        let mut uso = aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses);
        uso.uses_kind = Some(UsesKind::Reference);
        g.edges.extend([uso.clone(), uso.clone()]);
        uso.uses_kind = Some(UsesKind::Import);
        g.edges.push(uso);
        g.edges
            .push(aresta(12, "k::a::f2", 21, "k::b::g", Relation::Uses));
        g.edges
            .push(aresta(11, "k::a::f", 12, "k::a::f2", Relation::Uses));
        g.edges
            .push(aresta(99, "orfao", 11, "k::a::f", Relation::Uses));
        g
    }

    #[test]
    fn detalhe_celula_preserva_paralelas_subtipos_e_campos_originais() {
        use lente_core::entities::grafo::{Posicao, UsesKind};
        let mut g = detalhado_fixture();
        g.nodes[2].position = Some(Posicao {
            file: "declaracao.rs".into(),
            start_line: 2,
            end_line: 4,
        });
        let d = agregar_por_modulo_detalhado(&g);
        assert_eq!(d.dependencias.len(), 1);
        let celula = &d.dependencias[0];
        assert_eq!((celula.id_de, celula.id_para, celula.peso), (10, 20, 4));
        assert_eq!(
            (&celula.de, &celula.para),
            (&Path::from("k::a"), &Path::from("k::b"))
        );
        let evidencias: Vec<_> = celula.evidencias.iter().map(|&i| &g.edges[i]).collect();
        assert_eq!(
            evidencias
                .iter()
                .filter(|a| a.uses_kind == Some(UsesKind::Reference))
                .count(),
            2
        );
        assert_eq!(
            evidencias
                .iter()
                .filter(|a| a.uses_kind == Some(UsesKind::Import))
                .count(),
            1
        );
        assert_eq!(
            evidencias.iter().filter(|a| a.uses_kind.is_none()).count(),
            1
        );
        for r in &d.relacoes {
            assert_eq!(r.aresta, g.edges[r.indice_snapshot]);
        }
    }

    #[test]
    fn detalhe_internas_orfaos_e_owns_recebem_destinos_explicitos() {
        let g = detalhado_fixture();
        let d = agregar_por_modulo_detalhado(&g);
        assert_eq!(
            d.itens_sem_contenedor,
            vec![ItemSemContenedor {
                id: 99,
                path: "orfao".into(),
                problemas: vec![ProblemaContencao::SemModulo]
            }]
        );
        assert_eq!(
            d.relacoes
                .iter()
                .filter(|r| matches!(
                    r.destino,
                    DestinoRelacao::InternaAbsorvida { id_modulo: 10 }
                ))
                .count(),
            1
        );
        assert_eq!(
            d.relacoes
                .iter()
                .filter(|r| matches!(
                    r.destino,
                    DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoOwns)
                ))
                .count(),
            5
        );
        let orfa = d.relacoes.iter().find(|r| r.aresta.id_from == 99).unwrap();
        assert_eq!(
            orfa.destino,
            DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoInvalida {
                origem: vec![ProblemaContencao::SemModulo],
                destino: vec![]
            })
        );
    }

    #[test]
    fn detalhe_multiplos_pais_nao_escolhe_mesmo_quando_convergem() {
        let mut g = grafo_dois_modulos();
        g.nodes.push(no(12, "k::a::T", Kind::Struct));
        g.nodes.push(no(13, "k::a::f::filho", Kind::Fn));
        g.edges.extend([
            aresta(10, "k::a", 12, "k::a::T", Relation::Owns),
            aresta(12, "k::a::T", 11, "k::a::f", Relation::Owns),
            aresta(11, "k::a::f", 13, "k::a::f::filho", Relation::Owns),
            aresta(13, "k::a::f::filho", 21, "k::b::g", Relation::Uses),
        ]);
        let d = agregar_por_modulo_detalhado(&g);
        assert!(d.dependencias.is_empty());
        assert_eq!(d.itens_sem_contenedor.len(), 2);
        for item in d.itens_sem_contenedor {
            assert_eq!(
                item.problemas,
                vec![ProblemaContencao::MultiplosPais {
                    item: 11,
                    pais: vec![10, 12]
                }]
            );
        }
    }

    #[test]
    fn detalhe_ciclos_de_modulos_e_auto_ciclo_invalidam_descendentes() {
        let mut g = grafo_dois_modulos();
        g.edges.retain(|a| !(a.id_from == 1 && a.id_to == 10));
        g.edges.extend([
            aresta(11, "k::a::f", 10, "k::a", Relation::Owns),
            aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses),
            aresta(21, "k::b::g", 21, "k::b::g", Relation::Owns),
        ]);
        let d = agregar_por_modulo_detalhado(&g);
        assert!(d.dependencias.is_empty());
        for id in [10, 11] {
            assert_eq!(
                d.itens_sem_contenedor
                    .iter()
                    .find(|i| i.id == id)
                    .unwrap()
                    .problemas,
                vec![ProblemaContencao::Ciclo {
                    membros: vec![10, 11]
                }]
            );
        }
        let p = &d
            .itens_sem_contenedor
            .iter()
            .find(|i| i.id == 21)
            .unwrap()
            .problemas;
        assert_eq!(
            p,
            &vec![
                ProblemaContencao::MultiplosPais {
                    item: 21,
                    pais: vec![20, 21]
                },
                ProblemaContencao::Ciclo { membros: vec![21] }
            ]
        );
    }

    #[test]
    fn detalhe_owns_paralelos_nao_sao_multiplos_pais() {
        let mut g = grafo_dois_modulos();
        g.edges.push(g.edges[1].clone());
        g.edges
            .push(aresta(11, "k::a::f", 21, "k::b::g", Relation::Uses));
        let d = agregar_por_modulo_detalhado(&g);
        assert!(d.itens_sem_contenedor.is_empty());
        assert_eq!(d.dependencias[0].peso, 1);
    }

    #[test]
    fn detalhe_endpoint_e_pai_ausentes_sao_diagnosticados() {
        let mut g = grafo_dois_modulos();
        g.edges.retain(|a| a.id_to != 11);
        g.edges
            .push(aresta(404, "ausente", 11, "k::a::f", Relation::Owns));
        g.edges
            .push(aresta(11, "k::a::f", 405, "outro", Relation::Uses));
        let d = agregar_por_modulo_detalhado(&g);
        assert_eq!(
            d.itens_sem_contenedor[0].problemas,
            vec![ProblemaContencao::NoAusente { id: 404 }]
        );
        let r = d
            .relacoes
            .iter()
            .find(|r| r.aresta.relation == Relation::Uses)
            .unwrap();
        assert_eq!(
            r.destino,
            DestinoRelacao::NaoAgregavel(MotivoNaoAgregavel::ContencaoInvalida {
                origem: vec![ProblemaContencao::NoAusente { id: 404 }],
                destino: vec![ProblemaContencao::NoAusente { id: 405 }]
            })
        );
    }

    fn conferir_contabilidade(g: &Grafo, d: &AgregacaoDetalhada) {
        let mut todos: Vec<_> = d.relacoes.iter().map(|r| r.indice_snapshot).collect();
        todos.sort_unstable();
        assert_eq!(todos, (0..g.edges.len()).collect::<Vec<_>>());
        let mut agregadas: Vec<_> = d
            .relacoes
            .iter()
            .filter(|r| matches!(r.destino, DestinoRelacao::Agregada { .. }))
            .map(|r| r.indice_snapshot)
            .collect();
        let mut evidencias: Vec<_> = d
            .dependencias
            .iter()
            .flat_map(|d| {
                assert_eq!(d.peso, d.evidencias.len());
                d.evidencias.clone()
            })
            .collect();
        agregadas.sort_unstable();
        evidencias.sort_unstable();
        assert_eq!(agregadas, evidencias);
        for c in &d.dependencias {
            for &i in &c.evidencias {
                assert_eq!(
                    d.relacoes
                        .iter()
                        .find(|r| r.indice_snapshot == i)
                        .unwrap()
                        .destino,
                    DestinoRelacao::Agregada {
                        id_modulo_de: c.id_de,
                        id_modulo_para: c.id_para
                    }
                );
            }
        }
    }

    #[test]
    fn detalhe_contabilidade_e_permutacao_incluem_relacoes_identicas() {
        let mut g = detalhado_fixture();
        let antes = agregar_por_modulo_detalhado(&g);
        conferir_contabilidade(&g, &antes);
        for _ in 0..g.edges.len() {
            g.edges.rotate_left(1);
            g.nodes.reverse();
            let depois = agregar_por_modulo_detalhado(&g);
            conferir_contabilidade(&g, &depois);
            assert_eq!(antes.itens_sem_contenedor, depois.itens_sem_contenedor);
            assert_eq!(
                antes
                    .relacoes
                    .iter()
                    .map(|r| (&r.aresta, &r.destino))
                    .collect::<Vec<_>>(),
                depois
                    .relacoes
                    .iter()
                    .map(|r| (&r.aresta, &r.destino))
                    .collect::<Vec<_>>()
            );
            assert_eq!(antes.dependencias.len(), depois.dependencias.len());
            for (a, b) in antes.dependencias.iter().zip(&depois.dependencias) {
                assert_eq!(
                    (a.id_de, &a.de, a.id_para, &a.para, a.peso),
                    (b.id_de, &b.de, b.id_para, &b.para, b.peso)
                );
            }
        }
    }

    #[test]
    fn detalhe_regressao_pesos_e_arestas_legadas_em_grafos_validos() {
        let mut g = detalhado_fixture();
        g.nodes.retain(|n| n.id != 99);
        g.edges.retain(|a| a.id_from != 99);
        g.edges
            .push(aresta(21, "k::b::g", 11, "k::a::f", Relation::Uses));
        let salvo = g.clone();
        let legado = agregar_por_modulo(&g);
        let pesos = pesos_modulo_a_modulo(&g);
        let d = agregar_por_modulo_detalhado(&g);
        assert_eq!(g, salvo);
        assert_eq!(legado, agregar_por_modulo(&g));
        assert_eq!(
            d.dependencias
                .iter()
                .map(|c| ((c.id_de, c.id_para), c.peso))
                .collect::<HashMap<_, _>>(),
            pesos
        );
        assert_eq!(
            d.dependencias
                .iter()
                .map(|c| (c.id_de, c.id_para))
                .collect::<BTreeSet<_>>(),
            legado
                .edges
                .iter()
                .filter(|a| a.relation == Relation::Uses)
                .map(|a| (a.id_from, a.id_to))
                .collect()
        );
    }

    #[test]
    fn detalhe_permutacao_de_hierarquia_invalida_e_descendente_de_ciclo() {
        let mut g = grafo_dois_modulos();
        g.nodes.push(no(12, "descendente", Kind::Fn));
        g.nodes.push(no(99, "orfao_sem_arestas", Kind::Fn));
        g.edges.extend([
            aresta(11, "k::a::f", 10, "k::a", Relation::Owns),
            aresta(11, "k::a::f", 12, "descendente", Relation::Owns),
            aresta(12, "descendente", 21, "k::b::g", Relation::Uses),
        ]);
        let esperado = agregar_por_modulo_detalhado(&g);
        assert!(esperado.dependencias.is_empty());
        assert!(esperado.itens_sem_contenedor.iter().any(|i| i.id == 12
            && i.problemas.contains(&ProblemaContencao::Ciclo {
                membros: vec![10, 11]
            })));
        assert!(
            esperado
                .itens_sem_contenedor
                .iter()
                .any(|i| i.id == 99 && i.problemas == vec![ProblemaContencao::SemModulo])
        );
        for _ in 0..g.edges.len() {
            g.edges.rotate_left(1);
            g.nodes.rotate_left(1);
            let d = agregar_por_modulo_detalhado(&g);
            conferir_contabilidade(&g, &d);
            assert_eq!(d.itens_sem_contenedor, esperado.itens_sem_contenedor);
            assert_eq!(
                d.relacoes
                    .iter()
                    .map(|r| (&r.aresta, &r.destino))
                    .collect::<Vec<_>>(),
                esperado
                    .relacoes
                    .iter()
                    .map(|r| (&r.aresta, &r.destino))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn detalhe_contenedor_mais_proximo_em_cadeia_profunda_sem_recursao() {
        let mut g = Grafo::new("k");
        g.nodes.push(no(0, "modulo", Kind::Mod));
        for i in 1..4096 {
            g.nodes.push(no(i, &format!("item{i}"), Kind::Struct));
            g.edges.push(aresta(
                i - 1,
                g.nodes[i - 1].path.as_str(),
                i,
                g.nodes[i].path.as_str(),
                Relation::Owns,
            ));
        }
        g.nodes.push(no(4096, "submodulo", Kind::Mod));
        g.edges
            .push(aresta(4095, "item4095", 4096, "submodulo", Relation::Owns));
        g.edges
            .push(aresta(4095, "item4095", 4096, "submodulo", Relation::Uses));
        g.nodes.reverse();
        let d = agregar_por_modulo_detalhado(&g);
        assert!(d.itens_sem_contenedor.is_empty());
        assert_eq!(
            (
                d.dependencias[0].id_de,
                d.dependencias[0].id_para,
                d.dependencias[0].peso
            ),
            (0, 4096, 1)
        );
        assert_eq!(pesos_modulo_a_modulo(&g).get(&(0, 4096)), Some(&1));
        conferir_contabilidade(&g, &d);
        assert!(
            agregar_por_modulo_detalhado(&Grafo::new("vazio"))
                .relacoes
                .is_empty()
        );
    }
}
