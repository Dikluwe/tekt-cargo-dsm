//! Protótipo de Arena (laudo 0039 — segunda rodada do 0038): impacto de um
//! `git diff` mapeado em nós, agora **multi-crate** com impacto cruzando
//! crates.
//!
//! Pipeline:
//! 1. `cargo metadata --no-deps` para descobrir crates-membros do workspace.
//! 2. Ler `git diff` (stdin OU `git diff HEAD` invocado).
//! 3. Mapear cada arquivo do diff → crate dono (por prefixo de path).
//! 4. Extrair grafo de **cada crate tocado** (per-crate; rápido).
//! 5. Extrair grafo de **todos os crates do workspace** (workspace; custo
//!    maior — ~3.5s por crate × N crates) e **unir por path** (caminho B
//!    do prompt 0039) — IDs reatribuídos globalmente porque o petgraph
//!    emite IDs **instáveis entre extrações**.
//! 6. Para cada nó tocado: calcular o raio **no grafo do crate** (local)
//!    e **no grafo unido** (workspace) — o delta mostra o impacto que
//!    atravessa a fronteira do crate.
//! 7. Emitir JSON estruturado por crate. UI consome em camadas.
//!
//! O prompt 0038 confirmou: 119 nós com `position` em `lente_core`. Aqui
//! a contagem `nodes_com_position` por crate diz se a 5ª-rodada do fork
//! está respondendo coerentemente.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io::Read;
use std::path::{Path as StdPath, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use lente_core::domain::raio::{Classificacao, Raio, calcular_raio};
use lente_core::entities::grafo::{
    Aresta, Grafo, Modificadores, No, Path as PathGrafo, Posicao, Relation, UsesKind, Visibility,
};
use lente_core::entities::veredito::Veredito;
use lente_investiga::{ArestasNo, ParColidente, Vizinhanca};

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModoInput {
    Stdin,
    Git,
    Ambos,
}

#[derive(Debug)]
struct Args {
    repo: PathBuf,
    input: ModoInput,
    out: Option<PathBuf>,
    so_tocados: bool,
    cache_dir: PathBuf,
    limpar_cache: bool,
    invalidar: Vec<String>,
    /// Experimento de renomeação (prompt 0040 §4): após popular o cache,
    /// reescreve um path no cache para simular renomeação cross-crate.
    /// Sintaxe: `--simular-renomeacao <path_velho>=><path_novo>`.
    simular_renomeacao: Option<(String, String)>,
    /// Pula a etapa de resolução por-crate (prompt 0041): a união vai
    /// **fundir** os colididos. Usado para o antes/depois (cru-fundido vs
    /// resolvido) e para reproduzir o comportamento dos laudos 0039/0040.
    sem_resolucao: bool,
    /// Tocar sinteticamente um path colidido para o antes/depois do raio
    /// (prompt 0041 §4). Sintaxe: `--simular-tocar-colidido <path>`. O
    /// path é avaliado **antes** da resolução (cru-fundido) e **depois**
    /// (cada cópia distinta); o JSON traz os dois raios.
    simular_tocar_colidido: Option<String>,
}

fn parse_args() -> Args {
    let argv: Vec<String> = std::env::args().collect();
    let mut repo: Option<PathBuf> = None;
    let mut input = ModoInput::Ambos;
    let mut out: Option<PathBuf> = None;
    let mut so_tocados = false;
    let mut cache_dir: Option<PathBuf> = None;
    let mut limpar_cache = false;
    let mut invalidar: Vec<String> = Vec::new();
    let mut simular_renomeacao: Option<(String, String)> = None;
    let mut sem_resolucao = false;
    let mut simular_tocar_colidido: Option<String> = None;
    let mut i = 1;
    while i < argv.len() {
        let a = &argv[i];
        let next = || -> &str { argv.get(i + 1).map(|s| s.as_str()).unwrap_or("") };
        match a.as_str() {
            "--repo" => {
                repo = Some(PathBuf::from(next()));
                i += 2;
            }
            "--input" => {
                input = match next() {
                    "stdin" => ModoInput::Stdin,
                    "git" => ModoInput::Git,
                    "ambos" | "" => ModoInput::Ambos,
                    outro => {
                        eprintln!("input desconhecido: {} (use stdin|git|ambos)", outro);
                        std::process::exit(2);
                    }
                };
                i += 2;
            }
            "--out" => {
                out = Some(PathBuf::from(next()));
                i += 2;
            }
            "--so-tocados" => {
                so_tocados = true;
                i += 1;
            }
            "--cache-dir" => {
                cache_dir = Some(PathBuf::from(next()));
                i += 2;
            }
            "--limpar-cache" => {
                limpar_cache = true;
                i += 1;
            }
            "--invalidar" => {
                // Lista separada por vírgula de nomes de crate cujo cache
                // será apagado antes de rodar — simula edição naqueles
                // crates sem mexer no repo. Para cronometrar morno-N.
                invalidar = next().split(',').map(|s| s.trim().to_string()).collect();
                i += 2;
            }
            "--simular-renomeacao" => {
                let arg = next();
                if let Some((a, b)) = arg.split_once("=>") {
                    simular_renomeacao = Some((a.trim().to_string(), b.trim().to_string()));
                } else {
                    eprintln!("--simular-renomeacao espera <path>=><novo_path>");
                    std::process::exit(2);
                }
                i += 2;
            }
            "--sem-resolucao" => {
                sem_resolucao = true;
                i += 1;
            }
            "--simular-tocar-colidido" => {
                simular_tocar_colidido = Some(next().to_string());
                i += 2;
            }
            "-h" | "--help" => {
                eprintln!(
                    "uso: proto-impacto-diff [--repo <dir>] [--input stdin|git|ambos] \
                     [--out <arquivo>] [--so-tocados] \
                     [--cache-dir <dir>] [--limpar-cache] \
                     [--invalidar <crate1,crate2>] \
                     [--simular-renomeacao <path>=><novo>] \
                     [--sem-resolucao] \
                     [--simular-tocar-colidido <path>]"
                );
                std::process::exit(0);
            }
            _ => {
                eprintln!("argumento desconhecido: {}", a);
                std::process::exit(2);
            }
        }
    }
    let repo = repo.unwrap_or_else(|| {
        let out = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .expect("git rev-parse falhou");
        PathBuf::from(String::from_utf8(out.stdout).unwrap_or_default().trim())
    });
    let cache_dir = cache_dir.unwrap_or_else(|| {
        // Default: <bin>/../cache — colocar o cache na pasta da Arena.
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cache")
    });
    Args {
        repo,
        input,
        out,
        so_tocados,
        cache_dir,
        limpar_cache,
        invalidar,
        simular_renomeacao,
        sem_resolucao,
        simular_tocar_colidido,
    }
}

// ---------------------------------------------------------------------------
// Descoberta de crates via `cargo metadata`
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MetadataOut {
    packages: Vec<MetadataPackage>,
    workspace_members: Vec<String>,
    workspace_root: String,
}

#[derive(Debug, Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: String,
    id: String,
    #[serde(default)]
    targets: Vec<MetadataTarget>,
}

#[derive(Debug, Deserialize)]
struct MetadataTarget {
    name: String,
    kind: Vec<String>,
}

/// Alvo de seleção do `cargo modules export-json` (mesmo que o
/// `lente_infra::metadata::AlvoFork` faz internamente). Replicado na
/// Arena para escolher direto sem chamar `lente_infra::extrair_grafo`
/// (que produz `Grafo`, não JSON).
#[derive(Debug, Clone)]
enum AlvoFork {
    Lib,
    Bin(String),
}

#[derive(Debug, Clone)]
struct CrateInfo {
    nome: String,
    /// Diretório que contém o `Cargo.toml`.
    diretorio: PathBuf,
    /// Caminho relativo do diretório à raiz do repo (ex.: `01_core`).
    rel_dir: String,
    alvo: AlvoFork,
}

fn descobrir_workspace(repo: &StdPath) -> Vec<CrateInfo> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(repo)
        .output()
        .expect("cargo metadata falhou");
    if !out.status.success() {
        eprintln!(
            "cargo metadata exit != 0: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
        std::process::exit(1);
    }
    let m: MetadataOut =
        serde_json::from_slice(&out.stdout).expect("parse cargo metadata");
    let ids: BTreeSet<String> = m.workspace_members.into_iter().collect();
    let raiz = PathBuf::from(&m.workspace_root);
    m.packages
        .into_iter()
        .filter(|p| ids.contains(&p.id))
        .filter_map(|p| {
            let manifesto = PathBuf::from(&p.manifest_path);
            let diretorio = manifesto.parent().unwrap_or(StdPath::new("")).to_path_buf();
            let rel_dir = diretorio
                .strip_prefix(&raiz)
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let alvo = escolher_alvo(&p.targets)?;
            Some(CrateInfo {
                nome: p.name,
                diretorio,
                rel_dir,
                alvo,
            })
        })
        .collect()
}

