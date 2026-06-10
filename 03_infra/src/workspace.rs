//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-workspace.md
//! @prompt-hash 87fe796d
//! @layer L3
//! @updated 2026-06-07
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0001-fonte-do-grafo-fork-externo.md
//!          00_nucleo/adr/0003-workspace-cargo.md
//! Camada:  L3 — Infraestrutura. I/O (filesystem, subprocesso) permitido.
//!
//! Fundação de I/O do **grafo de workspace** (a união L1 e a orquestração L4
//! vêm no 0045): enumeração de membros e extração por crate com **cache de
//! chave completa**.
//!
//! Duas regras de fronteira deste módulo:
//! - A enumeração **lê os `Cargo.toml` direto** (via `toml`), **sem**
//!   `cargo metadata` — preservando o invariante de subprocessos do cargo
//!   (só o fork e o metadata já existentes). A versão do toolchain vem do
//!   `rustc` (subprocesso de `rustc`, não de `cargo`).
//! - A extração cacheada é **aditiva**: `extrair_grafo` e o fork não mudam.
//!
//! A chave de cache fecha a limitação registrada no laudo 0040 (a chave só
//! pegava os fontes): agora cobre **fontes + `Cargo.toml` do membro +
//! `Cargo.lock` do workspace + versão do toolchain**, em ordem fixa. A
//! re-extração espúria do arquivo solto (laudo 0043) **continua aceita** —
//! a enumeração de fontes é por glob de filesystem, de propósito; não é
//! "corrigida" aqui.

use core::error::Error;
use core::fmt;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use lente_core::entities::grafo::Grafo;

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

/// Um membro do workspace: nome do **pacote** (de `[package].name`, o que o
/// fork recebe — pode diferir do nome do diretório) e o diretório que contém
/// o `Cargo.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembroWorkspace {
    pub nome: String,
    pub dir: PathBuf,
}

/// Modos de falha da fundação de workspace.
#[derive(Debug)]
pub enum ErroWorkspace {
    /// Falha de I/O lendo manifestos, fontes, lock ou o cache.
    Io(std::io::Error),
    /// `Cargo.toml` malformado, sem `[workspace]`, ou membro sem `[package]`
    /// quando esperado. Carrega contexto legível.
    Manifesto(String),
    /// O fork (`cargo modules export-json`) falhou na extração (cache miss).
    Fork(crate::fork::ErroFork),
    /// O JSON do fork não traduziu para `Grafo` (enums/invariantes da borda).
    Adaptador(crate::ErroAdaptador),
    /// `rustc --version` falhou (toolchain indisponível ou saída inesperada).
    Toolchain(String),
}

impl fmt::Display for ErroWorkspace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroWorkspace::Io(e) => write!(f, "I/O: {}", e),
            ErroWorkspace::Manifesto(m) => write!(f, "manifesto: {}", m),
            ErroWorkspace::Fork(e) => write!(f, "extração (fork): {}", e),
            ErroWorkspace::Adaptador(e) => write!(f, "tradução do grafo: {}", e),
            ErroWorkspace::Toolchain(m) => write!(f, "toolchain: {}", m),
        }
    }
}

