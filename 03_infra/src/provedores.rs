//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-provedores.md
//! @prompt-hash 1d08478e
//! @layer L3
//! @updated 2026-09-26
//! Ferramentas opcionais via processo; nenhum algoritmo de extração reside aqui.

use crate::extracao::{CausaProvedor, ErroProvedor, ExtratorGrafo, SolicitacaoExtracao};
use lente_core::domain::extracao::{ExtracaoGrafo, IdentidadeProvedor};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfiguracaoFerramenta {
    pub executavel: Option<PathBuf>,
    pub helper: Option<PathBuf>,
    pub verificador: PathBuf,
    pub timeout_ms: u64,
    #[serde(default)]
    pub permitir_metadados_sem_vinculo: bool,
    #[serde(default)]
    pub incluir_ambiente: bool,
    pub parametros: Value,
}

#[derive(Debug, Clone)]
pub struct PedidoFerramenta {
    pub provedor: String,
    pub configuracao: ConfiguracaoFerramenta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroFerramenta {
    Parametros(String),
    Indisponivel(String),
    Execucao(String),
    Timeout(String),
    SaidaInvalida(String),
}
impl ErroFerramenta {
    pub fn categoria(&self) -> &'static str {
        match self {
            Self::Parametros(_) => "parametros",
            Self::Indisponivel(_) => "provedor_indisponivel",
            Self::Execucao(_) => "falha_execucao",
            Self::Timeout(_) => "timeout",
            Self::SaidaInvalida(_) => "saida_invalida",
        }
    }
}
impl std::fmt::Display for ErroFerramenta {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Parametros(s)
            | Self::Indisponivel(s)
            | Self::Execucao(s)
            | Self::Timeout(s)
            | Self::SaidaInvalida(s) => s,
        };
        write!(f, "{}: {}", self.categoria(), s)
    }
}
impl std::error::Error for ErroFerramenta {}
fn invalido(e: impl std::fmt::Display) -> ErroFerramenta {
    ErroFerramenta::SaidaInvalida(e.to_string())
}
fn io(e: impl std::fmt::Display) -> ErroFerramenta {
    ErroFerramenta::Execucao(e.to_string())
}
fn param(e: impl std::fmt::Display) -> ErroFerramenta {
    ErroFerramenta::Parametros(e.to_string())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Python {
    roots: Vec<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TypeScript {
    project: PathBuf,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Zig {
    root: PathBuf,
    modules: PathBuf,
    name: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct C {
    database: PathBuf,
    root: PathBuf,
    variante: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Cpp {
    database: PathBuf,
    root: PathBuf,
    libclang: PathBuf,
    configuration: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Go {
    project: PathBuf,
    goos: String,
    goarch: String,
    tests: String,
    work: String,
    tags: Option<String>,
    #[serde(default)]
    patterns: Vec<String>,
    module_cache: Option<PathBuf>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Pacote {
    diretorio: PathBuf,
    manifest_sha256: Option<String>,
}

enum Parametros {
    Python(Python),
    TypeScript(TypeScript),
    Zig(Zig),
    C(C),
    Cpp(Cpp),
    Go(Go),
    Pacote(Pacote),
}
fn ler<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, ErroFerramenta> {
    serde_json::from_value(v.clone()).map_err(param)
}
fn caminho(p: &Path, diretorio: bool) -> Result<(), ErroFerramenta> {
    if !p.is_absolute() || !(if diretorio { p.is_dir() } else { p.is_file() }) {
        return Err(param(format!(
            "caminho deve ser absoluto e {} existente: {}",
            if diretorio { "diretório" } else { "arquivo" },
            p.display()
        )));
    }
    Ok(())
}
fn ferramenta(p: &Path) -> Result<(), ErroFerramenta> {
    if !p.is_absolute() {
        return Err(param(format!(
            "ferramenta exige caminho absoluto: {}",
            p.display()
        )));
    }
    if !p.is_file() {
        return Err(ErroFerramenta::Indisponivel(format!(
            "{} ausente; prepare a ferramenta explicitamente conforme docs/adaptadores.md",
            p.display()
        )));
    }
    Ok(())
}
fn programa(p: &Path) -> Result<(), ErroFerramenta> {
    ferramenta(p)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(p).map_err(io)?.permissions().mode() & 0o111 == 0 {
            return Err(ErroFerramenta::Indisponivel(format!(
                "{} não é executável; prepare a ferramenta explicitamente",
                p.display()
            )));
        }
    }
    Ok(())
}
fn token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
fn digest(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

impl PedidoFerramenta {
    pub fn ler(provedor: &str, configuracao: &str) -> Result<Self, ErroFerramenta> {
        let pedido = Self {
            provedor: provedor.into(),
            configuracao: serde_json::from_str(configuracao).map_err(param)?,
        };
        pedido.validar()?;
        Ok(pedido)
    }
    fn validar(&self) -> Result<Parametros, ErroFerramenta> {
        let c = &self.configuracao;
        if c.timeout_ms == 0 || c.timeout_ms > 3_600_000 {
            return Err(param("timeout_ms deve estar entre 1 e 3600000"));
        }
        let v = &c.parametros;
        // Sintaxe e compatibilidade são verificadas antes de disponibilidade/I/O de execução.
        let p = match self.provedor.as_str() {
            "python" => Parametros::Python(ler(v)?),
            "typescript" => Parametros::TypeScript(ler(v)?),
            "zig" => Parametros::Zig(ler(v)?),
            "c" => Parametros::C(ler(v)?),
            "cpp" => Parametros::Cpp(ler(v)?),
            "go" => Parametros::Go(ler(v)?),
            "pacote-analise" => Parametros::Pacote(ler(v)?),
            _ => return Err(param("provedor desconhecido")),
        };
        let usa_helper = matches!(
            p,
            Parametros::Python(_)
                | Parametros::TypeScript(_)
                | Parametros::Zig(_)
                | Parametros::Cpp(_)
        );
        if usa_helper != c.helper.is_some() {
            return Err(param(
                "helper obrigatório somente para python/typescript/zig/cpp",
            ));
        }
        if matches!(p, Parametros::Pacote(_)) == c.executavel.is_some() {
            return Err(param(
                "executavel obrigatório para adaptadores e proibido para pacote-analise",
            ));
        }
        match &p {
            Parametros::Python(p) => {
                if p.roots.is_empty() {
                    return Err(param("roots não pode ser vazio"));
                }
                for r in &p.roots {
                    caminho(r, true)?;
                }
            }
            Parametros::TypeScript(p) => {
                caminho(&p.project, false)?;
                if p.name.is_empty() || p.name.starts_with('-') {
                    return Err(param("name vazio ou começando com hífen"));
                }
            }
            Parametros::Zig(p) => {
                caminho(&p.root, false)?;
                caminho(&p.modules, false)?;
                if p.name.is_empty() || p.name.starts_with('-') {
                    return Err(param("name vazio ou começando com hífen"));
                }
            }
            Parametros::C(p) => {
                caminho(&p.database, false)?;
                caminho(&p.root, true)?;
                if p.database
                    .file_name()
                    .is_none_or(|n| n != "compile_commands.json")
                {
                    return Err(param("C exige arquivo compile_commands.json"));
                }
            }
            Parametros::Cpp(p) => {
                caminho(&p.database, false)?;
                caminho(&p.root, true)?;
                ferramenta(&p.libclang)?;
                if p.configuration.as_ref().is_some_and(|s| !digest(s)) {
                    return Err(param("configuration exige SHA-256"));
                }
            }
            Parametros::Go(p) => {
                caminho(&p.project, true)?;
                if !token(&p.goos)
                    || !token(&p.goarch)
                    || !["include", "exclude"].contains(&p.tests.as_str())
                {
                    return Err(param("goos/goarch devem ser nomes e tests include|exclude"));
                }
                if !["auto", "off"].contains(&p.work.as_str()) {
                    caminho(Path::new(&p.work), false)?;
                }
                if p.patterns
                    .iter()
                    .any(|s| s.is_empty() || s.starts_with('-'))
                {
                    return Err(param("pattern vazio ou opção Go não permitido"));
                }
                if let Some(p) = &p.module_cache {
                    caminho(p, true)?;
                }
                if p.tags
                    .as_ref()
                    .is_some_and(|s| !s.is_empty() && !s.split(',').all(token))
                {
                    return Err(param(
                        "tags deve ser lista de identificadores separada por vírgula",
                    ));
                }
            }
            Parametros::Pacote(p) => {
                caminho(&p.diretorio, true)?;
                if p.manifest_sha256.as_ref().is_some_and(|s| !digest(s)) {
                    return Err(param("manifest_sha256 inválido"));
                }
            }
        }
        programa(&c.verificador)?;
        if let Some(exe) = &c.executavel {
            programa(exe)?;
        }
        if let Some(h) = &c.helper {
            ferramenta(h)?;
        }
        if let Parametros::TypeScript(_) = p {
            ferramenta(
                &c.helper
                    .as_ref()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("node_modules/typescript/lib/typescript.js"),
            )?;
        }
        if let Parametros::Go(_) = p {
            let instalado = std::env::var_os("PATH")
                .is_some_and(|paths| std::env::split_paths(&paths).any(|d| d.join("go").is_file()));
            if !instalado {
                return Err(ErroFerramenta::Indisponivel("Go ausente do PATH; instale explicitamente uma versão compatível, sem GOTOOLCHAIN automático".into()));
            }
        }
        if let Parametros::Zig(_) = p {
            ferramenta(
                &c.helper
                    .as_ref()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("bin/parser"),
            )?;
        }
        #[cfg(not(unix))]
        return Err(ErroFerramenta::Indisponivel(
            "execução com grupos de processos requer Unix".into(),
        ));
        #[cfg(unix)]
        programa(Path::new("/bin/kill"))?;
        Ok(p)
    }
}

static SEQUENCIA: AtomicU64 = AtomicU64::new(0);
struct Temporario(PathBuf);
impl Temporario {
    fn novo() -> Result<Self, ErroFerramenta> {
        loop {
            let p = std::env::temp_dir().join(format!(
                "lente-provedor-{}-{}",
                std::process::id(),
                SEQUENCIA.fetch_add(1, Ordering::Relaxed)
            ));
            let mut b = fs::DirBuilder::new();
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                b.mode(0o700);
            }
            match b.create(&p) {
                Ok(()) => return Ok(Self(p)),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(io(e)),
            }
        }
    }
}
impl Drop for Temporario {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
struct Processo(std::process::Child);
impl Drop for Processo {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            let _ = Command::new("/bin/kill")
                .args(["-KILL", "--", &format!("-{}", self.0.id())])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
struct SaidaProcesso {
    codigo: Option<i32>,
    stdout: String,
    stderr: String,
}
fn executar(
    exe: &Path,
    args: &[OsString],
    dir: &Path,
    prazo: Instant,
) -> Result<SaidaProcesso, ErroFerramenta> {
    if Instant::now() >= prazo {
        return Err(ErroFerramenta::Timeout(exe.display().to_string()));
    }
    let out = dir.join("stdout");
    let err = dir.join("stderr");
    let mut cmd = Command::new(exe);
    cmd.args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(fs::File::create(&out).map_err(io)?)
        .stderr(fs::File::create(&err).map_err(io)?)
        .env("RUSTUP_AUTO_INSTALL", "0")
        .env("CARGO_NET_OFFLINE", "true")
        .env("GOTOOLCHAIN", "local")
        .env("PYTHONDONTWRITEBYTECODE", "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut processo = Processo(cmd.spawn().map_err(|e| {
        if matches!(
            e.kind(),
            std::io::ErrorKind::NotFound | std::io::ErrorKind::PermissionDenied
        ) {
            ErroFerramenta::Indisponivel(format!(
                "{}: {}; prepare/verifique a ferramenta explicitamente",
                exe.display(),
                e
            ))
        } else {
            io(e)
        }
    })?);
    let status = loop {
        if let Some(s) = processo.0.try_wait().map_err(io)? {
            break s;
        }
        if Instant::now() >= prazo {
            return Err(ErroFerramenta::Timeout(format!(
                "{} excedeu o prazo; grupo encerrado",
                exe.display()
            )));
        }
        std::thread::sleep(
            Duration::from_millis(10).min(prazo.saturating_duration_since(Instant::now())),
        );
    };
    drop(processo); // Também encerra descendentes que sobrevivam ao líder.
    Ok(SaidaProcesso {
        codigo: status.code(),
        stdout: fs::read_to_string(out).map_err(invalido)?,
        stderr: fs::read_to_string(err).map_err(invalido)?,
    })
}
fn exigir_sucesso(s: SaidaProcesso, contexto: &str) -> Result<SaidaProcesso, ErroFerramenta> {
    if s.codigo != Some(0) {
        return Err(ErroFerramenta::Execucao(format!(
            "{contexto}: status {:?}: {}",
            s.codigo, s.stderr
        )));
    }
    Ok(s)
}

/// Mesma infraestrutura de processo, sem protocolo de extração ou empacotamento.
pub(crate) fn executar_artefato(
    exe: &Path,
    args: &[OsString],
    timeout_ms: u64,
    codigos: &[i32],
) -> Result<(i32, Value, String), ErroFerramenta> {
    if !(1..=3_600_000).contains(&timeout_ms) {
        return Err(param("timeout_ms deve estar entre 1 e 3600000"));
    }
    #[cfg(not(unix))]
    return Err(param("execução com grupos de processos requer Unix"));
    programa(exe)?;
    programa(Path::new("/bin/kill"))?;
    let dir = Temporario::novo()?;
    let s = executar(
        exe,
        args,
        &dir.0,
        Instant::now() + Duration::from_millis(timeout_ms),
    )?;
    let codigo = s.codigo.filter(|c| codigos.contains(c)).ok_or_else(|| {
        ErroFerramenta::Execucao(format!("análise: status {:?}: {}", s.codigo, s.stderr))
    })?;
    let json: Value = serde_json::from_str(&s.stdout).map_err(invalido)?;
    if !json.is_object() {
        return Err(invalido("análise deve devolver objeto JSON"));
    }
    Ok((codigo, json, s.stderr))
}
fn json_arquivo(p: &Path) -> Result<Value, ErroFerramenta> {
    serde_json::from_str(&fs::read_to_string(p).map_err(invalido)?).map_err(invalido)
}

/// Contexto de transporte e metadados integrais; summary vem do verificador oficial.
#[derive(Debug, Clone)]
pub struct ContextoFerramenta {
    pub dados: Value,
}
#[derive(Default)]
pub struct ExtratorFerramenta {
    pub contexto: Option<ContextoFerramenta>,
}
impl ExtratorGrafo for ExtratorFerramenta {
    fn extrair(&mut self, s: SolicitacaoExtracao<'_>) -> Result<ExtracaoGrafo, ErroProvedor> {
        self.contexto = None;
        let SolicitacaoExtracao::Ferramenta(p) = s else {
            return Err(ErroProvedor {
                provedor: IdentidadeProvedor::Json,
                contexto: "ferramenta externa".into(),
                causa: CausaProvedor::SolicitacaoIncompativel,
            });
        };
        let r = self.extrair_pedido(p);
        r.map_err(|e| ErroProvedor {
            provedor: IdentidadeProvedor::Json,
            contexto: format!("ferramenta {}", p.provedor),
            causa: CausaProvedor::Ferramenta(e),
        })
    }
}
impl ExtratorFerramenta {
    fn extrair_pedido(
        &mut self,
        pedido: &PedidoFerramenta,
    ) -> Result<ExtracaoGrafo, ErroFerramenta> {
        let params = pedido.validar()?;
        let c = &pedido.configuracao;
        let prazo = Instant::now() + Duration::from_millis(c.timeout_ms);
        let temp = Temporario::novo()?;
        let output = temp.0.join("saida");
        fs::create_dir(&output).map_err(io)?;
        let mut args: Vec<OsString> = vec![];
        if let Some(h) = &c.helper {
            if pedido.provedor != "typescript" {
                args.push("-B".into());
            }
            // C++ importa seu módulo irmão; -I excluiria esse módulo do sys.path.
            if pedido.provedor == "python" {
                args.push("-I".into());
            }
            args.push(h.as_os_str().to_owned());
        }
        let graph = output.join("grafo.json");
        let metadata = output.join("metadados.json");
        let mut par = (graph.clone(), metadata.clone());
        let mut legado = Value::Null;
        let mut exit = None;
        let mut stderr = String::new();
        let mut add = |k: &str, v: &std::ffi::OsStr| {
            args.push(k.into());
            args.push(v.to_owned());
        };
        match &params {
            Parametros::Python(p) => {
                for r in &p.roots {
                    add("--root", r.as_os_str());
                }
                add("--graph", graph.as_os_str());
                add("--metadata", metadata.as_os_str());
            }
            Parametros::TypeScript(p) => {
                add("--project", p.project.as_os_str());
                add("--name", p.name.as_ref());
                add("--out", output.as_os_str());
            }
            Parametros::Zig(p) => {
                add("--root", p.root.as_os_str());
                add("--modules", p.modules.as_os_str());
                add("--name", p.name.as_ref());
                add("--out", output.as_os_str());
            }
            Parametros::C(p) => {
                args.extend([
                    p.database.as_os_str().to_owned(),
                    p.root.as_os_str().to_owned(),
                    output.as_os_str().to_owned(),
                ]);
                par = (
                    output.join(format!("graph-{:04}.json", p.variante)),
                    output.join(format!("metadata-{:04}.json", p.variante)),
                );
            }
            Parametros::Cpp(p) => {
                add("--database", p.database.as_os_str());
                add("--root", p.root.as_os_str());
                add("--libclang", p.libclang.as_os_str());
                if let Some(x) = &p.configuration {
                    add("--configuration", x.as_ref());
                }
                add("--graph", graph.as_os_str());
                add("--metadata", metadata.as_os_str());
            }
            Parametros::Go(p) => {
                add("--project", p.project.as_os_str());
                add("--goos", p.goos.as_ref());
                add("--goarch", p.goarch.as_ref());
                add("--tests", p.tests.as_ref());
                add("--work", p.work.as_ref());
                add("--out", output.as_os_str());
                if let Some(x) = &p.tags {
                    add("--tags", x.as_ref());
                }
                for x in &p.patterns {
                    add("--pattern", x.as_ref());
                }
                if let Some(x) = &p.module_cache {
                    add("--module-cache", x.as_os_str());
                }
            }
            Parametros::Pacote(_) => {}
        }
        let pacote = match &params {
            Parametros::Pacote(p) => p.diretorio.clone(),
            _ => {
                let s = executar(c.executavel.as_ref().unwrap(), &args, &temp.0, prazo)?;
                if s.codigo == Some(127) {
                    return Err(ErroFerramenta::Indisponivel(format!(
                        "{}: carregador/dependência indisponível: {}",
                        pedido.provedor, s.stderr
                    )));
                }
                let aceito = match pedido.provedor.as_str() {
                    "python" | "cpp" => [Some(0), Some(1)].contains(&s.codigo),
                    "c" | "go" => [Some(0), Some(2)].contains(&s.codigo),
                    _ => s.codigo == Some(0),
                };
                if !aceito || !par.0.is_file() || !par.1.is_file() {
                    return Err(ErroFerramenta::Execucao(format!(
                        "{}: status {:?}, arquivos esperados ausentes ou falha: {}",
                        pedido.provedor, s.codigo, s.stderr
                    )));
                }
                exit = s.codigo;
                stderr = s.stderr;
                if let Parametros::C(p) = &params {
                    legado = json_arquivo(&output.join("manifest.json"))?;
                    if legado["variants"]
                        .as_array()
                        .and_then(|v| v.get(p.variante))
                        .is_none()
                    {
                        return Err(param("variante C fora do manifesto"));
                    }
                }
                let pacote = temp.0.join("pacote");
                let mut args = vec![
                    "create".into(),
                    "--graph".into(),
                    par.0.as_os_str().to_owned(),
                    "--metadata".into(),
                    par.1.as_os_str().to_owned(),
                    "--out".into(),
                    pacote.as_os_str().to_owned(),
                ];
                if c.permitir_metadados_sem_vinculo {
                    args.push("--allow-unbound-metadata".into());
                }
                if c.incluir_ambiente {
                    args.push("--include-environment-values".into());
                }
                exigir_sucesso(
                    executar(&c.verificador, &args, &temp.0, prazo)?,
                    "criação do pacote",
                )?;
                pacote
            }
        };
        let mut args = vec!["verify".into(), pacote.as_os_str().to_owned()];
        if !c.permitir_metadados_sem_vinculo {
            args.push("--require-bound".into());
        }
        if let Parametros::Pacote(p) = &params
            && let Some(d) = &p.manifest_sha256
        {
            args.extend(["--manifest-sha256".into(), d.into()]);
        }
        let verificacao = executar(&c.verificador, &args, &temp.0, prazo)?;
        if verificacao.codigo != Some(0) {
            return Err(invalido(format!("pacote recusado: {}", verificacao.stderr)));
        }
        let verificacao: Value = serde_json::from_str(&verificacao.stdout).map_err(invalido)?;
        if verificacao["integrity"] != "verified" {
            return Err(invalido("verificador não confirmou integridade"));
        }
        let texto = fs::read_to_string(pacote.join("graph.json")).map_err(invalido)?;
        let grafo = crate::desserializar_grafo(&texto).map_err(invalido)?;
        let mut paths = BTreeSet::new();
        if grafo.nodes.iter().any(|n| !paths.insert(n.path.as_str())) {
            return Err(invalido(
                "paths colidentes no grafo externo; resolução Rust não se aplica",
            ));
        }
        let metadados = if pacote.join("metadata.json").exists() {
            json_arquivo(&pacote.join("metadata.json"))?
        } else {
            Value::Null
        };
        let esperado = match pedido.provedor.as_str() {
            "python" => Some("experimental-python-ast"),
            "typescript" => Some("typescript-compiler-api-experimental"),
            "zig" => Some("zig-ast-experimental"),
            "c" => Some("libclang"),
            "cpp" => Some("experimental-libclang-cpp"),
            "go" => Some("experimental-go-list"),
            _ => None,
        };
        if esperado.is_some_and(|p| metadados["provider"] != p) {
            return Err(invalido(
                "metadados não correspondem ao provedor solicitado",
            ));
        }
        if matches!(
            verificacao
                .pointer("/summary/execution/value")
                .and_then(Value::as_str),
            Some("failed" | "error")
        ) {
            return Err(ErroFerramenta::Execucao(format!(
                "provedor declarou falha: {}",
                metadados
            )));
        }
        self.contexto = Some(ContextoFerramenta {
            dados: json!({ "selecao": pedido.provedor, "estado_integracao": "experimental-opcional", "transporte": "json", "cache": false, "verificacao": verificacao, "metadados_originais": metadados, "manifesto_legado": legado, "codigo_saida_adaptador": exit, "stderr_adaptador": stderr }),
        });
        Ok(ExtracaoGrafo {
            grafo,
            provedor: IdentidadeProvedor::Json,
            diagnosticos: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn pedido(p: &str, parametros: Value) -> PedidoFerramenta {
        PedidoFerramenta {
            provedor: p.into(),
            configuracao: ConfiguracaoFerramenta {
                executavel: Some("/ausente/lente-tool".into()),
                helper: Some("/ausente/helper.py".into()),
                verificador: "/ausente/lente-pacote-analise".into(),
                timeout_ms: 1000,
                permitir_metadados_sem_vinculo: false,
                incluir_ambiente: false,
                parametros,
            },
        }
    }
    #[test]
    fn incompatibilidade_antes_de_iniciar_ferramenta() {
        let p = pedido("python", json!({"roots":[], "goos":"linux"}));
        assert!(matches!(p.validar(), Err(ErroFerramenta::Parametros(_))));
        let p = pedido("python", json!({"roots":[]}));
        assert!(matches!(p.validar(), Err(ErroFerramenta::Parametros(_))));
        let p = pedido(
            "c",
            json!({"database":"/arquivo", "root":"/", "variante":0}),
        );
        assert!(matches!(p.validar(), Err(ErroFerramenta::Parametros(_))));
    }
    #[test]
    fn provedor_ausente_e_acionavel() {
        let p = pedido("python", json!({"roots":[std::env::temp_dir()]}));
        let e = p.validar().err().unwrap();
        assert!(matches!(e, ErroFerramenta::Indisponivel(_)));
        assert!(e.to_string().contains("explicitamente"));
    }
    #[test]
    fn temporarios_concorrentes_sao_exclusivos_e_removidos() {
        let handles: Vec<_> = (0..8)
            .map(|_| std::thread::spawn(|| Temporario::novo().unwrap()))
            .collect();
        let dirs: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        let paths: BTreeSet<_> = dirs.iter().map(|t| t.0.clone()).collect();
        assert_eq!(paths.len(), 8);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for p in &paths {
                assert_eq!(fs::metadata(p).unwrap().permissions().mode() & 0o777, 0o700);
            }
        }
        drop(dirs);
        assert!(paths.iter().all(|p| !p.exists()));
    }
    #[test]
    #[ignore = "helper interno de subprocesso; executado pelo teste de timeout"]
    fn helper_pai() {
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "provedores::tests::helper_filho"])
            .spawn()
            .unwrap();
        fs::write("filho.pid", child.id().to_string()).unwrap();
        let _ = child.wait();
    }
    #[test]
    #[ignore = "helper interno de subprocesso; executado pelo teste de timeout"]
    fn helper_filho() {
        std::thread::sleep(Duration::from_secs(60));
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn timeout_encerra_lider_e_descendente() {
        let temp = Temporario::novo().unwrap();
        let args = ["--ignored", "--exact", "provedores::tests::helper_pai"].map(OsString::from);
        let r = executar(
            &std::env::current_exe().unwrap(),
            &args,
            &temp.0,
            Instant::now() + Duration::from_secs(1),
        );
        assert!(matches!(r, Err(ErroFerramenta::Timeout(_))));
        let pid = fs::read_to_string(temp.0.join("filho.pid")).unwrap();
        // Linux pode reter um zombie até o reaper; zombie não está executando.
        for _ in 0..100 {
            let status = fs::read_to_string(format!("/proc/{pid}/stat"));
            if status.as_ref().map_or(true, |s| {
                s.split(") ").nth(1).is_some_and(|s| s.starts_with('Z'))
            }) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("descendente {pid} continua executando após timeout");
    }
    #[test]
    fn pacote_invalido_nao_produz_grafo() {
        let temp = Temporario::novo().unwrap();
        let p = PedidoFerramenta {
            provedor: "pacote-analise".into(),
            configuracao: ConfiguracaoFerramenta {
                executavel: None,
                helper: None,
                verificador: "/bin/false".into(),
                timeout_ms: 2000,
                permitir_metadados_sem_vinculo: false,
                incluir_ambiente: false,
                parametros: json!({"diretorio":temp.0}),
            },
        };
        let mut extrator = ExtratorFerramenta::default();
        assert!(matches!(
            extrator.extrair_pedido(&p),
            Err(ErroFerramenta::SaidaInvalida(_))
        ));
        assert!(extrator.contexto.is_none());
    }
}
