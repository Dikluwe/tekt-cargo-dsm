//! Remedição de colisões de path com o fork novo (commit 5fbcdfe8) —
//! agora com identidade-por-nó. Exercita o `lente_investiga` real.
//!
//! Uso:
//!   remedicao <dir-com-jsons> <dir-com-crates-fonte>
//!
//! Saídas:
//!   - `analise.json` (resultado estruturado)
//!   - `relatorio-base.md` (esqueleto do relatório com tabelas, sem interpretação)

use std::collections::HashMap;
use std::fs;
use std::path::{Path as StdPath, PathBuf};

use serde::Deserialize;

use lente_core::entities::grafo::{
    Aresta, Kind, No, Path as PathGrafo, Relation, Visibility,
};
use lente_core::entities::veredito::{Evidencia, Veredito};
use lente_investiga::{
    ArestasNo, ArquivoFonte, ParColidente, Vizinhanca, investigar,
};

#[derive(Debug, Deserialize)]
struct GrafoJson {
    #[serde(rename = "crate")]
    crate_name: String,
    nodes: Vec<NoJson>,
    edges: Vec<ArestaJson>,
}

#[derive(Debug, Deserialize, Clone)]
struct NoJson {
    id: usize,
    path: String,
    name: String,
    kind: String,
    visibility: String,
}

#[derive(Debug, Deserialize, Clone)]
struct ArestaJson {
    from: String,
    id_from: usize,
    to: String,
    id_to: usize,
    relation: String,
}

#[derive(Debug, serde::Serialize)]
struct ResultadoColisao {
    path: String,
    n_nos: usize,
    veredito: String,
    estrategia: String, // "E1" | "E2" | "—"
    detalhe: String,    // evidência ou diagnóstico
}

#[derive(Debug, serde::Serialize)]
struct ResultadoCrate {
    nome: String,
    total_nodes: usize,
    total_edges: usize,
    colisoes_totais: usize,
    colisoes_proprias: usize,
    decididas_e1: usize,
    decididas_e2: usize,
    nao_determinado: usize,
    detalhes: Vec<ResultadoColisao>,
}

fn no_para_lente(n: &NoJson) -> No {
    No {
        id: n.id,
        path: PathGrafo::from(n.path.as_str()),
        name: n.name.clone(),
        kind: Kind::try_from(n.kind.as_str()).unwrap_or(Kind::Mod),
        modificadores: Default::default(),
        visibility: Visibility::try_from(n.visibility.as_str())
            .unwrap_or(Visibility::Priv),
        crate_name: String::new(),
        trait_: None,
        trait_ref: None,
        cfg: None,
        macro_kind: None,
        is_non_exhaustive: false,
    }
}

fn aresta_para_lente(a: &ArestaJson) -> Option<Aresta> {
    let relation = Relation::try_from(a.relation.as_str()).ok()?;
    Some(Aresta {
        from: PathGrafo::from(a.from.as_str()),
        id_from: a.id_from,
        to: PathGrafo::from(a.to.as_str()),
        id_to: a.id_to,
        relation,
    })
}

fn vizinhanca_do_par(
    a: &NoJson,
    b: &NoJson,
    arestas: &[ArestaJson],
) -> Vizinhanca {
    let mut v_a = ArestasNo::default();
    let mut v_b = ArestasNo::default();
    for ar in arestas {
        if let Some(conv) = aresta_para_lente(ar) {
            if ar.id_to == a.id {
                v_a.entrando.push(conv.clone());
            }
            if ar.id_from == a.id {
                v_a.saindo.push(conv.clone());
            }
            if ar.id_to == b.id {
                v_b.entrando.push(conv.clone());
            }
            if ar.id_from == b.id {
                v_b.saindo.push(conv);
            }
        }
    }
    Vizinhanca { a: v_a, b: v_b }
}

fn descrever_veredito(v: &Veredito) -> (String, String) {
    match v {
        Veredito::MesmoItem => ("MesmoItem".to_string(), String::new()),
        Veredito::Distintos { evidencia } => match evidencia {
            Evidencia::VizinhancaDisjunta {
                exclusivas_a,
                exclusivas_b,
            } => (
                "Distintos/VizinhancaDisjunta".to_string(),
                format!("exclusivas_a={}, exclusivas_b={}", exclusivas_a, exclusivas_b),
            ),
            Evidencia::ImplDeTraitsDiferentes { traits } => (
                "Distintos/ImplDeTraitsDiferentes".to_string(),
                format!("traits=({:?}, {:?})", traits.0, traits.1),
            ),
        },
        Veredito::NaoDeterminado { diagnostico } => {
            ("NaoDeterminado".to_string(), diagnostico.clone())
        }
    }
}

fn ler_arquivos_rs(diretorio: &StdPath) -> Vec<ArquivoFonte> {
    let mut out = Vec::new();
    let src = diretorio.join("src");
    if !src.exists() {
        return out;
    }
    fn caminhar(dir: &StdPath, acc: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    caminhar(&p, acc);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    acc.push(p);
                }
            }
        }
    }
    let mut paths = Vec::new();
    caminhar(&src, &mut paths);
    for p in paths {
        if let Ok(conteudo) = fs::read_to_string(&p) {
            out.push(ArquivoFonte {
                caminho_logico: p.to_string_lossy().into_owned(),
                conteudo,
            });
        }
    }
    out
}