impl Error for ErroWorkspace {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ErroWorkspace::Io(e) => Some(e),
            ErroWorkspace::Fork(e) => Some(e),
            ErroWorkspace::Adaptador(e) => Some(e),
            ErroWorkspace::Manifesto(_) | ErroWorkspace::Toolchain(_) => None,
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Enumeração de membros (I/O, sem subprocesso)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct RaizManifesto {
    workspace: Option<SecaoWorkspace>,
}

#[derive(Debug, Deserialize)]
struct SecaoWorkspace {
    #[serde(default)]
    members: Vec<String>,
    #[serde(default)]
    exclude: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MembroManifesto {
    package: Option<SecaoPackage>,
}

#[derive(Debug, Deserialize)]
struct SecaoPackage {
    name: String,
}

/// Enumera os membros do workspace em `raiz`, lendo os `Cargo.toml` direto.
///
/// - Lê `raiz/Cargo.toml`, seção `[workspace].members` (lista de caminhos,
///   podem ter glob tipo `crates/*`).
/// - Expande globs pelo filesystem (ver [`expandir_padrao`]).
/// - Para cada diretório-membro, lê seu `Cargo.toml` e extrai `[package].name`.
/// - Membro sem `[package]` (sub-workspace virtual) é **pulado**.
/// - `[workspace].exclude` remove diretórios casados.
///
/// Ordem determinística: ordem de declaração dos `members`, com cada glob
/// expandido em ordem alfabética de diretório.
pub fn enumerar_membros(raiz: &Path) -> Result<Vec<MembroWorkspace>, ErroWorkspace> {
    let manifesto = raiz.join("Cargo.toml");
    let txt = std::fs::read_to_string(&manifesto).map_err(ErroWorkspace::Io)?;
    let man: RaizManifesto = toml::from_str(&txt)
        .map_err(|e| ErroWorkspace::Manifesto(format!("{}: {}", manifesto.display(), e)))?;
    let ws = man.workspace.ok_or_else(|| {
        ErroWorkspace::Manifesto(format!("{} não tem seção [workspace]", manifesto.display()))
    })?;

    // Diretórios excluídos (canonizados para comparar com segurança).
    let mut excluidos: BTreeSet<PathBuf> = BTreeSet::new();
    for ex in &ws.exclude {
        for d in expandir_padrao(raiz, ex) {
            excluidos.insert(d.canonicalize().unwrap_or(d));
        }
    }

    let mut membros: Vec<MembroWorkspace> = Vec::new();
    let mut vistos: BTreeSet<PathBuf> = BTreeSet::new();
    for padrao in &ws.members {
        for dir in expandir_padrao(raiz, padrao) {
            let canon = dir.canonicalize().unwrap_or_else(|_| dir.clone());
            if excluidos.contains(&canon) {
                continue;
            }
            if !vistos.insert(canon) {
                continue; // mesmo diretório casado por dois padrões
            }
            let man_membro = dir.join("Cargo.toml");
            let txt = match std::fs::read_to_string(&man_membro) {
                Ok(t) => t,
                Err(_) => continue, // diretório sem Cargo.toml: não é crate
            };
            let m: MembroManifesto = toml::from_str(&txt).map_err(|e| {
                ErroWorkspace::Manifesto(format!("{}: {}", man_membro.display(), e))
            })?;
            match m.package {
                Some(pkg) => membros.push(MembroWorkspace {
                    nome: pkg.name,
                    dir,
                }),
                None => continue, // sub-workspace virtual: pular
            }
        }
    }
    Ok(membros)
}

/// Expande um padrão de membro (relativo à `raiz`) em diretórios existentes.
///
/// Suporta `*` como curinga **por componente** (ex.: `crates/*`, `a/*/b`,
/// `pre*suf`). Um `*` casa qualquer trecho dentro de um componente; há no
/// máximo um `*` por componente (limitação registrada — cobre o caso comum
/// dos workspaces Cargo). Sem `*`, é um caminho direto.
fn expandir_padrao(raiz: &Path, padrao: &str) -> Vec<PathBuf> {
    let componentes: Vec<&str> = padrao.split('/').filter(|c| !c.is_empty()).collect();
    let mut atuais = vec![raiz.to_path_buf()];
    for comp in componentes {
        let mut proximos: Vec<PathBuf> = Vec::new();
        if comp.contains('*') {
            for base in &atuais {
                let Ok(rd) = std::fs::read_dir(base) else {
                    continue;
                };
                let mut casados: Vec<PathBuf> = rd
                    .flatten()
                    .map(|e| e.path())
                    .filter(|p| p.is_dir())
                    .filter(|p| {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .map(|n| casa_curinga(comp, n))
                            .unwrap_or(false)
                    })
                    .collect();
                casados.sort();
                proximos.extend(casados);
            }
        } else {
            for base in &atuais {
                let p = base.join(comp);
                if p.is_dir() {
                    proximos.push(p);
                }
            }
        }
        atuais = proximos;
    }
    atuais
}

/// Casa um componente com curinga (`*` único) contra um nome de diretório.
fn casa_curinga(padrao: &str, nome: &str) -> bool {
    if padrao == "*" {
        return true;
    }
    match padrao.split_once('*') {
        Some((pre, suf)) => {
            nome.len() >= pre.len() + suf.len() && nome.starts_with(pre) && nome.ends_with(suf)
        }
        None => padrao == nome,
    }
}

// ---------------------------------------------------------------------------
// 2. Versão do toolchain (subprocesso de `rustc`, não `cargo`)
// ---------------------------------------------------------------------------

/// Devolve a string do toolchain (`rustc --version`). **Uma** chamada por
/// rodada do grafo (o L4 consulta uma vez e passa como parâmetro adiante —
/// por isso a versão é argumento da extração, não consulta interna por
/// membro). Respeita um eventual `rust-toolchain.toml` via shim do rustup
/// (o `rustc` resolvido já é o pinado); não há pin neste projeto.
pub fn versao_toolchain() -> Result<String, ErroWorkspace> {
    let saida = Command::new("rustc")
        .arg("--version")
        .output()
        .map_err(|e| ErroWorkspace::Toolchain(format!("falha ao rodar rustc: {}", e)))?;
    if !saida.status.success() {
        return Err(ErroWorkspace::Toolchain(format!(
            "rustc --version saiu com {:?}: {}",
            saida.status.code(),
            String::from_utf8_lossy(&saida.stderr).trim()
        )));
    }
    String::from_utf8(saida.stdout)
        .map(|s| s.trim().to_string())
        .map_err(|e| ErroWorkspace::Toolchain(format!("stdout do rustc não é UTF-8: {}", e)))
}

// ---------------------------------------------------------------------------
// 3+4. Chave de cache completa e extração cacheada
// ---------------------------------------------------------------------------

/// Subdiretório do cache, sob `target/` (já gitignorado — sem mudança no
/// `.gitignore`).
fn dir_cache(raiz: &Path) -> PathBuf {
    raiz.join("target").join("lente-cache")
}

/// Computa a chave de cache (SHA-256) do membro. **Ordem fixa**, com domínio
/// separado por rótulo para evitar ambiguidade entre componentes:
///
/// 1. os **fontes** do membro — glob `dir/src/**.rs`, ordenado por caminho
///    relativo (decisão 0043: glob de filesystem, aceita re-extração espúria
///    do arquivo solto);
/// 2. o **`Cargo.toml`** do membro (o que faltava no 0040);
/// 3. o **`Cargo.lock`** do workspace (`raiz/Cargo.lock`; ausente → vazio);
/// 4. a **versão do toolchain**.
///
/// Isolável para testar a invalidação sem rodar o fork.
pub fn chave_cache(
    membro: &MembroWorkspace,
    raiz: &Path,
    versao_toolchain: &str,
) -> Result<String, ErroWorkspace> {
    let mut h = Sha256::new();

    // 1. fontes
    h.update(b"FONTES\0");
    for (rel, conteudo) in coletar_fontes(&membro.dir.join("src"))? {
        h.update(rel.as_bytes());
        h.update(b"\0");
        h.update((conteudo.len() as u64).to_le_bytes());
        h.update(b"\0");
        h.update(&conteudo);
        h.update(b"\0");
    }

    // 2. Cargo.toml do membro
    h.update(b"CARGO_TOML\0");
    let toml_membro = std::fs::read(membro.dir.join("Cargo.toml")).map_err(ErroWorkspace::Io)?;
    h.update((toml_membro.len() as u64).to_le_bytes());
    h.update(b"\0");
    h.update(&toml_membro);
    h.update(b"\0");

    // 3. Cargo.lock do workspace (ausente → vazio, determinístico)
    h.update(b"CARGO_LOCK\0");
    let lock = ler_opcional(&raiz.join("Cargo.lock"))?;
    h.update((lock.len() as u64).to_le_bytes());
    h.update(b"\0");
    h.update(&lock);
    h.update(b"\0");

    // 4. toolchain
    h.update(b"TOOLCHAIN\0");
    h.update(versao_toolchain.as_bytes());
    h.update(b"\0");

    Ok(format!("{:x}", h.finalize()))
}

/// Extrai o `Grafo` do membro, com cache de chave completa.
///
/// - Chave existe no cache → lê o JSON cru → desserializa (**acerto**, sem
///   fork).
/// - Não existe → roda o fork (`fork::invocar_fork`), grava o JSON cru no
///   cache de forma **atômica** (temp + rename), e desserializa (**erro de
///   cache**, com fork).
///
/// Concorrência: uso reativo (uma rodada por vez); duas rodadas extraindo o
/// mesmo crate são last-write-wins inofensivo (mesma chave ⇒ mesmo conteúdo).
pub fn extrair_grafo_cacheado(
    membro: &MembroWorkspace,
    raiz: &Path,
    versao_toolchain: &str,
) -> Result<Grafo, ErroWorkspace> {
    let chave = chave_cache(membro, raiz, versao_toolchain)?;
    let dir = dir_cache(raiz);
    let destino = dir.join(format!("{}.json", chave));

    match std::fs::read_to_string(&destino) {
        Ok(json) => return crate::desserializar_grafo(&json).map_err(ErroWorkspace::Adaptador),
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(ErroWorkspace::Io(e)),
        Err(_) => {} // NotFound → cache miss, segue para o fork
    }

    let json = crate::fork::invocar_fork(&membro.nome).map_err(ErroWorkspace::Fork)?;
    std::fs::create_dir_all(&dir).map_err(ErroWorkspace::Io)?;
    escrever_atomico(&destino, json.as_bytes())?;
    crate::desserializar_grafo(&json).map_err(ErroWorkspace::Adaptador)
}

// ---------------------------------------------------------------------------
// Helpers de I/O
// ---------------------------------------------------------------------------

/// Lista recursivamente os `.rs` sob `src`, em ordem determinística por
/// caminho relativo, devolvendo `(rel, conteúdo)`. `src` ausente → vazio.
fn coletar_fontes(src: &Path) -> Result<Vec<(String, Vec<u8>)>, ErroWorkspace> {
    let mut acc: Vec<(String, Vec<u8>)> = Vec::new();
    if !src.exists() {
        return Ok(acc);
    }
    fn walk(base: &Path, dir: &Path, acc: &mut Vec<(String, Vec<u8>)>) -> Result<(), ErroWorkspace> {
        let rd = std::fs::read_dir(dir).map_err(ErroWorkspace::Io)?;
        let mut entradas: Vec<PathBuf> = Vec::new();
        for e in rd {
            entradas.push(e.map_err(ErroWorkspace::Io)?.path());
        }
        entradas.sort();
        for p in entradas {
            if p.is_dir() {
                walk(base, &p, acc)?;
            } else if p.extension().map(|e| e == "rs").unwrap_or(false) {
                let conteudo = std::fs::read(&p).map_err(ErroWorkspace::Io)?;
                let rel = p
                    .strip_prefix(base)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .into_owned();
                acc.push((rel, conteudo));
            }
        }
        Ok(())
    }
    walk(src, src, &mut acc)?;
    acc.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(acc)
}

/// Lê um arquivo; ausente (`NotFound`) → vazio. Outras falhas de I/O propagam.
fn ler_opcional(p: &Path) -> Result<Vec<u8>, ErroWorkspace> {
    match std::fs::read(p) {
        Ok(b) => Ok(b),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(ErroWorkspace::Io(e)),
    }
}

/// Escreve `conteudo` em `destino` de forma atômica: grava num temp irmão
/// (sufixado pelo PID, para writers concorrentes não colidirem) e renomeia.
/// `rename` no mesmo diretório é atômico — o cache nunca tem entrada parcial.
fn escrever_atomico(destino: &Path, conteudo: &[u8]) -> Result<(), ErroWorkspace> {
    let dir = destino.parent().ok_or_else(|| {
        ErroWorkspace::Manifesto("destino do cache sem diretório-pai".to_string())
    })?;
    let nome = destino
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("cache");
    let tmp = dir.join(format!(".{}.{}.tmp", nome, std::process::id()));
    std::fs::write(&tmp, conteudo).map_err(ErroWorkspace::Io)?;
    std::fs::rename(&tmp, destino).map_err(ErroWorkspace::Io)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cria um diretório temporário único para o teste (sem dep externa).
    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "__lente_ws_{}_{}__",
            tag,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn escrever(p: &Path, conteudo: &str) {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(p, conteudo).unwrap();
    }

    fn crate_em(dir: &Path, nome: &str) {
        escrever(
            &dir.join("Cargo.toml"),
            &format!("[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n", nome),
        );
        escrever(&dir.join("src/lib.rs"), "pub fn x() {}\n");
    }

    // ---- Enumeração -------------------------------------------------------

    #[test]
    fn enumera_membros_diretos_e_glob_pulando_virtual() {
        let raiz = temp_dir("enum");
        escrever(
            &raiz.join("Cargo.toml"),
            "[workspace]\nresolver = \"2\"\nmembers = [\"a\", \"crates/*\", \"virtual_dir\"]\n",
        );
        crate_em(&raiz.join("a"), "pacote_a");
        crate_em(&raiz.join("crates/b"), "pacote_b");
        crate_em(&raiz.join("crates/c"), "pacote_c");
        // membro virtual: tem Cargo.toml mas sem [package] → pulado
        escrever(&raiz.join("virtual_dir/Cargo.toml"), "[workspace]\n");

        let membros = enumerar_membros(&raiz).expect("enumeração deve funcionar");
        let nomes: Vec<&str> = membros.iter().map(|m| m.nome.as_str()).collect();
        // ordem: "a" → pacote_a; "crates/*" expandido alfabético → b, c
        assert_eq!(nomes, vec!["pacote_a", "pacote_b", "pacote_c"]);
        // dir correto (o do glob aponta para crates/b)
        assert!(membros[1].dir.ends_with("crates/b"));

        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn exclude_remove_membro_casado() {
        let raiz = temp_dir("excl");
        escrever(
            &raiz.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\nexclude = [\"crates/ignorado\"]\n",
        );
        crate_em(&raiz.join("crates/bom"), "pacote_bom");
        crate_em(&raiz.join("crates/ignorado"), "pacote_ignorado");

        let membros = enumerar_membros(&raiz).unwrap();
        let nomes: Vec<&str> = membros.iter().map(|m| m.nome.as_str()).collect();
        assert_eq!(nomes, vec!["pacote_bom"]);

        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn casa_curinga_basico() {
        assert!(casa_curinga("*", "qualquer"));
        assert!(casa_curinga("pre*", "prefixado"));
        assert!(casa_curinga("*suf", "tem_suf"));
        assert!(casa_curinga("a*z", "abcz"));
        assert!(!casa_curinga("pre*", "outro"));
        assert!(!casa_curinga("a*z", "abc")); // sem sufixo z
        assert!(casa_curinga("exato", "exato"));
        assert!(!casa_curinga("exato", "outro"));
    }

    /// Não-regressão de layout real: a enumeração do workspace principal NÃO
    /// lista nenhum crate do `lab/` (que tem `[workspace]` próprio, logo não é
    /// membro do principal). Lê só arquivos — sem subprocesso.
    #[test]
    fn enumera_workspace_principal_sem_lab() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("raiz do workspace")
            .to_path_buf();
        let membros = enumerar_membros(&raiz).expect("enumeração do workspace real");
        let nomes: Vec<&str> = membros.iter().map(|m| m.nome.as_str()).collect();
        assert!(nomes.contains(&"lente_core"), "lente_core deve estar: {:?}", nomes);
        assert!(nomes.contains(&"lente_infra"));
        // Nenhum crate do lab (ex.: proto-impacto-diff) aparece.
        assert!(
            !membros.iter().any(|m| m.dir.components().any(|c| c.as_os_str() == "lab")),
            "nenhum membro deve vir de lab/: {:?}",
            membros
        );
        assert!(!nomes.iter().any(|n| n.contains("proto")), "sem crate proto-*: {:?}", nomes);
    }

    // ---- Toolchain --------------------------------------------------------

    /// `rustc --version` está garantido num ambiente que roda `cargo test`.
    #[test]
    fn versao_toolchain_contem_rustc() {
        let v = versao_toolchain().expect("rustc deve responder");
        assert!(v.contains("rustc"), "versão inesperada: {}", v);
    }

    // ---- Chave de cache: estabilidade e invalidação -----------------------

    /// Monta um membro real (src/lib.rs + Cargo.toml) e uma raiz com
    /// Cargo.lock, e devolve `(raiz, membro)` para os testes de chave.
    fn fixture_membro(tag: &str) -> (PathBuf, MembroWorkspace) {
        let raiz = temp_dir(tag);
        escrever(&raiz.join("Cargo.lock"), "# lock v1\n");
        let dir = raiz.join("m");
        crate_em(&dir, "membro_x");
        (
            raiz.clone(),
            MembroWorkspace {
                nome: "membro_x".to_string(),
                dir,
            },
        )
    }

    #[test]
    fn chave_estavel_para_mesmos_insumos() {
        let (raiz, m) = fixture_membro("chave_estavel");
        let a = chave_cache(&m, &raiz, "rustc 1.91.0").unwrap();
        let b = chave_cache(&m, &raiz, "rustc 1.91.0").unwrap();
        assert_eq!(a, b);
        assert_eq!(a.len(), 64, "SHA-256 hex tem 64 chars");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn chave_muda_com_fonte() {
        let (raiz, m) = fixture_membro("chave_fonte");
        let antes = chave_cache(&m, &raiz, "tc").unwrap();
        escrever(&m.dir.join("src/lib.rs"), "pub fn x() { let _ = 1; }\n");
        let depois = chave_cache(&m, &raiz, "tc").unwrap();
        assert_ne!(antes, depois);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn chave_muda_com_novo_arquivo_fonte() {
        // Achado 0043: glob de filesystem — um .rs novo (mesmo solto) muda a
        // chave. Comportamento aceito de propósito, não corrigido aqui.
        let (raiz, m) = fixture_membro("chave_novo_arq");
        let antes = chave_cache(&m, &raiz, "tc").unwrap();
        escrever(&m.dir.join("src/extra.rs"), "pub fn y() {}\n");
        let depois = chave_cache(&m, &raiz, "tc").unwrap();
        assert_ne!(antes, depois);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn chave_muda_com_cargo_toml_do_membro() {
        // O que faltava no 0040: mudança de features/deps no Cargo.toml invalida.
        let (raiz, m) = fixture_membro("chave_toml");
        let antes = chave_cache(&m, &raiz, "tc").unwrap();
        escrever(
            &m.dir.join("Cargo.toml"),
            "[package]\nname = \"membro_x\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[dependencies]\nserde = \"1\"\n",
        );
        let depois = chave_cache(&m, &raiz, "tc").unwrap();
        assert_ne!(antes, depois);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn chave_muda_com_cargo_lock_do_workspace() {
        let (raiz, m) = fixture_membro("chave_lock");
        let antes = chave_cache(&m, &raiz, "tc").unwrap();
        escrever(&raiz.join("Cargo.lock"), "# lock v2 — dep nova\n");
        let depois = chave_cache(&m, &raiz, "tc").unwrap();
        assert_ne!(antes, depois);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn chave_muda_com_toolchain() {
        let (raiz, m) = fixture_membro("chave_tc");
        let a = chave_cache(&m, &raiz, "rustc 1.91.0").unwrap();
        let b = chave_cache(&m, &raiz, "rustc 1.92.0").unwrap();
        assert_ne!(a, b);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    // ---- Extração cacheada: ACERTO sem fork (pré-gravado) -----------------

    #[test]
    fn cache_hit_le_sem_rodar_fork() {
        // Pré-grava um JSON cru válido na chave do membro. `extrair_grafo_cacheado`
        // deve LER o cache e devolver o Grafo — sem tocar o fork (o nome do
        // membro é fictício; se o fork rodasse, falharia).
        let (raiz, m) = fixture_membro("hit");
        let chave = chave_cache(&m, &raiz, "tc").unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        let json = r#"{
            "crate": "membro_x",
            "nodes": [
                {"id":1,"path":"membro_x","name":"membro_x","kind":"crate","visibility":"pub"}
            ],
            "edges": []
        }"#;
        escrever(&destino, json);

        let g = extrair_grafo_cacheado(&m, &raiz, "tc").expect("deve ler o cache");
        assert_eq!(g.crate_name, "membro_x");
        assert_eq!(g.nodes.len(), 1);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn cache_hit_json_corrompido_propaga_erro_de_traducao() {
        // Entrada de cache corrompida (não é JSON do fork) → erro de tradução,
        // não pânico. Documenta que o cache não é "auto-curado" silenciosamente.
        let (raiz, m) = fixture_membro("hit_corrompido");
        let chave = chave_cache(&m, &raiz, "tc").unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        escrever(&destino, "{ não é json do fork");
        match extrair_grafo_cacheado(&m, &raiz, "tc") {
            Err(ErroWorkspace::Adaptador(_)) => {}
            outro => panic!("esperava Adaptador, veio {:?}", outro),
        }
        let _ = std::fs::remove_dir_all(&raiz);
    }

    // ---- Escrita atômica --------------------------------------------------

    #[test]
    fn escrita_atomica_grava_e_nao_deixa_temp() {
        let dir = temp_dir("atomico");
        let destino = dir.join("abc.json");
        escrever_atomico(&destino, b"conteudo").unwrap();
        assert_eq!(std::fs::read_to_string(&destino).unwrap(), "conteudo");
        // Nenhum arquivo .tmp remanescente.
        let restos: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(restos.is_empty(), "temp não deve sobrar: {:?}", restos);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- Display dos erros ------------------------------------------------

    #[test]
    fn display_cobre_variantes_de_erro_workspace() {
        let variantes: Vec<ErroWorkspace> = vec![
            ErroWorkspace::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
            ErroWorkspace::Manifesto("sem [workspace]".to_string()),
            ErroWorkspace::Fork(crate::fork::ErroFork::StatusErro {
                codigo: Some(1),
                stderr: "falhou".to_string(),
            }),
            ErroWorkspace::Adaptador(crate::ErroAdaptador::JsonInvalido("eof".to_string())),
            ErroWorkspace::Toolchain("rustc ausente".to_string()),
        ];
        for v in &variantes {
            assert!(!format!("{}", v).is_empty());
        }
    }

    // ---- E2E com fork (#[ignore], padrão dos E2E do crate) ----------------

    /// Cache vazio + fork disponível: roda o fork, grava o cache, devolve o
    /// Grafo. Requer o fork instalado e `cargo test` da raiz do workspace.
    #[test]
    #[ignore]
    fn e2e_cache_miss_roda_fork_e_grava() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let membro = MembroWorkspace {
            nome: "lente_core".to_string(),
            dir: raiz.join("01_core").join("core"),
        };
        let tc = versao_toolchain().unwrap();
        let chave = chave_cache(&membro, &raiz, &tc).unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        let _ = std::fs::remove_file(&destino);

        let g = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("fork deve extrair");
        assert_eq!(g.crate_name, "lente_core");
        assert!(destino.exists(), "cache deve ter sido gravado");
    }

    /// Transparência: erro→fork→grava e depois acerto→cache devolvem Grafos
    /// iguais. Requer o fork.
    #[test]
    #[ignore]
    fn e2e_cache_transparente_miss_depois_hit() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf();
        let membro = MembroWorkspace {
            nome: "lente_core".to_string(),
            dir: raiz.join("01_core").join("core"),
        };
        let tc = versao_toolchain().unwrap();
        let chave = chave_cache(&membro, &raiz, &tc).unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        let _ = std::fs::remove_file(&destino);

        let g1 = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("miss");
        let g2 = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("hit");
        assert_eq!(g1, g2, "cache deve ser transparente");
    }
}