/// Mesma regra do `lente_infra::metadata::selecionar_alvo` (laudo 0023):
/// tem `lib` (qualquer variante) → Lib; senão, único `bin` → Bin(nome);
/// outros casos não suportados aqui (Arena).
fn escolher_alvo(targets: &[MetadataTarget]) -> Option<AlvoFork> {
    const KINDS_LIB: &[&str] = &["lib", "rlib", "dylib", "cdylib", "staticlib", "proc-macro"];
    let tem_lib = targets
        .iter()
        .any(|t| t.kind.iter().any(|k| KINDS_LIB.contains(&k.as_str())));
    if tem_lib {
        return Some(AlvoFork::Lib);
    }
    let bins: Vec<&MetadataTarget> = targets
        .iter()
        .filter(|t| t.kind.iter().any(|k| k == "bin"))
        .collect();
    if bins.len() == 1 {
        Some(AlvoFork::Bin(bins[0].name.clone()))
    } else {
        None
    }
}

/// Mapeia um caminho (relativo à raiz do repo) ao crate-membro dono. Faz
/// **maior-prefixo-casa**: se o repo tem `02_shell/cli` e `02_shell`, a
/// busca mais específica vence.
fn crate_de_arquivo<'a>(arq_rel: &str, crates: &'a [CrateInfo]) -> Option<&'a CrateInfo> {
    let mut melhor: Option<&CrateInfo> = None;
    let mut melhor_len = 0usize;
    for c in crates {
        let prefixo = &c.rel_dir;
        if prefixo.is_empty() {
            continue;
        }
        let casa = arq_rel == prefixo.as_str()
            || arq_rel.starts_with(&format!("{}/", prefixo));
        if casa && prefixo.len() > melhor_len {
            melhor = Some(c);
            melhor_len = prefixo.len();
        }
    }
    melhor
}

// ---------------------------------------------------------------------------
// Cache do JSON cru por crate (prompt 0040)
// ---------------------------------------------------------------------------
//
// Chave de invalidação: SHA-256 do conteúdo dos `.rs` sob `src/` do crate.
// **Não** usa commit-hash — o uso reativo tem edições não-comitadas, e a
// chave precisa pegar isso. O hash é determinístico entre execuções
// (SHA-256, ao contrário do `DefaultHasher` da stdlib, cujo seed muda
// entre rodadas).

/// Lista recursivamente todos os arquivos `.rs` sob `dir`, em ordem
/// determinística (pelo path absoluto), retornando seus conteúdos.
fn coletar_fontes(dir: &StdPath) -> Vec<(PathBuf, Vec<u8>)> {
    let mut acc: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    fn walk(d: &StdPath, acc: &mut Vec<(PathBuf, Vec<u8>)>) {
        let Ok(rd) = std::fs::read_dir(d) else {
            return;
        };
        let mut entradas: Vec<PathBuf> = rd
            .flatten()
            .map(|e| e.path())
            .collect();
        entradas.sort();
        for p in entradas {
            if p.is_dir() {
                // Pula target/ por defesa (não deveria estar sob src/).
                if p.file_name().map(|s| s == "target").unwrap_or(false) {
                    continue;
                }
                walk(&p, acc);
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                if let Ok(bytes) = std::fs::read(&p) {
                    acc.push((p, bytes));
                }
            }
        }
    }
    walk(dir, &mut acc);
    acc
}

/// Hash SHA-256 da árvore de fontes do crate: para cada `.rs` em `src/`,
/// concatena `path-relativo + 0x00 + tamanho + 0x00 + conteúdo + 0x00`.
/// Determinístico entre execuções e sensível a qualquer mudança de
/// conteúdo ou estrutura de arquivo.
fn hash_fontes(crate_dir: &StdPath) -> String {
    let src = crate_dir.join("src");
    let arquivos = coletar_fontes(&src);
    let mut hasher = Sha256::new();
    for (p, conteudo) in &arquivos {
        let rel = p.strip_prefix(&src).unwrap_or(p).to_string_lossy();
        hasher.update(rel.as_bytes());
        hasher.update(b"\0");
        hasher.update(conteudo.len().to_le_bytes());
        hasher.update(b"\0");
        hasher.update(conteudo);
        hasher.update(b"\0");
    }
    format!("{:x}", hasher.finalize())
}

#[derive(Debug)]
struct ExtracaoResultado {
    json: String,
    from_cache: bool,
    dur: Duration,
}

fn cache_paths(cache_dir: &StdPath, crate_nome: &str) -> (PathBuf, PathBuf) {
    (
        cache_dir.join(format!("{}.json", crate_nome)),
        cache_dir.join(format!("{}.hash", crate_nome)),
    )
}

/// Reusa o cache se hash bater; caso contrário, roda o fork e atualiza.
fn extrair_json_cru(
    info: &CrateInfo,
    cache_dir: &StdPath,
) -> Result<ExtracaoResultado, String> {
    std::fs::create_dir_all(cache_dir).map_err(|e| format!("criar cache_dir: {}", e))?;
    let hash_atual = hash_fontes(&info.diretorio);
    let (path_json, path_hash) = cache_paths(cache_dir, &info.nome);
    if let Ok(hash_cache) = std::fs::read_to_string(&path_hash) {
        if hash_cache.trim() == hash_atual {
            let t0 = Instant::now();
            let json = std::fs::read_to_string(&path_json)
                .map_err(|e| format!("ler cache {}: {}", path_json.display(), e))?;
            return Ok(ExtracaoResultado {
                json,
                from_cache: true,
                dur: t0.elapsed(),
            });
        }
    }
    // Cold ou stale: rodar o fork.
    let t0 = Instant::now();
    let mut cmd = Command::new("cargo");
    cmd.args(["modules", "export-json", "--sysroot", "--compact"]);
    match &info.alvo {
        AlvoFork::Lib => {
            cmd.arg("--lib");
        }
        AlvoFork::Bin(nome) => {
            cmd.args(["--bin", nome]);
        }
    }
    cmd.args(["--package", &info.nome]).current_dir(&info.diretorio);
    let saida = cmd
        .output()
        .map_err(|e| format!("rodar cargo modules: {}", e))?;
    let dur = t0.elapsed();
    if !saida.status.success() {
        return Err(format!(
            "cargo modules export-json falhou: {}",
            String::from_utf8_lossy(&saida.stderr).trim()
        ));
    }
    let json = String::from_utf8(saida.stdout).map_err(|e| format!("utf-8: {}", e))?;
    std::fs::write(&path_json, &json).map_err(|e| format!("escrever cache: {}", e))?;
    std::fs::write(&path_hash, &hash_atual)
        .map_err(|e| format!("escrever hash: {}", e))?;
    Ok(ExtracaoResultado {
        json,
        from_cache: false,
        dur,
    })
}

// ---------------------------------------------------------------------------
// Resolução de colisões por-crate (prompt 0041)
// ---------------------------------------------------------------------------
//
// Replica o laço do `lente_wiring::obter_grafo_resolvido` (laudo 0019):
// para cada path com 2+ nós, investigar (E1) + aplicar veredito. Diferenças:
// 1. Em `NaoDeterminado`, NÃO falha — registra diagnóstico e mantém o blob
//    fundido, marcado como "raio impreciso".
// 2. Devolve um *censo* por crate: vereditos por path, contagens.
//
// Roda **por crate**, ANTES da união. Quando A resolve `A::M::T::fmt` em
// `A::M::T::<Display>::fmt` + `<Debug>::fmt`, a união por path já recebe
// os nomes únicos — não funde mais nada.

