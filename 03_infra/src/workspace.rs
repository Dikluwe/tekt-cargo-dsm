//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-workspace.md
//! @prompt-hash b9fcc0e7
//! @layer L3
//! @updated 2026-09-26
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0001-fonte-do-grafo-fork-externo.md
//!          00_nucleo/adr/0003-workspace-cargo.md
//! Camada:  L3 — Infraestrutura. I/O (filesystem, subprocesso) permitido.
//!
//! Enumeração por TOML, descoberta de toolchain no diretório do membro e
//! extração com validade conservadora. Ver infra-cache-validade.md: chave v2,
//! dependências locais transitivas, motivos de bypass e contexto por rodada.

use core::error::Error;
use core::fmt;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

use lente_core::entities::grafo::Grafo;

// ---------------------------------------------------------------------------
// Tipos públicos
// ---------------------------------------------------------------------------

/// Um membro do workspace: nome do **pacote** (de `[package].name`, o que o
/// fork recebe — pode diferir do nome do diretório) e o diretório que contém
/// o `Cargo.toml`. Não é o nome canônico do crate: esse vem de
/// `Grafo.crate_name` após a extração, inclusive quando há `[lib].name`.
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

/// Natureza de uma raiz para o `--comparar` (prompt 0075): um diretório-de-crate
/// (tem `[package]`) ou um workspace virtual (só `[workspace]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NaturezaRaiz {
    Crate,
    Workspace,
}

/// Manifesto mínimo só para detectar a natureza da raiz: presença de `[package]`
/// e/ou `[workspace]`. (Um crate pode ter ambos — é tratado como crate.)
#[derive(Debug, Deserialize)]
struct ManifestoNatureza {
    package: Option<toml::Value>,
    workspace: Option<toml::Value>,
}

