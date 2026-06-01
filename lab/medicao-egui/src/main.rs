//! Medição panorama da lente contra o workspace egui.
//!
//! Uso:  medicao-egui <PATH_WORKSPACE_EGUI>
//!
//! Para cada crate-membro do egui: 1 invocação do fork, detecta colisões,
//! resolve, calcula raio de cada nó. Persiste checkpoint por crate em
//! `checkpoints/<crate>.json`. Agrega tudo em `dados.json`.

use std::collections::HashMap;
use std::path::{Path as StdPath, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use lente_core::domain::raio::{Classificacao, calcular_raio};
use lente_core::entities::grafo::{Aresta, Grafo, Path, Relation};
use lente_investiga::{ArestasNo, ParColidente, Vizinhanca};

// Os 12 crates do workspace egui v0.34.3 (members do Cargo.toml).
const CRATES_EGUI: &[&str] = &[
    "ecolor",
    "egui_demo_app",
    "egui_demo_lib",
    "egui_extras",
    "egui_glow",
    "egui_kittest",
    "egui-wgpu",
    "egui-winit",
    "egui",
    "emath",
    "epaint",
    "epaint_default_fonts",
];

#[derive(Debug, Serialize, Deserialize, Default)]
struct PerfilRaios {
    isolados: usize,
    folhas: usize,
    bases: usize,
    intermediarios: usize,
    soma_uses_entrada: usize,
    soma_montante: usize,
    n: usize,
}

impl PerfilRaios {
    fn adicionar(&mut self, classif: Classificacao, uses_entrada: usize, montante: usize) {
        match classif {
            Classificacao::Isolado => self.isolados += 1,
            Classificacao::Folha => self.folhas += 1,
            Classificacao::Base => self.bases += 1,
            Classificacao::Intermediario => self.intermediarios += 1,
        }
        self.soma_uses_entrada += uses_entrada;
        self.soma_montante += montante;
        self.n += 1;
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct ContagemVereditos {
    distintos_vizinhanca_disjunta: usize,
    distintos_impl_traits_diferentes: usize,
    mesmo_item: usize,
    nao_determinado: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct ResultadoCrate {
    nome: String,
    erro: Option<String>,
    tempo_fork_seg: f64,
    tempo_total_seg: f64,
    nodes_antes: usize,
    nodes_depois: usize,
    edges: usize,
    colisoes: usize,
    vereditos: ContagemVereditos,
    /// Padrões dos `NaoDeterminado`: até 5 paths colidentes que ficaram sem
    /// resolução (amostra para análise qualitativa).
    nao_determinado_amostra: Vec<String>,
    /// Para colisões resolvidas como `Distintos`: quantas tinham `trait_ref`
    /// distinto entre as cópias (sinal de que `trait_ref` está trabalhando
    /// na diferenciação).
    trait_ref_distinto: usize,
    /// Para colisões resolvidas como `Distintos`: quantas tinham `trait_ref`
    /// igual (ou `None` em ambas) — outra evidência discrimina.
    trait_ref_igual: usize,
    perfil: PerfilRaios,
    /// Top-10 nós por tamanho do raio transitivo (montante).
    top10_transitivos: Vec<(String, usize)>,
    /// Diagnóstico do "limite Folha/comportamental" — quantos nós com nome
    /// `fmt`, `from`, `default`, `clone`, `eq`, `hash`, `cmp` aparecem como
    /// Folha com 0 transitivos.
    folhas_comportamentais: usize,
}

#[derive(Debug, Serialize)]
struct Agregado<'a> {
    crates_total: usize,
    crates_ok: usize,
    crates_com_erro: usize,
    tempo_total_seg: f64,
    nodes_total: usize,
    edges_total: usize,
    colisoes_total: usize,
    vereditos_total: ContagemVereditos,
    perfil_total: PerfilRaios,
    folhas_comportamentais_total: usize,
    crates: &'a [ResultadoCrate],
}

// ----- Pipeline replicado do lente_wiring (precisamos do grafo intermediário) -----

fn detectar_colisoes(grafo: &Grafo) -> Vec<Path> {
    let mut por_path: HashMap<&Path, usize> = HashMap::new();
    for n in &grafo.nodes {
        *por_path.entry(&n.path).or_insert(0) += 1;
    }
    por_path
        .into_iter()
        .filter(|(_, c)| *c > 1)
        .map(|(p, _)| p.clone())
        .collect()
}

fn construir_vizinhanca(grafo: &Grafo, id_a: usize, id_b: usize) -> Vizinhanca {
    let mut va: ArestasNo = ArestasNo::default();
    let mut vb: ArestasNo = ArestasNo::default();
    for a in &grafo.edges {
        if a.id_to == id_a {
            va.entrando.push(clonar_aresta(a));
        }
        if a.id_from == id_a {
            va.saindo.push(clonar_aresta(a));
        }
        if a.id_to == id_b {
            vb.entrando.push(clonar_aresta(a));
        }
        if a.id_from == id_b {
            vb.saindo.push(clonar_aresta(a));
        }
    }
    Vizinhanca { a: va, b: vb }
}

fn clonar_aresta(a: &Aresta) -> Aresta {
    a.clone()
}

// ----- Medição de um crate -----

fn medir_crate(
    workspace: &StdPath,
    nome: &str,
) -> Result<ResultadoCrate, Box<dyn std::error::Error>> {
    use lente_core::entities::veredito::{Evidencia, Veredito};

    let t_inicio = Instant::now();
    let crate_path = workspace.join("crates").join(nome);

    // 1. Fork + desserialização (lente_infra::extrair_grafo faz os dois).
    let t_fork_start = Instant::now();
    let mut grafo = lente_infra::extrair_grafo(&crate_path)?;
    let tempo_fork = t_fork_start.elapsed().as_secs_f64();
    let nodes_antes = grafo.nodes.len();
    let edges = grafo.edges.len();

    // 2. Detectar colisões.
    let colisoes_paths = detectar_colisoes(&grafo);
    let n_colisoes = colisoes_paths.len();

    let mut vereditos = ContagemVereditos::default();
    let mut nao_determinado_amostra: Vec<String> = Vec::new();
    let mut trait_ref_distinto = 0usize;
    let mut trait_ref_igual = 0usize;

    for path_colidente in &colisoes_paths {
        // ids ordenados das cópias colidentes
        let mut ids: Vec<usize> = grafo
            .nodes
            .iter()
            .filter(|n| &n.path == path_colidente)
            .map(|n| n.id)
            .collect();
        if ids.len() < 2 {
            continue;
        }
        ids.sort_unstable();
        let (id_a, id_b) = (ids[0], ids[1]);

        // trait_ref distinct?
        let tref_a = grafo.nodes.iter().find(|n| n.id == id_a).and_then(|n| n.trait_ref.clone());
        let tref_b = grafo.nodes.iter().find(|n| n.id == id_b).and_then(|n| n.trait_ref.clone());
        match (tref_a.as_deref(), tref_b.as_deref()) {
            (Some(a), Some(b)) if a != b => trait_ref_distinto += 1,
            _ => trait_ref_igual += 1,
        }

        let viz = construir_vizinhanca(&grafo, id_a, id_b);
        let no_a = grafo.nodes.iter().find(|n| n.id == id_a).unwrap();
        let no_b = grafo.nodes.iter().find(|n| n.id == id_b).unwrap();
        let par = ParColidente { a: no_a, b: no_b };

        let veredito = lente_investiga::investigar(par, &viz, None);

        match &veredito {
            Veredito::Distintos { evidencia: Evidencia::VizinhancaDisjunta { .. } } => {
                vereditos.distintos_vizinhanca_disjunta += 1
            }
            Veredito::Distintos { evidencia: Evidencia::ImplDeTraitsDiferentes { .. } } => {
                vereditos.distintos_impl_traits_diferentes += 1
            }
            Veredito::MesmoItem => vereditos.mesmo_item += 1,
            Veredito::NaoDeterminado { .. } => {
                vereditos.nao_determinado += 1;
                if nao_determinado_amostra.len() < 5 {
                    nao_determinado_amostra.push(path_colidente.as_str().to_string());
                }
            }
        }

        // Aplicar: se NaoDeterminado, propaga erro — mas para a medição,
        // continuamos com o grafo sem renomear esse path. Outros paths podem
        // ser resolvidos.
        match lente_resolve::aplicar(&grafo, path_colidente, &veredito) {
            Ok(g_novo) => grafo = g_novo,
            Err(_) => {
                // NaoDeterminado: deixa a colisão; o cálculo do raio ainda
                // funciona, só que com path ambíguo. Aceitamos.
            }
        }
    }

    // 3. Perfil de raios + top-10.
    let mut perfil = PerfilRaios::default();
    let mut transitivos: Vec<(String, usize)> = Vec::new();
    let mut folhas_comportamentais = 0usize;
    let nomes_comportamentais = [
        "fmt", "from", "into", "default", "clone", "eq", "ne", "hash", "cmp",
        "partial_cmp", "as_ref", "as_mut", "deref", "deref_mut", "drop",
    ];

    // Para evitar trabalho repetido em paths colidentes (que viraram únicos
    // após resolução, mas pode haver caso bug onde sobreviveram), iteramos
    // sobre paths únicos, mas chamando calcular_raio por nó é só pelo path
    // -- então um raio por path é o mesmo independente do número de nós.
    let paths_unicos: Vec<Path> = {
        let mut s: Vec<Path> = grafo.nodes.iter().map(|n| n.path.clone()).collect();
        s.sort();
        s.dedup();
        s
    };
    for p in &paths_unicos {
        if let Ok(r) = calcular_raio(&grafo, p) {
            perfil.adicionar(r.classificacao, r.uses_entrada, r.montante.len());
            transitivos.push((p.as_str().to_string(), r.montante.len()));
            // folhas comportamentais: nome do método (último segmento sem <>)
            // bate com a lista, classificação Folha, montante 0.
            if r.classificacao == Classificacao::Folha && r.montante.is_empty() {
                let nome_metodo = p
                    .as_str()
                    .rsplit("::")
                    .next()
                    .unwrap_or("");
                if nomes_comportamentais.contains(&nome_metodo) {
                    folhas_comportamentais += 1;
                }
            }
        }
    }
    transitivos.sort_by_key(|(_, t)| std::cmp::Reverse(*t));
    transitivos.truncate(10);

    Ok(ResultadoCrate {
        nome: nome.to_string(),
        erro: None,
        tempo_fork_seg: tempo_fork,
        tempo_total_seg: t_inicio.elapsed().as_secs_f64(),
        nodes_antes,
        nodes_depois: grafo.nodes.len(),
        edges,
        colisoes: n_colisoes,
        vereditos,
        nao_determinado_amostra,
        trait_ref_distinto,
        trait_ref_igual,
        perfil,
        top10_transitivos: transitivos,
        folhas_comportamentais,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let workspace_egui = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home/dikluwe/Documentos/GitHub/egui"));

    // Dirs de saída relativos ao cwd onde o binário é invocado.
    let out_dir = PathBuf::from("lab/medicao-egui");
    let cp_dir = out_dir.join("checkpoints");
    std::fs::create_dir_all(&cp_dir)?;

    let t_global = Instant::now();
    let mut resultados: Vec<ResultadoCrate> = Vec::new();

    for nome in CRATES_EGUI {
        let cp_path = cp_dir.join(format!("{}.json", nome));
        if cp_path.exists() {
            eprintln!(">>> {} (checkpoint, pula)", nome);
            let texto = std::fs::read_to_string(&cp_path)?;
            let res: ResultadoCrate = serde_json::from_str(&texto)?;
            resultados.push(res);
            continue;
        }
        eprintln!(">>> {}", nome);
        let t0 = Instant::now();
        match medir_crate(&workspace_egui, nome) {
            Ok(res) => {
                eprintln!(
                    "  ok em {:.1}s — nodes={}, edges={}, colisões={}",
                    res.tempo_total_seg, res.nodes_antes, res.edges, res.colisoes
                );
                std::fs::write(&cp_path, serde_json::to_string_pretty(&res)?)?;
                resultados.push(res);
            }
            Err(e) => {
                let msg = e.to_string();
                eprintln!("  ERRO em {:.1}s: {}", t0.elapsed().as_secs_f64(), msg);
                let res = ResultadoCrate {
                    nome: nome.to_string(),
                    erro: Some(msg),
                    tempo_fork_seg: 0.0,
                    tempo_total_seg: t0.elapsed().as_secs_f64(),
                    nodes_antes: 0,
                    nodes_depois: 0,
                    edges: 0,
                    colisoes: 0,
                    vereditos: ContagemVereditos::default(),
                    nao_determinado_amostra: Vec::new(),
                    trait_ref_distinto: 0,
                    trait_ref_igual: 0,
                    perfil: PerfilRaios::default(),
                    top10_transitivos: Vec::new(),
                    folhas_comportamentais: 0,
                };
                std::fs::write(&cp_path, serde_json::to_string_pretty(&res)?)?;
                resultados.push(res);
            }
        }
    }

    // Agregar
    let mut crates_ok = 0;
    let mut crates_com_erro = 0;
    let mut nodes_total = 0;
    let mut edges_total = 0;
    let mut colisoes_total = 0;
    let mut vereditos_total = ContagemVereditos::default();
    let mut perfil_total = PerfilRaios::default();
    let mut folhas_comportamentais_total = 0;
    for r in &resultados {
        if r.erro.is_some() {
            crates_com_erro += 1;
            continue;
        }
        crates_ok += 1;
        nodes_total += r.nodes_antes;
        edges_total += r.edges;
        colisoes_total += r.colisoes;
        vereditos_total.distintos_vizinhanca_disjunta +=
            r.vereditos.distintos_vizinhanca_disjunta;
        vereditos_total.distintos_impl_traits_diferentes +=
            r.vereditos.distintos_impl_traits_diferentes;
        vereditos_total.mesmo_item += r.vereditos.mesmo_item;
        vereditos_total.nao_determinado += r.vereditos.nao_determinado;
        perfil_total.isolados += r.perfil.isolados;
        perfil_total.folhas += r.perfil.folhas;
        perfil_total.bases += r.perfil.bases;
        perfil_total.intermediarios += r.perfil.intermediarios;
        perfil_total.soma_uses_entrada += r.perfil.soma_uses_entrada;
        perfil_total.soma_montante += r.perfil.soma_montante;
        perfil_total.n += r.perfil.n;
        folhas_comportamentais_total += r.folhas_comportamentais;
    }

    let agreg = Agregado {
        crates_total: resultados.len(),
        crates_ok,
        crates_com_erro,
        tempo_total_seg: t_global.elapsed().as_secs_f64(),
        nodes_total,
        edges_total,
        colisoes_total,
        vereditos_total,
        perfil_total,
        folhas_comportamentais_total,
        crates: &resultados,
    };

    let dados_path = out_dir.join("dados.json");
    std::fs::write(&dados_path, serde_json::to_string_pretty(&agreg)?)?;
    eprintln!(
        "\n=== AGREGADO ===\ncrates {}/{} ok, tempo total {:.1}s, nodes {}, colisões {}",
        crates_ok, agreg.crates_total, agreg.tempo_total_seg, nodes_total, colisoes_total
    );
    Ok(())
}