#[derive(Debug, Clone, Serialize)]
struct VeredictoPath {
    path: String,
    n_copias: usize,
    veredito: String, // "Distintos" | "MesmoItem" | "NaoDeterminado"
    diagnostico: Option<String>,
    /// Quando `Distintos`, os paths gerados pela resolução (substituições do
    /// path antigo). Permite à UI mostrar o "antes/depois" da nomeação e
    /// distinguir fantasma-de-resolução do fantasma-de-edição.
    novos_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
struct ColisoesResumoCrate {
    total: usize,
    resolvidas_distintos: usize,
    resolvidas_mesmo_item: usize,
    nao_determinadas: usize,
    /// **Distintos com regra ADR-0006 insuficiente**: o investigar decidiu
    /// `Distintos` (vizinhança disjunta), mas a regra de nomeação do
    /// `lente_resolve` produziu paths novos colidentes — porque todas as
    /// cópias compartilham o **mesmo `trait_`** (impls genéricos do mesmo
    /// trait, com `trait_ref` distintos no fork — ex.: `From<ErroFork>`
    /// vs `From<ErroAdaptador>`). Achado da Arena prompt 0041; produto
    /// precisa olhar `trait_ref` nessas situações.
    distintos_mas_colidem_pos_regra: usize,
    /// Detalhe por path colidido. Ordem determinística (alfabética por path).
    detalhes: Vec<VeredictoPath>,
}

/// Devolve os paths colididos (2+ nós) num grafo de crate.
fn detectar_colisoes_grafo(grafo: &Grafo) -> Vec<PathGrafo> {
    let mut contagem: BTreeMap<String, (PathGrafo, usize)> = BTreeMap::new();
    for n in &grafo.nodes {
        let key = n.path.as_str().to_string();
        let e = contagem.entry(key).or_insert_with(|| (n.path.clone(), 0));
        e.1 += 1;
    }
    contagem
        .into_iter()
        .filter(|(_, (_, c))| *c > 1)
        .map(|(_, (p, _))| p)
        .collect()
}

/// Constrói a `Vizinhanca` que o `lente_investiga` espera — entrando/saindo
/// para cada um dos dois ids colidentes. Replicado do `lente_wiring`
/// (laudo 0019). Idêntico em forma; estamos na Arena, não importamos o L4.
fn construir_vizinhanca(grafo: &Grafo, id_a: usize, id_b: usize) -> Vizinhanca {
    let mut va = ArestasNo::default();
    let mut vb = ArestasNo::default();
    for a in &grafo.edges {
        if a.id_to == id_a {
            va.entrando.push(a.clone());
        }
        if a.id_from == id_a {
            va.saindo.push(a.clone());
        }
        if a.id_to == id_b {
            vb.entrando.push(a.clone());
        }
        if a.id_from == id_b {
            vb.saindo.push(a.clone());
        }
    }
    Vizinhanca { a: va, b: vb }
}

/// Resolve um grafo de crate: para cada path colidido, investiga (E1) e
/// aplica o veredito. Em `NaoDeterminado`, **mantém** o blob fundido e
/// registra o diagnóstico no censo.
///
/// Devolve `(grafo_resolvido, censo)`. O censo é exibido no JSON de saída
/// e usado pela UI ("este path tem raio impreciso").
fn resolver_grafo(grafo: Grafo) -> (Grafo, ColisoesResumoCrate) {
    let mut censo = ColisoesResumoCrate::default();
    let mut g = grafo;
    let colisoes = detectar_colisoes_grafo(&g);
    censo.total = colisoes.len();
    for path_col in colisoes {
        // Coletar ids colidentes ATUAIS (uma resolução anterior pode ter
        // mexido na contagem; em geral cada path vem só uma vez aqui).
        let mut ids: Vec<usize> = g
            .nodes
            .iter()
            .filter(|n| n.path == path_col)
            .map(|n| n.id)
            .collect();
        if ids.len() < 2 {
            continue;
        }
        ids.sort_unstable();
        let n_copias = ids.len();
        let (id_a, id_b) = (ids[0], ids[1]);
        let viz = construir_vizinhanca(&g, id_a, id_b);
        let no_a = g.nodes.iter().find(|n| n.id == id_a).expect("id_a");
        let no_b = g.nodes.iter().find(|n| n.id == id_b).expect("id_b");
        let par = ParColidente { a: no_a, b: no_b };
        // E2 em quarentena (laudo 0014) — sempre None.
        let veredito = lente_investiga::investigar(par, &viz, None);

        match &veredito {
            Veredito::Distintos { .. } => {
                let g_novo = match lente_resolve::aplicar(&g, &path_col, &veredito) {
                    Ok(g) => g,
                    Err(e) => {
                        eprintln!(
                            "    !! aplicar Distintos falhou em {}: {} — mantendo blob",
                            path_col.as_str(),
                            e
                        );
                        censo.detalhes.push(VeredictoPath {
                            path: path_col.as_str().to_string(),
                            n_copias,
                            veredito: "NaoDeterminado".to_string(),
                            diagnostico: Some(format!("aplicar Distintos falhou: {}", e)),
                            novos_paths: vec![],
                        });
                        censo.nao_determinadas += 1;
                        continue;
                    }
                };
                // Achar os novos paths para o(s) id(s) colidentes — devem ser
                // diferentes do path original.
                let novos: Vec<String> = g_novo
                    .nodes
                    .iter()
                    .filter(|n| ids.contains(&n.id))
                    .map(|n| n.path.as_str().to_string())
                    .filter(|p| p != path_col.as_str())
                    .collect();
                // Achado da Arena: a regra ADR-0006 usa `trait_` (não
                // `trait_ref`). Se as cópias compartilham o mesmo trait
                // (impls genéricos), os "novos paths" se repetem → a
                // colisão **permanece**. Marca como impreciso.
                let mut paths_unicos: Vec<&String> = novos.iter().collect();
                paths_unicos.sort();
                let total_novos = paths_unicos.len();
                paths_unicos.dedup();
                let realmente_distintos = paths_unicos.len() == total_novos && total_novos > 0;
                if !realmente_distintos {
                    censo.distintos_mas_colidem_pos_regra += 1;
                    censo.detalhes.push(VeredictoPath {
                        path: path_col.as_str().to_string(),
                        n_copias,
                        veredito: "DistintosPosRegraColide".to_string(),
                        diagnostico: Some(format!(
                            "lente_resolve nomeou {} cópias mas só produziu {} paths únicos — \
                             cópias compartilham `trait` (e.g. impls de `From<T>`); regra \
                             ADR-0006 precisaria usar `trait_ref` para distinguir",
                            n_copias,
                            paths_unicos.len()
                        )),
                        novos_paths: novos,
                    });
                    // Aplica mesmo assim (paths colidem entre si — a união
                    // vai fundir; sinaliza a UI via paths_imprecisos).
                    g = g_novo;
                    continue;
                }
                censo.detalhes.push(VeredictoPath {
                    path: path_col.as_str().to_string(),
                    n_copias,
                    veredito: "Distintos".to_string(),
                    diagnostico: None,
                    novos_paths: novos,
                });
                censo.resolvidas_distintos += 1;
                g = g_novo;
            }
            Veredito::MesmoItem => {
                let g_novo = match lente_resolve::aplicar(&g, &path_col, &veredito) {
                    Ok(g) => g,
                    Err(e) => {
                        eprintln!(
                            "    !! aplicar MesmoItem falhou em {}: {}",
                            path_col.as_str(),
                            e
                        );
                        continue;
                    }
                };
                censo.detalhes.push(VeredictoPath {
                    path: path_col.as_str().to_string(),
                    n_copias,
                    veredito: "MesmoItem".to_string(),
                    diagnostico: None,
                    novos_paths: vec![],
                });
                censo.resolvidas_mesmo_item += 1;
                g = g_novo;
            }
            Veredito::NaoDeterminado { diagnostico } => {
                // Mantém o blob fundido. A união por path vai fundir os
                // colididos; o raio do path colidido fica IMPRECISO. A UI
                // mostra essa marca (consumidor: campo `raio_impreciso`
                // do NoTocado).
                censo.detalhes.push(VeredictoPath {
                    path: path_col.as_str().to_string(),
                    n_copias,
                    veredito: "NaoDeterminado".to_string(),
                    diagnostico: Some(diagnostico.clone()),
                    novos_paths: vec![],
                });
                censo.nao_determinadas += 1;
            }
        }
    }
    censo.detalhes.sort_by(|a, b| a.path.cmp(&b.path));
    (g, censo)
}

// ---------------------------------------------------------------------------
// Parser do `git diff`
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, Hash)]
struct Faixa {
    inicio: u32,
    fim: u32,
}

#[derive(Debug, Clone, Serialize)]
struct ArquivoDiff {
    caminho: String,
    faixas: Vec<Faixa>,
}

fn parse_diff(texto: &str) -> Vec<ArquivoDiff> {
    let mut por_arquivo: BTreeMap<String, Vec<Faixa>> = BTreeMap::new();
    let mut arquivo_corrente: Option<String> = None;
    for linha in texto.lines() {
        if let Some(resto) = linha.strip_prefix("+++ ") {
            let resto = resto.trim();
            if resto == "/dev/null" {
                arquivo_corrente = None;
                continue;
            }
            let cam = resto.strip_prefix("b/").unwrap_or(resto).to_string();
            arquivo_corrente = Some(cam);
        } else if linha.starts_with("@@") {
            let Some(arquivo) = arquivo_corrente.as_ref() else {
                continue;
            };
            let Some(parte_mais) = linha.split('+').nth(1) else {
                continue;
            };
            let parte_mais = parte_mais.split(' ').next().unwrap_or("");
            let (c, d) = match parte_mais.split_once(',') {
                Some((c, d)) => (
                    c.parse::<u32>().unwrap_or(0),
                    d.parse::<u32>().unwrap_or(1),
                ),
                None => (parte_mais.parse::<u32>().unwrap_or(0), 1),
            };
            if c == 0 || d == 0 {
                continue;
            }
            por_arquivo.entry(arquivo.clone()).or_default().push(Faixa {
                inicio: c,
                fim: c + d - 1,
            });
        }
    }
    por_arquivo
        .into_iter()
        .map(|(caminho, faixas)| ArquivoDiff { caminho, faixas })
        .collect()
}

fn ler_diff_stdin() -> String {
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    s
}