/// Detecta a natureza de uma raiz lendo o `Cargo.toml` (prompt 0075): tem
/// `[package]` → [`NaturezaRaiz::Crate`]; só `[workspace]` → `Workspace`; nenhum
/// (ou `Cargo.toml` ausente) → erro claro citando a raiz. Decisão **estrutural**
/// (presença de tabela), não casamento de string de erro frágil.
pub fn natureza_raiz(raiz: &Path) -> Result<NaturezaRaiz, ErroWorkspace> {
    let manifesto = raiz.join("Cargo.toml");
    let txt = std::fs::read_to_string(&manifesto).map_err(|_| {
        ErroWorkspace::Manifesto(format!(
            "{}: Cargo.toml ausente ou ilegível (a raiz é um diretório de crate ou workspace?)",
            manifesto.display()
        ))
    })?;
    let man: ManifestoNatureza = toml::from_str(&txt)
        .map_err(|e| ErroWorkspace::Manifesto(format!("{}: {}", manifesto.display(), e)))?;
    if man.package.is_some() {
        Ok(NaturezaRaiz::Crate)
    } else if man.workspace.is_some() {
        Ok(NaturezaRaiz::Workspace)
    } else {
        Err(ErroWorkspace::Manifesto(format!(
            "{} não tem [package] nem [workspace]",
            manifesto.display()
        )))
    }
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

/// Devolve a versão do toolchain no mesmo diretório usado pela extração.
/// O shim do rustup respeita overrides por membro; não instala toolchains.
pub fn versao_toolchain(diretorio: &Path) -> Result<String, ErroWorkspace> {
    let versao = executar_toolchain(
        diretorio,
        Command::new("rustc"),
        &["--version", "--verbose"],
    )?;
    let sysroot = executar_toolchain(diretorio, Command::new("rustc"), &["--print", "sysroot"])?;
    // Fronteiras inequívocas entre versão completa e identidade da instalação.
    Ok(serde_json::json!([versao, sysroot]).to_string())
}

// Command injetável para testar subprocessos sem alterar PATH ou cwd globais.
#[cfg(test)]
fn consultar_toolchain(diretorio: &Path, comando: Command) -> Result<String, ErroWorkspace> {
    executar_toolchain(diretorio, comando, &["--version"])
}

fn executar_toolchain(
    diretorio: &Path,
    mut comando: Command,
    argumentos: &[&str],
) -> Result<String, ErroWorkspace> {
    let contexto = || format!("rustc {} em {}", argumentos.join(" "), diretorio.display());
    let saida = comando
        .args(argumentos)
        .current_dir(diretorio)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .output()
        .map_err(|e| ErroWorkspace::Toolchain(format!("falha ao rodar {}: {}", contexto(), e)))?;
    if !saida.status.success() {
        return Err(ErroWorkspace::Toolchain(format!(
            "{} saiu com {:?}: {}",
            contexto(),
            saida.status.code(),
            String::from_utf8_lossy(&saida.stderr).trim()
        )));
    }
    let texto = String::from_utf8(saida.stdout).map_err(|e| {
        ErroWorkspace::Toolchain(format!("stdout de {} não é UTF-8: {}", contexto(), e))
    })?;
    if texto.trim().is_empty() {
        return Err(ErroWorkspace::Toolchain(format!(
            "stdout vazio de {}",
            contexto()
        )));
    }
    Ok(texto.trim().to_string())
}

// ---------------------------------------------------------------------------
// 3+4. Chave v2 e extração conservadora
// ---------------------------------------------------------------------------

/// Subdiretório do cache, sob `target/` (já gitignorado — sem mudança no
/// `.gitignore`).
fn dir_cache(raiz: &Path) -> PathBuf {
    raiz.join("target").join("lente-cache")
}

/// Digest v2. Não prova reutilização: consultar `CacheExtracao::avaliar`
/// para conhecer motivos de bypass. A versão deve corresponder ao membro.
pub fn chave_cache(
    membro: &MembroWorkspace,
    raiz: &Path,
    versao_toolchain: &str,
) -> Result<String, ErroWorkspace> {
    Ok(crate::CacheExtracao::default()
        .avaliar(membro, raiz, versao_toolchain)?
        .chave)
}

/// Conveniência para uma extração. Para vários membros, compartilhar uma
/// `CacheExtracao` evita reler árvores, dependências e executável do fork.
pub fn extrair_grafo_cacheado(
    membro: &MembroWorkspace,
    raiz: &Path,
    versao_toolchain: &str,
) -> Result<Grafo, ErroWorkspace> {
    crate::CacheExtracao::default().extrair(membro, raiz, versao_toolchain)
}

impl crate::CacheExtracao {
    /// Reutiliza somente quando todas as entradas estão no domínio do contrato.
    /// Entradas com I/O dinâmico/configuração não rastreável fazem bypass.
    pub fn extrair(
        &mut self,
        membro: &MembroWorkspace,
        raiz: &Path,
        versao_toolchain: &str,
    ) -> Result<Grafo, ErroWorkspace> {
        let validade = self.avaliar(membro, raiz, versao_toolchain)?;
        extrair_com_validade(membro, raiz, &validade, crate::invocacao::invocar)
    }
}

fn extrair_com_validade(
    membro: &MembroWorkspace,
    raiz: &Path,
    validade: &crate::ValidadeCache,
    invocar: impl FnOnce(&Path) -> Result<String, crate::ErroAdaptador>,
) -> Result<Grafo, ErroWorkspace> {
    let destino = dir_cache(raiz).join(format!("{}.json", validade.chave));
    let reutilizavel = validade.motivos_bypass.is_empty();
    if reutilizavel {
        match std::fs::read_to_string(&destino) {
            Ok(json) => return crate::desserializar_grafo(&json).map_err(ErroWorkspace::Adaptador),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(ErroWorkspace::Io(e)),
        }
    }
    let json = invocar(&membro.dir).map_err(ErroWorkspace::Adaptador)?;
    // A validação antecede até a criação do diretório de cache.
    let grafo = crate::desserializar_grafo(&json).map_err(ErroWorkspace::Adaptador)?;
    if reutilizavel {
        std::fs::create_dir_all(dir_cache(raiz)).map_err(ErroWorkspace::Io)?;
        escrever_atomico(&destino, json.as_bytes())?;
    }
    Ok(grafo)
}

static SEQUENCIA_TEMP: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn criar_temporario(destino: &Path) -> Result<(PathBuf, std::fs::File), ErroWorkspace> {
    use std::sync::atomic::Ordering;
    loop {
        let seq = SEQUENCIA_TEMP.fetch_add(1, Ordering::Relaxed);
        let mut nome = destino.as_os_str().to_os_string();
        nome.push(format!(".{}.{}.tmp", std::process::id(), seq));
        let p = PathBuf::from(nome);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&p)
        {
            Ok(f) => return Ok((p, f)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(ErroWorkspace::Io(e)),
        }
    }
}

/// Temporário exclusivo por chamada/processo; rename publica apenas JSON completo.
fn escrever_atomico(destino: &Path, conteudo: &[u8]) -> Result<(), ErroWorkspace> {
    use std::io::Write;
    let (tmp, mut arquivo) = criar_temporario(destino)?;
    let resultado = (|| {
        arquivo.write_all(conteudo)?;
        arquivo.sync_all()?;
        drop(arquivo);
        std::fs::rename(&tmp, destino)
    })();
    if resultado.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    resultado.map_err(ErroWorkspace::Io)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- natureza_raiz (prompt 0075) ----

    fn escrever_cargo(tag: &str, cargo: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("__lente_nat_{}_{}__", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("Cargo.toml"), cargo).unwrap();
        d
    }

    #[test]
    fn natureza_raiz_detecta_crate_workspace_e_erro() {
        let crate_dir = escrever_cargo(
            "crate",
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\nedition=\"2024\"\n",
        );
        assert_eq!(natureza_raiz(&crate_dir).unwrap(), NaturezaRaiz::Crate);

        let ws_dir = escrever_cargo("ws", "[workspace]\nmembers=[\"a\"]\n");
        assert_eq!(natureza_raiz(&ws_dir).unwrap(), NaturezaRaiz::Workspace);

        // package + workspace (crate que também é raiz de workspace) → crate.
        let ambos = escrever_cargo(
            "ambos",
            "[workspace]\n\n[package]\nname=\"y\"\nversion=\"0\"\nedition=\"2024\"\n",
        );
        assert_eq!(natureza_raiz(&ambos).unwrap(), NaturezaRaiz::Crate);

        // sem nenhuma das tabelas → erro claro.
        let nenhum = escrever_cargo("nenhum", "[dependencies]\n");
        assert!(natureza_raiz(&nenhum).is_err());

        // Cargo.toml ausente → erro claro citando a raiz.
        let vazio =
            std::env::temp_dir().join(format!("__lente_nat_vazio_{}__", std::process::id()));
        let _ = std::fs::remove_dir_all(&vazio);
        std::fs::create_dir_all(&vazio).unwrap();
        assert!(natureza_raiz(&vazio).is_err());

        for d in [crate_dir, ws_dir, ambos, nenhum, vazio] {
            let _ = std::fs::remove_dir_all(d);
        }
    }

    /// Cria um diretório temporário único para o teste (sem dep externa).
    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("__lente_ws_{}_{}__", tag, std::process::id()));
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
            &format!(
                "[package]\nname = \"{}\"\nversion = \"0.0.0\"\nedition = \"2024\"\n",
                nome
            ),
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
        assert!(
            nomes.contains(&"lente_core"),
            "lente_core deve estar: {:?}",
            nomes
        );
        assert!(nomes.contains(&"lente_infra"));
        // Nenhum crate do lab (ex.: proto-impacto-diff) aparece.
        assert!(
            !membros
                .iter()
                .any(|m| m.dir.components().any(|c| c.as_os_str() == "lab")),
            "nenhum membro deve vir de lab/: {:?}",
            membros
        );
        assert!(
            !nomes.iter().any(|n| n.contains("proto")),
            "sem crate proto-*: {:?}",
            nomes
        );
    }

    // ---- Toolchain --------------------------------------------------------

    #[cfg(unix)]
    fn shell(script: &str) -> Command {
        let mut comando = Command::new("/bin/sh");
        comando.args(["-c", script, "rustc-controlado"]);
        comando
    }

    #[test]
    #[cfg(unix)]
    fn consulta_toolchain_no_diretorio_recebido_sem_mudar_cwd() {
        let cwd = std::env::current_dir().unwrap();
        let (a, membro_a) = fixture_membro("toolchain_a");
        let (b, membro_b) = fixture_membro("toolchain_b");
        escrever(&membro_a.dir.join("versao"), "rustc contexto-a\n");
        escrever(&membro_b.dir.join("versao"), "rustc contexto-b\n");
        let consultar =
            |dir: &Path| {
                consultar_toolchain(dir, shell(
            "test \"$1\" = --version && test \"$RUSTUP_AUTO_INSTALL\" = 0 && cat versao"
        )).unwrap()
            };
        assert_ne!(cwd, a);
        let versao_a = consultar(&membro_a.dir);
        let versao_b = consultar(&membro_b.dir);
        assert_eq!(versao_a, "rustc contexto-a");
        assert_eq!(versao_b, "rustc contexto-b");
        // Versão e diretório são componentes independentes da chave v2.
        let chave_a = chave_cache(&membro_a, &a, &versao_a).unwrap();
        assert_ne!(chave_a, chave_cache(&membro_b, &b, &versao_a).unwrap());
        assert_ne!(chave_a, chave_cache(&membro_a, &a, &versao_b).unwrap());
        assert_eq!(std::env::current_dir().unwrap(), cwd);
        std::fs::remove_dir_all(a).unwrap();
        std::fs::remove_dir_all(b).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn contexto_sysroot_usa_diretorio_e_rejeita_saida_vazia() {
        let dir = temp_dir("sysroot_contexto");
        escrever(&dir.join("sysroot"), "/toolchains/contexto-especifico");
        let saida = executar_toolchain(&dir, shell("test \"$1\" = --print && test \"$2\" = sysroot && test \"$RUSTUP_AUTO_INSTALL\" = 0 && cat sysroot"), &["--print", "sysroot"]).unwrap();
        assert_eq!(saida, "/toolchains/contexto-especifico");
        let erro = executar_toolchain(&dir, shell("exit 0"), &["--print", "sysroot"]).unwrap_err();
        assert!(erro.to_string().contains("stdout vazio"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn toolchain_falha_de_execucao_tem_contexto() {
        let dir = temp_dir("toolchain_spawn");
        let erro = consultar_toolchain(&dir, Command::new(dir.join("inexistente"))).unwrap_err();
        assert!(matches!(erro, ErroWorkspace::Toolchain(_)));
        let mensagem = erro.to_string();
        assert!(mensagem.contains("falha ao rodar rustc"));
        assert!(mensagem.contains(dir.to_str().unwrap()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(unix)]
    fn toolchain_preserva_status_stderr_e_stdout_invalido() {
        let dir = temp_dir("toolchain_erros");
        let erro = consultar_toolchain(&dir, shell("echo toolchain-indisponivel >&2; exit 42"))
            .unwrap_err()
            .to_string();
        assert!(erro.contains("42"));
        assert!(erro.contains("toolchain-indisponivel"));
        assert!(erro.contains(dir.to_str().unwrap()));
        let erro = consultar_toolchain(&dir, shell("printf '\\377'"))
            .unwrap_err()
            .to_string();
        assert!(erro.contains("não é UTF-8"));
        assert!(erro.contains(dir.to_str().unwrap()));
        std::fs::remove_dir_all(dir).unwrap();
    }
    // ---- Chave de cache: estabilidade e invalidação -----------------------

    /// Monta um membro real (src/lib.rs + Cargo.toml) e uma raiz com
    /// Cargo.lock, e devolve `(raiz, membro)` para os testes de chave.
    fn fixture_membro(tag: &str) -> (PathBuf, MembroWorkspace) {
        let raiz = temp_dir(tag);
        escrever(&raiz.join("Cargo.toml"), "[workspace]\nmembers=[\"m\"]\n");
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

        let validade = crate::ValidadeCache {
            chave,
            motivos_bypass: BTreeSet::new(),
        };
        let g = extrair_com_validade(&m, &raiz, &validade, |_| {
            panic!("hit não deve invocar fork")
        })
        .expect("deve ler o cache");
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
        let validade = crate::ValidadeCache {
            chave,
            motivos_bypass: BTreeSet::new(),
        };
        match extrair_com_validade(&m, &raiz, &validade, |_| {
            panic!("cache corrompido deve falhar antes do fork")
        }) {
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

    #[test]
    fn json_invalido_nao_publica_e_bypass_nao_le_nem_publica() {
        let (raiz, m) = fixture_membro("publicacao");
        let mut validade = crate::ValidadeCache {
            chave: "teste".into(),
            motivos_bypass: BTreeSet::new(),
        };
        let destino = dir_cache(&raiz).join("teste.json");
        let r = extrair_com_validade(&m, &raiz, &validade, |_| Ok("{}".into()));
        assert!(matches!(r, Err(ErroWorkspace::Adaptador(_))));
        assert!(!destino.exists());
        assert!(!dir_cache(&raiz).exists());
        escrever(&destino, "cache inválido que não deve ser lido");
        validade.motivos_bypass.insert("build script".into());
        let json = r#"{"crate":"x","nodes":[],"edges":[]}"#;
        extrair_com_validade(&m, &raiz, &validade, |dir| {
            assert_eq!(dir, m.dir);
            Ok(json.into())
        })
        .unwrap();
        assert_eq!(
            std::fs::read_to_string(destino).unwrap(),
            "cache inválido que não deve ser lido"
        );
        std::fs::remove_dir_all(raiz).unwrap();
    }

    #[test]
    fn temporarios_distintos_em_chamadas_simultaneas() {
        let dir = temp_dir("temps_concorrentes");
        let destino = dir.join("grafo.json");
        let barreira = std::sync::Barrier::new(8);
        std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        let (p, f) = criar_temporario(&destino).unwrap();
                        barreira.wait();
                        assert!(p.exists());
                        drop(f);
                        p
                    })
                })
                .collect();
            let paths: BTreeSet<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert_eq!(paths.len(), 8);
            for p in paths {
                std::fs::remove_file(p).unwrap();
            }
        });
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn escritas_concorrentes_publicam_apenas_conteudo_completo() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let dir = temp_dir("atomico_concorrente");
        let destino = dir.join("grafo.json");
        escrever_atomico(&destino, &vec![0; 65536]).unwrap();
        let terminou = AtomicBool::new(false);
        std::thread::scope(|scope| {
            let leitor = scope.spawn(|| {
                while !terminou.load(Ordering::Acquire) {
                    let bytes = std::fs::read(&destino).unwrap();
                    assert_eq!(bytes.len(), 65536);
                    assert!(bytes.iter().all(|b| *b == bytes[0]));
                }
            });
            let handles: Vec<_> = (1..=8)
                .map(|n| {
                    let destino = &destino;
                    scope.spawn(move || escrever_atomico(destino, &vec![n; 65536]).unwrap())
                })
                .collect();
            for h in handles {
                h.join().unwrap();
            }
            terminou.store(true, Ordering::Release);
            leitor.join().unwrap();
        });
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn falha_de_rename_limpa_temporario() {
        let dir = temp_dir("rename_falha");
        let destino = dir.join("destino");
        std::fs::create_dir(&destino).unwrap();
        assert!(escrever_atomico(&destino, b"x").is_err());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        std::fs::remove_dir_all(dir).unwrap();
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
        let tc = versao_toolchain(&membro.dir).unwrap();
        let chave = chave_cache(&membro, &raiz, &tc).unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        let _ = std::fs::remove_file(&destino);

        let g = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("fork deve extrair");
        assert_eq!(g.crate_name, "lente_core");
        let validade = crate::CacheExtracao::default()
            .avaliar(&membro, &raiz, &tc)
            .unwrap();
        assert_eq!(
            destino.exists(),
            validade.motivos_bypass.is_empty(),
            "publicação deve respeitar bypass"
        );
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
        let tc = versao_toolchain(&membro.dir).unwrap();
        let chave = chave_cache(&membro, &raiz, &tc).unwrap();
        let destino = dir_cache(&raiz).join(format!("{}.json", chave));
        let _ = std::fs::remove_file(&destino);

        let g1 = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("miss");
        let g2 = extrair_grafo_cacheado(&membro, &raiz, &tc).expect("hit");
        assert_eq!(g1, g2, "cache deve ser transparente");
    }
}