fn analisar_crate(
    json_path: &StdPath,
    crates_dir: &StdPath,
) -> Result<ResultadoCrate, Box<dyn std::error::Error>> {
    let texto = fs::read_to_string(json_path)?;
    let grafo: GrafoJson = serde_json::from_str(&texto)?;

    // Diretório do crate-fonte (typst-utils → crates/typst-utils/)
    let dir_crate = crates_dir.join(grafo.crate_name.replace('_', "-"));
    let dir_crate_opt = if dir_crate.exists() {
        Some(dir_crate)
    } else {
        None
    };

    // Detectar colisões de path
    let mut por_path: HashMap<String, Vec<NoJson>> = HashMap::new();
    for n in &grafo.nodes {
        por_path.entry(n.path.clone()).or_default().push(n.clone());
    }
    let colisoes_totais = por_path.values().filter(|v| v.len() > 1).count();

    // Próprias: path começa com o nome do crate
    let prefixo = format!("{}::", grafo.crate_name);
    let proprias: Vec<(&String, &Vec<NoJson>)> = por_path
        .iter()
        .filter(|(p, ns)| {
            ns.len() > 1 && (p.as_str() == grafo.crate_name || p.starts_with(&prefixo))
        })
        .collect();

    let mut detalhes = Vec::new();
    let mut decididas_e1 = 0usize;
    let mut decididas_e2 = 0usize;
    let mut nao_det = 0usize;

    // Ler fontes uma vez se vamos precisar.
    let mut fontes: Option<Vec<ArquivoFonte>> = None;

    for (path, nos) in &proprias {
        // Investiga só o primeiro par (consistência com a primeira medição).
        let a = &nos[0];
        let b = &nos[1];
        let viz = vizinhanca_do_par(a, b, &grafo.edges);
        let no_a = no_para_lente(a);
        let no_b = no_para_lente(b);
        let par = ParColidente { a: &no_a, b: &no_b };

        // E1 isolada (sem fontes)
        let v1 = investigar(par, &viz, None);
        let (rotulo1, detalhe1) = descrever_veredito(&v1);
        let mut estrategia = String::from("—");
        let mut veredito_final = rotulo1.clone();
        let mut detalhe_final = detalhe1.clone();

        match &v1 {
            Veredito::Distintos { .. } | Veredito::MesmoItem => {
                estrategia = "E1".to_string();
                decididas_e1 += 1;
            }
            Veredito::NaoDeterminado { .. } => {
                // Tentar E2 com fontes
                if fontes.is_none() {
                    if let Some(d) = &dir_crate_opt {
                        fontes = Some(ler_arquivos_rs(d));
                    }
                }
                if let Some(fs_vec) = fontes.as_ref() {
                    let v2 = investigar(par, &viz, Some(fs_vec.as_slice()));
                    let (rotulo2, detalhe2) = descrever_veredito(&v2);
                    veredito_final = rotulo2.clone();
                    detalhe_final = detalhe2.clone();
                    match &v2 {
                        Veredito::Distintos { .. } | Veredito::MesmoItem => {
                            estrategia = "E2".to_string();
                            decididas_e2 += 1;
                        }
                        Veredito::NaoDeterminado { .. } => {
                            estrategia = "—".to_string();
                            nao_det += 1;
                        }
                    }
                } else {
                    // sem fontes disponíveis (diretório não encontrado)
                    nao_det += 1;
                }
            }
        }

        detalhes.push(ResultadoColisao {
            path: (*path).clone(),
            n_nos: nos.len(),
            veredito: veredito_final,
            estrategia,
            detalhe: detalhe_final,
        });
    }

    Ok(ResultadoCrate {
        nome: grafo.crate_name.clone(),
        total_nodes: grafo.nodes.len(),
        total_edges: grafo.edges.len(),
        colisoes_totais,
        colisoes_proprias: proprias.len(),
        decididas_e1,
        decididas_e2,
        nao_determinado: nao_det,
        detalhes,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("uso: remedicao <dir_jsons> <dir_crates_fonte>");
        std::process::exit(1);
    }
    let dir_jsons = StdPath::new(&args[1]);
    let dir_crates = StdPath::new(&args[2]);

    let mut jsons: Vec<PathBuf> = fs::read_dir(dir_jsons)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("json"))
        .collect();
    jsons.sort();

    let mut resultados = Vec::new();
    for j in &jsons {
        eprintln!(">>> {}", j.display());
        match analisar_crate(j, dir_crates) {
            Ok(r) => resultados.push(r),
            Err(e) => eprintln!("  ERRO: {}", e),
        }
    }

    // Agregar e serializar
    #[derive(serde::Serialize)]
    struct Agregados {
        total_colisoes_proprias: usize,
        decididas_e1: usize,
        decididas_e2: usize,
        nao_determinado: usize,
    }
    let agreg = Agregados {
        total_colisoes_proprias: resultados.iter().map(|r| r.colisoes_proprias).sum(),
        decididas_e1: resultados.iter().map(|r| r.decididas_e1).sum(),
        decididas_e2: resultados.iter().map(|r| r.decididas_e2).sum(),
        nao_determinado: resultados.iter().map(|r| r.nao_determinado).sum(),
    };

    #[derive(serde::Serialize)]
    struct Saida<'a> {
        agregados: &'a Agregados,
        por_crate: &'a [ResultadoCrate],
    }
    let saida = Saida {
        agregados: &agreg,
        por_crate: &resultados,
    };
    println!("{}", serde_json::to_string_pretty(&saida)?);

    Ok(())
}