fn ler_diff_git(repo: &StdPath) -> String {
    let out = Command::new("git")
        .args(["diff", "HEAD", "--unified=0", "--no-color"])
        .current_dir(repo)
        .output()
        .expect("git diff falhou");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

// ---------------------------------------------------------------------------
// Untracked: arquivos novos não rastreados (prompt 0043)
// ---------------------------------------------------------------------------
//
// O ponto cego dos laudos 0038/0040: `git diff HEAD` NÃO mostra arquivo novo
// sem `git add`. `git ls-files --others --exclude-standard` enxerga os
// untracked no nível de filesystem/git — tanto os LIGADOS (com `mod`, que o
// cargo compila → viram nós no grafo) quanto os SOLTOS (sem `mod`, que o
// cargo ignora → não viram nós). O corte que decide tudo: cruzar a lista do
// git com o conjunto de fontes que o cargo de fato compilou (= os `file` das
// `position` dos nós do grafo unido).

/// Lista os arquivos não rastreados (respeitando `.gitignore`), relativos à
/// raiz do repo.
fn ler_untracked(repo: &StdPath) -> Vec<String> {
    let out = Command::new("git")
        .args(["ls-files", "--others", "--exclude-standard"])
        .current_dir(repo)
        .output()
        .expect("git ls-files falhou");
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Conjunto de fontes que o cargo DE FATO compilou: os caminhos relativos das
/// `position` de todos os nós do grafo. É a verdade-de-campo de "ligado": se
/// um `.rs` novo aparece aqui, o cargo o compilou (logo, tem `mod`).
fn fontes_compiladas(grafo: &Grafo, raiz: &StdPath) -> BTreeSet<String> {
    grafo
        .nodes
        .iter()
        .filter_map(|n| n.position.as_ref())
        .filter_map(|p| relativizar(&p.file, raiz))
        .collect()
}

/// Conta as linhas de um arquivo (para sintetizar o hunk "tudo adicionado").
fn contar_linhas(repo: &StdPath, rel: &str) -> u32 {
    std::fs::read_to_string(repo.join(rel))
        .map(|s| s.lines().count() as u32)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// União de grafos por path (abordagem B do prompt 0039)
// ---------------------------------------------------------------------------

/// Une vários `Grafo` num único, casando nós por **path**. IDs do petgraph
/// são instáveis entre extrações; o path é a única identidade estável
/// entre extrações de crates diferentes (briefing §7).
///
/// Política de fusão:
/// - Nós: para cada path único, mantém **uma** entrada. Prefere a versão
///   que tem `Some(position)` (ou seja: a versão produzida pela extração
///   do **próprio crate** do nó, vs uma referência leve produzida por
///   outro crate).
/// - Arestas: reatribuídas com IDs globais via `path → id_global`. Se uma
///   aresta referencia um path que não existe na união (raro: ex.: stdlib
///   referenciada mas não extraída), é **descartada** e contada como
///   "solta".
///
/// Devolve: (grafo unido, n_arestas_soltas).
fn unir_grafos(grafos: Vec<Grafo>) -> (Grafo, usize) {
    unir_grafos_com_origens(grafos.into_iter().map(|g| ("?".to_string(), g)).collect()).0
}

/// Wrapper que devolve, junto, o mapa de **origens** por path. Origens =
/// conjunto de crates cujos caches produziram o path. Usado para detectar
/// **nós fantasma** (path cujo primeiro segmento é um crate do workspace
/// mas que NÃO está entre as origens — sinal de renomeação/remoção stale).
fn unir_grafos_com_origens(
    grafos_com_nome: Vec<(String, Grafo)>,
) -> ((Grafo, usize), BTreeMap<String, BTreeSet<String>>) {
    // Mapa path → conjunto de crates que produziram o nó.
    let mut origens: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let grafos: Vec<Grafo> = grafos_com_nome
        .iter()
        .map(|(_, g)| g.clone())
        .collect();
    for (nome_crate, g) in &grafos_com_nome {
        for n in &g.nodes {
            origens
                .entry(n.path.as_str().to_string())
                .or_default()
                .insert(nome_crate.clone());
        }
    }
    // 1. Coletar nós únicos por path (preferindo os que têm position).
    let mut melhor_no_por_path: BTreeMap<String, No> = BTreeMap::new();
    for g in &grafos {
        for n in &g.nodes {
            let key = n.path.as_str().to_string();
            let prefere_novo = match melhor_no_por_path.get(&key) {
                None => true,
                Some(antigo) => antigo.position.is_none() && n.position.is_some(),
            };
            if prefere_novo {
                melhor_no_por_path.insert(key, n.clone());
            }
        }
    }
    // 2. Reatribuir IDs globais sequenciais. Path → id_global.
    let mut id_por_path: BTreeMap<String, usize> = BTreeMap::new();
    let mut nos_unidos: Vec<No> = Vec::with_capacity(melhor_no_por_path.len());
    for (i, (path, mut no)) in melhor_no_por_path.into_iter().enumerate() {
        id_por_path.insert(path, i);
        no.id = i;
        nos_unidos.push(no);
    }
    // 3. Arestas: reanchorar via path. Cada extração já tem `from`/`to`
    //    como Path; reusa.
    let mut arestas_unidas: Vec<Aresta> = Vec::new();
    let mut soltas: usize = 0;
    // `Relation` e `UsesKind` derivam `Hash` mas não `Ord` — usar HashSet.
    // O uses_kind vira `u8` (None=0, Reference=1, Import=2) para chave
    // simples.
    let mut vistas: HashSet<(usize, usize, u8, u8)> = HashSet::new();
    for g in &grafos {
        for a in &g.edges {
            let from_key = a.from.as_str();
            let to_key = a.to.as_str();
            let (Some(&id_from), Some(&id_to)) =
                (id_por_path.get(from_key), id_por_path.get(to_key))
            else {
                soltas += 1;
                continue;
            };
            let r_byte: u8 = match a.relation {
                Relation::Owns => 1,
                Relation::Uses => 2,
            };
            let uk_byte: u8 = match a.uses_kind {
                None => 0,
                Some(UsesKind::Reference) => 1,
                Some(UsesKind::Import) => 2,
            };
            let chave = (id_from, id_to, r_byte, uk_byte);
            if !vistas.insert(chave) {
                continue;
            }
            arestas_unidas.push(Aresta {
                from: a.from.clone(),
                id_from,
                to: a.to.clone(),
                id_to,
                relation: a.relation,
                uses_kind: a.uses_kind,
            });
        }
    }
    let crate_name = grafos
        .first()
        .map(|g| g.crate_name.clone())
        .unwrap_or_default();
    (
        (
            Grafo {
                crate_name,
                nodes: nos_unidos,
                edges: arestas_unidas,
            },
            soltas,
        ),
        origens,
    )
}

// ---------------------------------------------------------------------------
// Tipos de saída
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
struct ResumoExtracao {
    nodes: usize,
    com_position: usize,
    edges: usize,
    colisoes: usize,
    /// Quantos nós são "leves" (referência a outro crate, sem `position` e
    /// sem campos do descritor). Indicador da abordagem A do prompt.
    nos_leves_referencias: usize,
    tempo_seg: f64,
}

#[derive(Debug, Clone, Serialize)]
struct RaioResumo {
    classificacao: &'static str,
    /// Grau de entrada por `Uses` (quem depende direto — montante direto).
    diretos: usize,
    /// Tamanho do montante transitivo (quem sente, transitivamente).
    transitivos: usize,
    amostra_montante: Vec<String>,
    /// Prompt 0043: grau de saída por `Uses` (do que o nó depende — jusante
    /// direto). Para arquivo NOVO, é aqui que está o valor: o montante é
    /// quase vazio (ninguém o usa ainda), mas o jusante mostra o que ele
    /// passou a usar.
    diretos_saida: usize,
    /// Tamanho do jusante transitivo (do que depende, transitivamente).
    transitivos_jusante: usize,
    amostra_jusante: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct NoTocado {
    id: usize,
    path: String,
    kind: String,
    cadeia: Vec<String>,
    file_relativo: String,
    start_line: u32,
    end_line: u32,
    raio_local: RaioResumo,
    raio_workspace: RaioResumo,
    /// `transitivos` no workspace − `transitivos` no local. > 0 ⇒ há
    /// dependentes em outros crates.
    delta_cross_crate: i64,
    /// Prompt 0041: este nó vive sob um path que ficou `NaoDeterminado` na
    /// investigação — o raio mostrado é do *blob fundido*, não da cópia
    /// individual. A UI marca como "raio impreciso aqui".
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    raio_impreciso: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ImpactoArquivo {
    arquivo: String,
    faixas: Vec<Faixa>,
    nos_tocados: Vec<NoTocado>,
}

#[derive(Debug, Clone, Serialize)]
struct ImpactoCrate {
    /// Nome do crate que dona o arquivo.
    crate_nome: String,
    arquivos: Vec<ImpactoArquivo>,
}

#[derive(Debug, Clone, Serialize)]
struct ModoSaida {
    fonte: &'static str,
    arquivos_tocados: Vec<ArquivoDiff>,
    impacto_por_crate: Vec<ImpactoCrate>,
    /// Arquivos do diff que não casam com nenhum crate do workspace
    /// (ex.: README, docs, scripts).
    arquivos_sem_crate: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ComparacaoCaminhos {
    iguais: bool,
    so_em_stdin: Vec<String>,
    so_em_git: Vec<String>,
}

// --- Untracked (prompt 0043) ----------------------------------------------

/// Arquivo novo LIGADO (com `mod`): o cargo compila, o grafo tem seus nós.
/// O `impacto` traz o mapeamento por `position` dos hunks sintetizados
/// ("tudo adicionado"), com montante (esperado ~vazio) e jusante (o valor).
#[derive(Debug, Clone, Serialize)]
struct UntrackedLigado {
    arquivo: String,
    crate_nome: String,
    /// Linhas do arquivo (extensão do hunk sintetizado 1..=linhas).
    linhas: u32,
    impacto: ImpactoArquivo,
}

/// Arquivo novo SOLTO (sem `mod`): o cargo ignora, o grafo não tem seus nós.
/// Sinal acionável, não omissão silenciosa nem panic.
#[derive(Debug, Clone, Serialize)]
struct UntrackedSolto {
    arquivo: String,
    crate_nome: String,
    sinal: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct UntrackedResumo {
    comando: &'static str,
    /// Total de untracked detectados pelo git (nível filesystem).
    total_detectados: usize,
    /// `.rs` que caem dentro de um crate do workspace (os candidatos).
    rs_em_crate: Vec<String>,
    /// Ligados: `.rs` em crate que aparecem nas fontes compiladas.
    ligados: Vec<UntrackedLigado>,
    /// Soltos: `.rs` em crate ausentes das fontes compiladas.
    soltos: Vec<UntrackedSolto>,
    /// Não-fonte: untracked que não é `.rs` em crate (docs, lab, dados).
    /// Só contagem + amostra para não inflar o JSON.
    nao_fonte_total: usize,
    nao_fonte_amostra: Vec<String>,
    /// Enumeração que a chave de cache usa hoje (prompt 0043 §D).
    enumeracao_cache: &'static str,
    nota: &'static str,
}

#[derive(Debug, Serialize)]
struct Saida {
    raiz_repo: String,
    crates_workspace: Vec<String>,
    crates_tocados: Vec<String>,
    extracoes: BTreeMap<String, ResumoExtracao>,
    uniao: UniaoResumo,
    cache: CacheResumo,
    cronometria: Cronometria,
    /// Prompt 0041: censo de colisões por crate + agregado do workspace.
    colisoes: ColisoesResumo,
    stdin: Option<ModoSaida>,
    git: Option<ModoSaida>,
    /// Prompt 0043: censo dos arquivos não rastreados (ligados vs soltos).
    untracked: UntrackedResumo,
    comparacao: Option<ComparacaoCaminhos>,
    /// Prompt 0041 §4: antes/depois do raio para um path colidido tocado
    /// sinteticamente (`--simular-tocar-colidido <path>`).
    antes_depois: Option<AntesDepoisColisao>,
    nota_honestidade: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct ColisoesResumo {
    /// Política aplicada: "resolver" (default) ou "sem-resolucao" (flag).
    politica: &'static str,
    /// Total no workspace, antes da resolução.
    total: usize,
    resolvidas_distintos: usize,
    resolvidas_mesmo_item: usize,
    nao_determinadas: usize,
    /// Achado da Arena 0041: a regra de nomeação ADR-0006 do `lente_resolve`
    /// usa `trait_`, mas múltiplos `impl <Trait> for T` genéricos (ex.:
    /// `From<A> for T`, `From<B> for T`) compartilham `trait_` — paths
    /// novos colidem entre si. Total agregado no workspace.
    distintos_mas_colidem_pos_regra: usize,
    por_crate: BTreeMap<String, ColisoesResumoCrate>,
    /// Tempo total da etapa de resolução (soma por crate).
    tempo_seg: f64,
}

#[derive(Debug, Clone, Serialize)]
struct AntesDepoisColisao {
    /// O path colidido que foi avaliado.
    path_colidido: String,
    /// Crate dono (primeiro segmento do path).
    crate_nome: String,
    /// Raio do path colidido na união CRU-FUNDIDA (sem resolução). É o
    /// número que a vista do 0039/0040 mostraria — pode estar errado.
    raio_cru_fundido: Option<RaioResumo>,
    /// Raios resolvidos: 1 entrada por cópia distinta (típico: 2 para
    /// `<Display>::fmt` + `<Debug>::fmt`).
    raios_resolvidos: Vec<RaioPorCopia>,
    /// Soma dos `transitivos` das cópias resolvidas. Comparar com
    /// `raio_cru_fundido.transitivos` para quantificar o erro.
    soma_transitivos_resolvidos: usize,
    /// Diferença entre a soma resolvida e o cru-fundido (positivo =
    /// cru-fundido subreporta; negativo = cru-fundido sobrereporta).
    delta: i64,
}

#[derive(Debug, Clone, Serialize)]
struct RaioPorCopia {
    /// Novo path da cópia (ex.: `M::T::<Display>::fmt`).
    path: String,
    raio: RaioResumo,
}

#[derive(Debug, Serialize)]
struct UniaoResumo {
    nodes_total: usize,
    edges_total: usize,
    arestas_soltas: usize,
    tempo_seg: f64,
    crates_unidos: Vec<String>,
    /// Nós cuja "casa" (crate cujo nome bate o primeiro segmento do path)
    /// **não** está entre as origens — sinal de cache stale, renomeação
    /// ou remoção (prompt 0040 §4).
    nos_fantasma: Vec<NoFantasma>,
}

#[derive(Debug, Serialize)]
struct NoFantasma {
    path: String,
    crate_esperado: String,
    origens: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CacheResumo {
    diretorio: String,
    extracoes_realizadas: Vec<String>,
    extracoes_reusadas: Vec<String>,
    tempo_extracao_fork_seg: f64,
    tempo_cache_io_seg: f64,
    /// Saída do cenário: cold = todos extraídos; quente = todos reusados; etc.
    cenario: String,
}

#[derive(Debug, Serialize)]
struct Cronometria {
    /// Tempo total do `main()` da extração até a serialização do JSON.
    total_seg: f64,
    /// `cargo metadata --no-deps`.
    metadata_seg: f64,
    /// Soma dos `cargo modules export-json` (cold ou stale).
    extracoes_fork_seg: f64,
    /// Soma dos `read_to_string` do cache.
    cache_reuso_seg: f64,
    /// Desserialização + união (parte sem-fork).
    desserializar_unir_seg: f64,
    /// Prompt 0041: tempo da etapa de resolução por-crate (entre
    /// desserializar e unir). Hipótese do prompt: desprezível.
    resolucao_seg: f64,
    /// Mapeamento diff→nós + cálculo de raio.
    mapeamento_raio_seg: f64,
}

// ---------------------------------------------------------------------------
// Mapeamento diff → nós, cadeia, raio
// ---------------------------------------------------------------------------

fn classificacao_texto(c: Classificacao) -> &'static str {
    match c {
        Classificacao::Isolado => "Isolado",
        Classificacao::Folha => "Folha",
        Classificacao::Base => "Base",
        Classificacao::Intermediario => "Intermediário",
    }
}

fn raio_resumo(r: &Raio) -> RaioResumo {
    let mut amostra: Vec<String> = r.montante.keys().map(|p| p.as_str().to_string()).collect();
    amostra.sort();
    amostra.truncate(10);
    let mut amostra_j: Vec<String> = r.jusante.keys().map(|p| p.as_str().to_string()).collect();
    amostra_j.sort();
    amostra_j.truncate(10);
    RaioResumo {
        classificacao: classificacao_texto(r.classificacao),
        diretos: r.uses_entrada,
        transitivos: r.montante.len(),
        amostra_montante: amostra,
        diretos_saida: r.uses_saida,
        transitivos_jusante: r.jusante.len(),
        amostra_jusante: amostra_j,
    }
}

fn raio_vazio() -> RaioResumo {
    RaioResumo {
        classificacao: "?",
        diretos: 0,
        transitivos: 0,
        amostra_montante: vec![],
        diretos_saida: 0,
        transitivos_jusante: 0,
        amostra_jusante: vec![],
    }
}

fn mapa_owns_pai(grafo: &Grafo) -> BTreeMap<usize, usize> {
    grafo
        .edges
        .iter()
        .filter(|a| a.relation == Relation::Owns)
        .map(|a| (a.id_to, a.id_from))
        .collect()
}

fn cadeia_de(no_id: usize, grafo: &Grafo, pai: &BTreeMap<usize, usize>) -> Vec<String> {
    let path_por_id: BTreeMap<usize, String> = grafo
        .nodes
        .iter()
        .map(|n| (n.id, n.path.as_str().to_string()))
        .collect();
    let mut ids: Vec<usize> = Vec::new();
    let mut atual = no_id;
    let mut visitados: BTreeSet<usize> = BTreeSet::new();
    loop {
        if !visitados.insert(atual) {
            break;
        }
        ids.push(atual);
        match pai.get(&atual) {
            Some(&p) => atual = p,
            None => break,
        }
    }
    ids.reverse();
    ids.into_iter()
        .filter_map(|id| path_por_id.get(&id).cloned())
        .collect()
}

fn relativizar(absoluto: &str, raiz: &StdPath) -> Option<String> {
    let raiz_s = raiz.to_string_lossy();
    let r = raiz_s.trim_end_matches('/');
    if absoluto.starts_with(r) {
        Some(absoluto[r.len()..].trim_start_matches('/').to_string())
    } else {
        None
    }
}

fn intersecta(a: u32, b: u32, c: u32, d: u32) -> bool {
    a <= d && c <= b
}

/// Retorna nós com `position` cujo arquivo relativo bate o `arq_rel`.
fn nos_no_arquivo<'a>(grafo: &'a Grafo, arq_rel: &str, raiz: &StdPath) -> Vec<(&'a No, &'a Posicao)> {
    grafo
        .nodes
        .iter()
        .filter_map(|n| n.position.as_ref().map(|p| (n, p)))
        .filter(|(_, p)| {
            relativizar(&p.file, raiz)
                .map(|r| r == arq_rel)
                .unwrap_or(false)
                || p.file.ends_with(arq_rel)
        })
        .collect()
}

fn mapear_arquivo(
    grafo_local: &Grafo,
    grafo_workspace: &Grafo,
    arq: &ArquivoDiff,
    raiz_repo: &StdPath,
    paths_imprecisos: &BTreeSet<String>,
) -> ImpactoArquivo {
    let pai_local = mapa_owns_pai(grafo_local);
    let nos = nos_no_arquivo(grafo_local, &arq.caminho, raiz_repo);
    let mut nos_tocados: Vec<NoTocado> = Vec::new();
    let mut vistos: BTreeSet<usize> = BTreeSet::new();
    for f in &arq.faixas {
        for (no, pos) in &nos {
            if !intersecta(pos.start_line, pos.end_line, f.inicio, f.fim) {
                continue;
            }
            if !vistos.insert(no.id) {
                continue;
            }
            let cadeia = cadeia_de(no.id, grafo_local, &pai_local);
            let raio_local = calcular_raio(grafo_local, &no.path)
                .map(|r| raio_resumo(&r))
                .unwrap_or_else(|_| raio_vazio());
            let raio_workspace = calcular_raio(grafo_workspace, &no.path)
                .map(|r| raio_resumo(&r))
                .unwrap_or_else(|_| raio_vazio());
            let delta_cross_crate =
                raio_workspace.transitivos as i64 - raio_local.transitivos as i64;
            let raio_impreciso = paths_imprecisos.contains(no.path.as_str());
            nos_tocados.push(NoTocado {
                id: no.id,
                path: no.path.as_str().to_string(),
                kind: format!("{:?}", no.kind),
                cadeia,
                file_relativo: relativizar(&pos.file, raiz_repo)
                    .unwrap_or_else(|| pos.file.clone()),
                start_line: pos.start_line,
                end_line: pos.end_line,
                raio_local,
                raio_workspace,
                delta_cross_crate,
                raio_impreciso,
            });
        }
    }
    nos_tocados.sort_by(|a, b| {
        a.start_line
            .cmp(&b.start_line)
            .then_with(|| a.path.cmp(&b.path))
    });
    ImpactoArquivo {
        arquivo: arq.caminho.clone(),
        faixas: arq.faixas.clone(),
        nos_tocados,
    }
}

fn diferencas(a: &[ArquivoDiff], b: &[ArquivoDiff]) -> ComparacaoCaminhos {
    let sa: BTreeSet<&str> = a.iter().map(|x| x.caminho.as_str()).collect();
    let sb: BTreeSet<&str> = b.iter().map(|x| x.caminho.as_str()).collect();
    let so_a: Vec<String> = sa.difference(&sb).map(|s| s.to_string()).collect();
    let so_b: Vec<String> = sb.difference(&sa).map(|s| s.to_string()).collect();
    ComparacaoCaminhos {
        iguais: so_a.is_empty() && so_b.is_empty(),
        so_em_stdin: so_a,
        so_em_git: so_b,
    }
}

fn colisoes_path(grafo: &Grafo) -> usize {
    let mut por_path: BTreeMap<&str, usize> = BTreeMap::new();
    for n in &grafo.nodes {
        *por_path.entry(n.path.as_str()).or_default() += 1;
    }
    por_path.values().filter(|c| **c > 1).count()
}

/// Conta nós-leves: sem `position` E sem campos de descritor preenchidos.
/// É a marca dos nós-referência do `cargo modules` (ver achado da
/// abordagem A no laudo 0038/0039).
fn nos_leves(grafo: &Grafo) -> usize {
    grafo
        .nodes
        .iter()
        .filter(|n| {
            n.position.is_none()
                && n.trait_.is_none()
                && n.trait_ref.is_none()
                && n.cfg.is_none()
                && n.macro_kind.is_none()
                && n.modificadores == Modificadores::default()
                && !n.is_non_exhaustive
                && n.visibility == Visibility::Pub
        })
        .count()
}

fn main() {
    let t_main = Instant::now();
    let args = parse_args();
    eprintln!(
        "lab/proto-impacto-diff (cache) — repo={} input={:?} cache_dir={} \
         limpar={} invalidar={:?} renomeacao={:?}",
        args.repo.display(),
        args.input,
        args.cache_dir.display(),
        args.limpar_cache,
        args.invalidar,
        args.simular_renomeacao,
    );
    let raiz = std::fs::canonicalize(&args.repo).unwrap_or(args.repo.clone());

    // 0. Limpeza do cache (cold simulado) ou invalidação seletiva (morno-N).
    if args.limpar_cache && args.cache_dir.exists() {
        std::fs::remove_dir_all(&args.cache_dir).ok();
    }
    for crate_nome in &args.invalidar {
        let (j, h) = cache_paths(&args.cache_dir, crate_nome);
        let _ = std::fs::remove_file(&j);
        let _ = std::fs::remove_file(&h);
    }

    // 1. `cargo metadata --no-deps`.
    let t_meta = Instant::now();
    let crates = descobrir_workspace(&raiz);
    let dt_meta = t_meta.elapsed();
    let nomes_workspace: Vec<String> = crates.iter().map(|c| c.nome.clone()).collect();
    eprintln!("  metadata: {} crates em {:.2}s", crates.len(), dt_meta.as_secs_f64());

    // 2. Ler diff (um ou ambos).
    let (texto_stdin, texto_git) = match args.input {
        ModoInput::Stdin => (Some(ler_diff_stdin()), None),
        ModoInput::Git => (None, Some(ler_diff_git(&raiz))),
        ModoInput::Ambos => (Some(ler_diff_stdin()), Some(ler_diff_git(&raiz))),
    };
    let stdin_arqs: Option<Vec<ArquivoDiff>> = texto_stdin.as_ref().map(|s| parse_diff(s));
    let git_arqs: Option<Vec<ArquivoDiff>> = texto_git.as_ref().map(|s| parse_diff(s));

    // 3. Identificar crates tocados pelo diff.
    let mut tocados: BTreeSet<String> = BTreeSet::new();
    for arqs in [stdin_arqs.as_ref(), git_arqs.as_ref()]
        .into_iter()
        .flatten()
    {
        for a in arqs.iter() {
            if let Some(c) = crate_de_arquivo(&a.caminho, &crates) {
                tocados.insert(c.nome.clone());
            }
        }
    }
    let crates_tocados: Vec<String> = tocados.iter().cloned().collect();
    eprintln!("  crates tocados pelo diff: {:?}", crates_tocados);

    // 4. Conjunto a extrair: tocados (`--so-tocados`) ou workspace inteiro.
    let conjunto: Vec<&CrateInfo> = if args.so_tocados {
        crates.iter().filter(|c| tocados.contains(&c.nome)).collect()
    } else {
        crates.iter().collect()
    };

    // 5. Extrair cada crate **com cache** (prompt 0040).
    let mut grafos_por_crate: BTreeMap<String, Grafo> = BTreeMap::new();
    let mut extracoes: BTreeMap<String, ResumoExtracao> = BTreeMap::new();
    let mut realizadas: Vec<String> = Vec::new();
    let mut reusadas: Vec<String> = Vec::new();
    let mut tempo_fork = Duration::ZERO;
    let mut tempo_cache_io = Duration::ZERO;
    let mut tempo_desserializar = Duration::ZERO;
    for c in &conjunto {
        let res_ext = match extrair_json_cru(c, &args.cache_dir) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("  ! falha em {}: {}", c.nome, e);
                continue;
            }
        };

        // Aplicar simulação de renomeação (prompt 0040 §4): só o crate
        // DONO do path renomeia velho→novo (no JSON em memória, sem
        // tocar o cache); os demais mantêm o velho. Crate dono = primeiro
        // segmento do path velho.
        let json = if let Some((velho, novo)) = &args.simular_renomeacao {
            let dono = velho.split("::").next().unwrap_or("");
            if c.nome == dono {
                res_ext.json.replace(velho.as_str(), novo.as_str())
            } else {
                res_ext.json.clone()
            }
        } else {
            res_ext.json.clone()
        };

        if res_ext.from_cache {
            tempo_cache_io += res_ext.dur;
            reusadas.push(c.nome.clone());
        } else {
            tempo_fork += res_ext.dur;
            realizadas.push(c.nome.clone());
        }

        let t_des = Instant::now();
        let g = match lente_infra::desserializar_grafo(&json) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("  ! falha ao desserializar {}: {}", c.nome, e);
                continue;
            }
        };
        tempo_desserializar += t_des.elapsed();
        let com_position = g.nodes.iter().filter(|n| n.position.is_some()).count();
        let leves = nos_leves(&g);
        let res = ResumoExtracao {
            nodes: g.nodes.len(),
            com_position,
            edges: g.edges.len(),
            colisoes: colisoes_path(&g),
            nos_leves_referencias: leves,
            tempo_seg: (res_ext.dur.as_secs_f64() * 100.0).round() / 100.0,
        };
        eprintln!(
            "  {}: {} nodes, {} edges, colisões={} ({}, {:.2}s)",
            c.nome,
            res.nodes,
            res.edges,
            res.colisoes,
            if res_ext.from_cache { "cache" } else { "fork" },
            res.tempo_seg
        );
        extracoes.insert(c.nome.clone(), res);
        grafos_por_crate.insert(c.nome.clone(), g);
    }

    // 5.5 Resolver colisões POR CRATE, antes de unir (prompt 0041). A união
    // por path funde colisões intra-crate em silêncio; resolver antes faz a
    // união encontrar paths únicos. Em NaoDeterminado, mantém o blob fundido
    // e o marca para a UI ("raio impreciso").
    //
    // **Pré-resolução** (cópia dos grafos sem resolução, para o
    // antes/depois e para o modo `--sem-resolucao`). Reusa a mesma fonte
    // (cache/JSON) — não toca o fork de novo.
    let grafos_por_crate_cru: BTreeMap<String, Grafo> = grafos_por_crate.clone();

    let t_res = Instant::now();
    let mut colisoes_por_crate: BTreeMap<String, ColisoesResumoCrate> = BTreeMap::new();
    let mut paths_imprecisos: BTreeSet<String> = BTreeSet::new();
    if !args.sem_resolucao {
        let mut grafos_resolvidos: BTreeMap<String, Grafo> = BTreeMap::new();
        for (nome, g) in grafos_por_crate.into_iter() {
            let (g_res, censo) = resolver_grafo(g);
            if censo.total > 0 {
                eprintln!(
                    "    resolução {}: total={} distintos={} mesmo_item={} nao_det={}",
                    nome,
                    censo.total,
                    censo.resolvidas_distintos,
                    censo.resolvidas_mesmo_item,
                    censo.nao_determinadas,
                );
            }
            for det in &censo.detalhes {
                if det.veredito == "NaoDeterminado" || det.veredito == "DistintosPosRegraColide" {
                    paths_imprecisos.insert(det.path.clone());
                    // No caso `DistintosPosRegraColide`, o path ATUAL no
                    // grafo é `novos_paths[0]` (repetido) — também é
                    // impreciso.
                    for np in &det.novos_paths {
                        paths_imprecisos.insert(np.clone());
                    }
                }
            }
            colisoes_por_crate.insert(nome.clone(), censo);
            grafos_resolvidos.insert(nome, g_res);
        }
        grafos_por_crate = grafos_resolvidos;
    } else {
        // Modo cru: ainda computa o censo para reporte, mas NÃO aplica.
        for (nome, g) in grafos_por_crate.iter() {
            let (_, censo) = resolver_grafo(g.clone());
            colisoes_por_crate.insert(nome.clone(), censo);
        }
    }
    let dt_res = t_res.elapsed();

    let mut agg_total = 0usize;
    let mut agg_dist = 0usize;
    let mut agg_msm = 0usize;
    let mut agg_nd = 0usize;
    let mut agg_dprc = 0usize;
    for c in colisoes_por_crate.values() {
        agg_total += c.total;
        agg_dist += c.resolvidas_distintos;
        agg_msm += c.resolvidas_mesmo_item;
        agg_nd += c.nao_determinadas;
        agg_dprc += c.distintos_mas_colidem_pos_regra;
    }
    eprintln!(
        "  colisões: total={} distintos={} mesmo_item={} nao_det={} \
         dist_pos_regra_colide={} (política={} {:.2}ms)",
        agg_total,
        agg_dist,
        agg_msm,
        agg_nd,
        agg_dprc,
        if args.sem_resolucao { "sem-resolucao" } else { "resolver" },
        dt_res.as_secs_f64() * 1000.0,
    );

    // 6. União dos grafos por path. A versão `com_origens` rastreia, por
    // path, quais crates o produziram — necessário para detectar nós
    // fantasma (prompt 0040 §4: o sinal real de renomeação no monorepo).
    let t_uniao = Instant::now();
    let grafos_com_nome: Vec<(String, Grafo)> = grafos_por_crate
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let ((grafo_workspace, soltas), origens_por_path) =
        unir_grafos_com_origens(grafos_com_nome);
    let dt_uniao = t_uniao.elapsed();

    // Detectar nós fantasma: path cujo primeiro segmento é nome de crate
    // do workspace, mas que NÃO está entre as origens (= nenhum cache do
    // crate-dono produziu o nó). Sinal de renomeação/remoção stale.
    let workspace_set: BTreeSet<String> = nomes_workspace.iter().cloned().collect();
    let mut fantasmas: Vec<NoFantasma> = Vec::new();
    for n in &grafo_workspace.nodes {
        let path = n.path.as_str();
        let primeiro = path.split("::").next().unwrap_or("");
        if !workspace_set.contains(primeiro) {
            continue; // crate externo (stdlib, dep) — não é o caso
        }
        let origens = origens_por_path
            .get(path)
            .cloned()
            .unwrap_or_default();
        if !origens.contains(primeiro) {
            fantasmas.push(NoFantasma {
                path: path.to_string(),
                crate_esperado: primeiro.to_string(),
                origens: origens.into_iter().collect(),
            });
        }
    }
    let uniao = UniaoResumo {
        nodes_total: grafo_workspace.nodes.len(),
        edges_total: grafo_workspace.edges.len(),
        arestas_soltas: soltas,
        tempo_seg: (dt_uniao.as_secs_f64() * 100.0).round() / 100.0,
        crates_unidos: grafos_por_crate.keys().cloned().collect(),
        nos_fantasma: fantasmas,
    };
    eprintln!(
        "  união: {} nodes, {} edges, {} soltas, {} fantasmas, {:.2}s",
        uniao.nodes_total,
        uniao.edges_total,
        uniao.arestas_soltas,
        uniao.nos_fantasma.len(),
        uniao.tempo_seg
    );
    if !uniao.nos_fantasma.is_empty() {
        eprintln!("  ↳ fantasmas (sinal de cache stale / renomeação):");
        for f in uniao.nos_fantasma.iter().take(10) {
            eprintln!(
                "      {} (esperado em {}, vem de {:?})",
                f.path, f.crate_esperado, f.origens
            );
        }
    }

    let cenario = if realizadas.is_empty() {
        "cache-quente".to_string()
    } else if realizadas.len() == conjunto.len() {
        "cold".to_string()
    } else if realizadas.len() == 1 {
        "morno-1".to_string()
    } else if realizadas.len() <= 3 {
        format!("morno-{}", realizadas.len())
    } else {
        format!("morno-{}-de-{}", realizadas.len(), conjunto.len())
    };
    let cache = CacheResumo {
        diretorio: args.cache_dir.to_string_lossy().to_string(),
        extracoes_realizadas: realizadas.clone(),
        extracoes_reusadas: reusadas.clone(),
        tempo_extracao_fork_seg: (tempo_fork.as_secs_f64() * 100.0).round() / 100.0,
        tempo_cache_io_seg: (tempo_cache_io.as_secs_f64() * 1000.0).round() / 1000.0,
        cenario: cenario.clone(),
    };
    eprintln!(
        "  cache: cenário={} extraídos={} reusados={} fork={:.2}s cache_io={:.3}s",
        cenario,
        realizadas.len(),
        reusadas.len(),
        tempo_fork.as_secs_f64(),
        tempo_cache_io.as_secs_f64(),
    );

    // 7. Mapear impacto por crate.
    let t_mapa = Instant::now();
    let processar = |arqs: &[ArquivoDiff], fonte: &'static str| -> ModoSaida {
        let mut por_crate: BTreeMap<String, Vec<ImpactoArquivo>> = BTreeMap::new();
        let mut sem_crate: Vec<String> = Vec::new();
        for a in arqs {
            match crate_de_arquivo(&a.caminho, &crates) {
                Some(c) => {
                    let Some(g_local) = grafos_por_crate.get(&c.nome) else {
                        sem_crate.push(a.caminho.clone());
                        continue;
                    };
                    let im = mapear_arquivo(
                        g_local,
                        &grafo_workspace,
                        a,
                        &raiz,
                        &paths_imprecisos,
                    );
                    por_crate.entry(c.nome.clone()).or_default().push(im);
                }
                None => sem_crate.push(a.caminho.clone()),
            }
        }
        let impacto_por_crate: Vec<ImpactoCrate> = por_crate
            .into_iter()
            .map(|(crate_nome, arquivos)| ImpactoCrate {
                crate_nome,
                arquivos,
            })
            .collect();
        ModoSaida {
            fonte,
            arquivos_tocados: arqs.to_vec(),
            impacto_por_crate,
            arquivos_sem_crate: sem_crate,
        }
    };
    let stdin_modo = stdin_arqs.as_ref().map(|a| processar(a, "stdin"));
    let git_modo = git_arqs.as_ref().map(|a| processar(a, "git"));

    // 7.3 Untracked (prompt 0043): detectar os arquivos novos não rastreados
    // e cruzar com as fontes que o cargo compilou (= `position.file` dos nós
    // do grafo unido). Ligado = está nas compiladas → mapear; solto = não
    // está → sinal "presente, não compilado".
    let untracked = {
        let detectados = ler_untracked(&raiz);
        let compiladas = fontes_compiladas(&grafo_workspace, &raiz);
        let mut rs_em_crate: Vec<String> = Vec::new();
        let mut ligados: Vec<UntrackedLigado> = Vec::new();
        let mut soltos: Vec<UntrackedSolto> = Vec::new();
        let mut nao_fonte: Vec<String> = Vec::new();
        for arq in &detectados {
            let eh_rs = arq.ends_with(".rs");
            let dono = crate_de_arquivo(arq, &crates);
            match (eh_rs, dono) {
                (true, Some(c)) => {
                    rs_em_crate.push(arq.clone());
                    if compiladas.contains(arq) {
                        let linhas = contar_linhas(&raiz, arq).max(1);
                        let arq_sint = ArquivoDiff {
                            caminho: arq.clone(),
                            faixas: vec![Faixa {
                                inicio: 1,
                                fim: linhas,
                            }],
                        };
                        let impacto = match grafos_por_crate.get(&c.nome) {
                            Some(gl) => mapear_arquivo(
                                gl,
                                &grafo_workspace,
                                &arq_sint,
                                &raiz,
                                &paths_imprecisos,
                            ),
                            None => ImpactoArquivo {
                                arquivo: arq.clone(),
                                faixas: arq_sint.faixas.clone(),
                                nos_tocados: vec![],
                            },
                        };
                        ligados.push(UntrackedLigado {
                            arquivo: arq.clone(),
                            crate_nome: c.nome.clone(),
                            linhas,
                            impacto,
                        });
                    } else {
                        soltos.push(UntrackedSolto {
                            arquivo: arq.clone(),
                            crate_nome: c.nome.clone(),
                            sinal: "presente, não compilado — ligue com um `mod` no módulo-pai",
                        });
                    }
                }
                _ => nao_fonte.push(arq.clone()),
            }
        }
        let nao_fonte_total = nao_fonte.len();
        nao_fonte.truncate(10);
        eprintln!(
            "  untracked: {} detectados, {} .rs em crate ({} ligados, {} soltos), {} não-fonte",
            detectados.len(),
            rs_em_crate.len(),
            ligados.len(),
            soltos.len(),
            nao_fonte_total,
        );
        for l in &ligados {
            eprintln!("    ↳ ligado: {} ({} nós tocados)", l.arquivo, l.impacto.nos_tocados.len());
        }
        for s in &soltos {
            eprintln!("    ↳ solto:  {} — {}", s.arquivo, s.sinal);
        }
        UntrackedResumo {
            comando: "git ls-files --others --exclude-standard",
            total_detectados: detectados.len(),
            rs_em_crate,
            ligados,
            soltos,
            nao_fonte_total,
            nao_fonte_amostra: nao_fonte,
            enumeracao_cache: "filesystem glob (coletar_fontes percorre src/*.rs) — \
                 inclui soltos → re-extração espúria; o cargo, por sua vez, só \
                 compila os ligados",
            nota: "Quadro completo da árvore de trabalho = git diff HEAD (rastreados \
                 editados) ∪ untracked ligados (hunks sintéticos). Soltos ficam fora \
                 do grafo por construção (cargo não compila); reportados como sinal.",
        }
    };

    // 7.5 Antes/depois de um path colidido tocado sinteticamente (prompt
    // 0041 §4). Compara o raio CRU-FUNDIDO (sem resolução, na união
    // ingênua) contra o raio RESOLVIDO (após resolução; cada cópia distinta
    // tem seu próprio raio).
    let antes_depois = args.simular_tocar_colidido.as_ref().map(|path_alvo| {
        // Crate dono (primeiro segmento do path).
        let crate_nome = path_alvo.split("::").next().unwrap_or("").to_string();
        // 1. União CRU (sem resolver — direto dos grafos crus, antes da
        //    etapa 5.5). Reusa `grafos_por_crate_cru`.
        let grafos_cru: Vec<(String, Grafo)> = grafos_por_crate_cru
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let ((grafo_cru_unido, _), _) = unir_grafos_com_origens(grafos_cru);
        let raio_cru_fundido = calcular_raio(&grafo_cru_unido, &PathGrafo::from(path_alvo.as_str()))
            .map(|r| raio_resumo(&r))
            .ok();
        // 2. Cópias resolvidas: olhar o censo do crate dono. Os
        //    `novos_paths` da entrada do censo são as cópias renomeadas.
        let raios_resolvidos: Vec<RaioPorCopia> = colisoes_por_crate
            .get(&crate_nome)
            .map(|c| {
                c.detalhes
                    .iter()
                    .find(|d| d.path == *path_alvo)
                    .map(|d| {
                        d.novos_paths
                            .iter()
                            .filter_map(|p_novo| {
                                calcular_raio(
                                    &grafo_workspace,
                                    &PathGrafo::from(p_novo.as_str()),
                                )
                                .ok()
                                .map(|r| RaioPorCopia {
                                    path: p_novo.clone(),
                                    raio: raio_resumo(&r),
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        let soma: usize = raios_resolvidos.iter().map(|r| r.raio.transitivos).sum();
        let cru_trans = raio_cru_fundido.as_ref().map(|r| r.transitivos as i64).unwrap_or(0);
        let delta = soma as i64 - cru_trans;
        AntesDepoisColisao {
            path_colidido: path_alvo.clone(),
            crate_nome,
            raio_cru_fundido,
            raios_resolvidos,
            soma_transitivos_resolvidos: soma,
            delta,
        }
    });
    let comparacao = match (stdin_arqs.as_ref(), git_arqs.as_ref()) {
        (Some(a), Some(b)) => Some(diferencas(a, b)),
        _ => None,
    };

    let dt_mapa = t_mapa.elapsed();
    let dt_total = t_main.elapsed();
    let cronometria = Cronometria {
        total_seg: (dt_total.as_secs_f64() * 100.0).round() / 100.0,
        metadata_seg: (dt_meta.as_secs_f64() * 1000.0).round() / 1000.0,
        extracoes_fork_seg: (tempo_fork.as_secs_f64() * 100.0).round() / 100.0,
        cache_reuso_seg: (tempo_cache_io.as_secs_f64() * 1000.0).round() / 1000.0,
        desserializar_unir_seg: ((tempo_desserializar + dt_uniao).as_secs_f64() * 1000.0).round()
            / 1000.0,
        resolucao_seg: (dt_res.as_secs_f64() * 1000.0).round() / 1000.0,
        mapeamento_raio_seg: (dt_mapa.as_secs_f64() * 1000.0).round() / 1000.0,
    };
    eprintln!(
        "  cronometria: total={:.2}s meta={:.0}ms fork={:.2}s cache_io={:.0}ms \
         desser+união={:.0}ms resolução={:.1}ms mapa={:.0}ms",
        cronometria.total_seg,
        cronometria.metadata_seg * 1000.0,
        cronometria.extracoes_fork_seg,
        cronometria.cache_reuso_seg * 1000.0,
        cronometria.desserializar_unir_seg * 1000.0,
        cronometria.resolucao_seg,
        cronometria.mapeamento_raio_seg * 1000.0,
    );

    let colisoes_resumo = ColisoesResumo {
        politica: if args.sem_resolucao { "sem-resolucao" } else { "resolver" },
        total: agg_total,
        resolvidas_distintos: agg_dist,
        resolvidas_mesmo_item: agg_msm,
        nao_determinadas: agg_nd,
        distintos_mas_colidem_pos_regra: agg_dprc,
        por_crate: colisoes_por_crate,
        tempo_seg: (dt_res.as_secs_f64() * 1000.0).round() / 1000.0,
    };

    let saida = Saida {
        raiz_repo: raiz.to_string_lossy().to_string(),
        crates_workspace: nomes_workspace,
        crates_tocados,
        extracoes,
        uniao,
        cache,
        cronometria,
        colisoes: colisoes_resumo,
        stdin: stdin_modo,
        git: git_modo,
        untracked,
        comparacao,
        antes_depois,
        nota_honestidade:
            "Impacto ESTRUTURAL (quem depende via `Uses`), não comportamental. \
             O 'raio workspace' atravessa crates (união por path); o 'raio local' \
             fica dentro do crate dono do arquivo. Cache invalida por SHA-256 \
             dos fontes do crate (pega edições não-comitadas). Colisões \
             resolvidas por crate antes da união (prompt 0041); NaoDeterminado \
             marca o raio como impreciso.",
    };

    let json = serde_json::to_string_pretty(&saida).expect("serializar");
    if let Some(p) = args.out {
        std::fs::write(&p, json).expect("escrever");
        eprintln!("saída: {}", p.display());
    } else {
        println!("{}", json);
    }
}
