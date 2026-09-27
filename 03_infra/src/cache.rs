//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-cache-validade.md
//! @prompt-hash 1b495c99
//! @layer L3
//! @updated 2026-09-26
//! Cache conservador: fingerprint não equivale a prova de reutilização.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::{ErroWorkspace, MembroWorkspace};

type Resultado<T> = Result<T, ErroWorkspace>;

/// Decisão explícita: uma chave só pode ser usada quando não há motivos de bypass.
#[derive(Debug, Clone)]
pub struct ValidadeCache {
    pub chave: String,
    pub motivos_bypass: BTreeSet<String>,
}

/// Leituras compartilhadas durante uma rodada, nunca entre rodadas.
/// Não alterar entradas durante seu uso. Toolchains instalados são considerados
/// imutáveis para sua identificação; ver contrato L0 para o domínio de validade.
pub struct CacheExtracao {
    arquivos: BTreeMap<PathBuf, Option<Arc<Vec<u8>>>>,
    membros: BTreeMap<PathBuf, Vec<PathBuf>>,
    manifestos: BTreeMap<PathBuf, toml::Value>,
    digests: BTreeMap<PathBuf, Vec<u8>>,
    arvores: BTreeMap<PathBuf, (Vec<u8>, BTreeSet<String>)>,
    ambiente: BTreeMap<OsString, OsString>,
    fork: Option<(Vec<u8>, bool)>,
}

impl Default for CacheExtracao {
    fn default() -> Self {
        Self {
            arquivos: BTreeMap::new(),
            membros: BTreeMap::new(),
            manifestos: BTreeMap::new(),
            digests: BTreeMap::new(),
            arvores: BTreeMap::new(),
            ambiente: std::env::vars_os().collect(),
            fork: None,
        }
    }
}

fn io(p: &Path, e: std::io::Error) -> ErroWorkspace {
    ErroWorkspace::Io(std::io::Error::new(
        e.kind(),
        format!("{}: {e}", p.display()),
    ))
}

fn canonico(p: &Path) -> Resultado<PathBuf> {
    p.canonicalize().map_err(|e| io(p, e))
}

// Tag e valor são ambos delimitados por tamanho. Nem NUL nem bytes de caminho
// podem se confundir com fronteiras de componentes.
fn campo(h: &mut Sha256, tag: &[u8], bytes: &[u8]) {
    h.update((tag.len() as u64).to_le_bytes());
    h.update(tag);
    h.update((bytes.len() as u64).to_le_bytes());
    h.update(bytes);
}

impl CacheExtracao {
    fn ler(&mut self, p: &Path) -> Resultado<Option<Arc<Vec<u8>>>> {
        if let Some(b) = self.arquivos.get(p) {
            return Ok(b.clone());
        }
        let b = match std::fs::read(p) {
            Ok(b) => Some(Arc::new(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(io(p, e)),
        };
        self.arquivos.insert(p.to_owned(), b.clone());
        Ok(b)
    }

    fn arquivo(&mut self, h: &mut Sha256, p: &Path, obrigatorio: bool) -> Resultado<()> {
        campo(h, b"arquivo", p.as_os_str().as_encoded_bytes());
        match self.ler(p)? {
            Some(b) => {
                let digest = self
                    .digests
                    .entry(p.to_owned())
                    .or_insert_with(|| Sha256::digest(b.as_slice()).to_vec());
                campo(h, b"presente", digest);
            }
            None if obrigatorio => return Err(io(p, std::io::ErrorKind::NotFound.into())),
            None => campo(h, b"ausente", b""),
        }
        Ok(())
    }

    fn manifesto(&mut self, p: &Path) -> Resultado<toml::Value> {
        if let Some(v) = self.manifestos.get(p) {
            return Ok(v.clone());
        }
        let b = self
            .ler(p)?
            .ok_or_else(|| io(p, std::io::ErrorKind::NotFound.into()))?;
        let s = std::str::from_utf8(&b)
            .map_err(|e| ErroWorkspace::Manifesto(format!("{}: {e}", p.display())))?;
        let v: toml::Value = toml::from_str(s)
            .map_err(|e| ErroWorkspace::Manifesto(format!("{}: {e}", p.display())))?;
        self.manifestos.insert(p.to_owned(), v.clone());
        Ok(v)
    }

    fn arvore(&mut self, dir: &Path) -> Resultado<(Vec<u8>, BTreeSet<String>)> {
        if let Some(a) = self.arvores.get(dir) {
            return Ok(a.clone());
        }
        let mut h = Sha256::new();
        let mut motivos = BTreeSet::new();
        let mut entradas = std::fs::read_dir(dir)
            .map_err(|e| io(dir, e))?
            .map(|e| e.map(|e| e.path()).map_err(|e| io(dir, e)))
            .collect::<Resultado<Vec<_>>>()?;
        entradas.sort();
        for p in entradas {
            if p.file_name().is_some_and(|n| n == ".git" || n == "target") {
                continue;
            }
            let meta = std::fs::symlink_metadata(&p).map_err(|e| io(&p, e))?;
            campo(&mut h, b"caminho", p.as_os_str().as_encoded_bytes());
            if meta.file_type().is_symlink() {
                motivos.insert(format!("link simbólico: {}", p.display()));
                let destino = std::fs::read_link(&p).map_err(|e| io(&p, e))?;
                campo(&mut h, b"link", destino.as_os_str().as_encoded_bytes());
            } else if meta.is_dir() {
                let (digest, m) = self.arvore(&p)?;
                campo(&mut h, b"diretorio", &digest);
                motivos.extend(m);
            } else if meta.is_file() {
                self.arquivo(&mut h, &p, true)?;
                if p.extension().is_some_and(|e| e == "rs") {
                    let b = self.ler(&p)?.expect("arquivo obrigatório já lido");
                    // Sobreaproximação intencional (inclusive comentários): sem
                    // proc macros no fechamento, estes tokens podem abrir entradas
                    // externas ou construir nomes de macros dinamicamente.
                    for token in ["include", "env", "path", "macro_rules", "concat"] {
                        if b.windows(token.len()).any(|w| w == token.as_bytes()) {
                            motivos.insert(format!("fonte com {token}: {}", p.display()));
                        }
                    }
                }
            } else {
                motivos.insert(format!("arquivo especial: {}", p.display()));
            }
        }
        let a = (h.finalize().to_vec(), motivos);
        self.arvores.insert(dir.to_owned(), a.clone());
        Ok(a)
    }

    fn configuracao(
        &mut self,
        h: &mut Sha256,
        dir: &Path,
        motivos: &mut BTreeSet<String>,
    ) -> Resultado<()> {
        for ancestral in dir.ancestors() {
            for nome in [
                ".cargo/config",
                ".cargo/config.toml",
                "rust-toolchain",
                "rust-toolchain.toml",
            ] {
                let p = ancestral.join(nome);
                self.arquivo(h, &p, false)?;
                if nome.starts_with(".cargo/") && self.ler(&p)?.is_some() {
                    motivos.insert(format!("configuração Cargo: {}", p.display()));
                }
            }
        }
        Ok(())
    }

    fn identidade_fork(&mut self) -> Resultado<(Vec<u8>, bool)> {
        if let Some(f) = &self.fork {
            return Ok(f.clone());
        }
        let mut candidatos = BTreeSet::new();
        let mut h = Sha256::new();
        let mut dirs = Vec::new();
        if let Some(home) = self.cargo_home() {
            dirs.push(home.join("bin"));
        }
        if let Some(path) = self.ambiente.get(&OsString::from("PATH")) {
            dirs.extend(std::env::split_paths(path));
        }
        let mut confiavel = true;
        for dir in dirs {
            if !dir.is_absolute() {
                confiavel = false;
                continue;
            }
            let p = dir.join(format!("cargo-modules{}", std::env::consts::EXE_SUFFIX));
            match std::fs::metadata(&p) {
                Ok(m) if m.is_file() => {
                    candidatos.insert(canonico(&p)?);
                }
                Ok(_) => {
                    confiavel = false;
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(io(&p, e)),
            }
        }
        // Não presumir precedência se há múltiplos executáveis distintos.
        confiavel &= candidatos.len() == 1;
        for p in candidatos {
            self.arquivo(&mut h, &p, true)?;
        }
        let f = (h.finalize().to_vec(), confiavel);
        self.fork = Some(f.clone());
        Ok(f)
    }

    fn cargo_home(&self) -> Option<PathBuf> {
        self.ambiente
            .get(&OsString::from("CARGO_HOME"))
            .map(PathBuf::from)
            .or_else(|| {
                self.ambiente
                    .get(&OsString::from("HOME"))
                    .map(|p| PathBuf::from(p).join(".cargo"))
            })
    }

    /// Calcula o digest e explica condições que proíbem ler/publicar cache.
    /// `toolchain` deve vir de `versao_toolchain(&membro.dir)` nesta rodada.
    pub fn avaliar(
        &mut self,
        membro: &MembroWorkspace,
        raiz: &Path,
        toolchain: &str,
    ) -> Resultado<ValidadeCache> {
        let raiz = canonico(raiz)?;
        let dir = canonico(&membro.dir)?;
        let mut h = Sha256::new();
        let mut motivos = BTreeSet::new();
        campo(&mut h, b"formato", b"lente-cache-v2");
        campo(
            &mut h,
            b"extracao",
            b"export-json --sysroot --compact; metadata --no-deps; lib-preferida",
        );
        campo(&mut h, b"raiz", raiz.as_os_str().as_encoded_bytes());
        campo(&mut h, b"membro", dir.as_os_str().as_encoded_bytes());
        campo(&mut h, b"pacote", membro.nome.as_bytes());
        campo(&mut h, b"toolchain", toolchain.as_bytes());
        for (k, v) in &self.ambiente {
            campo(&mut h, b"env-nome", k.as_encoded_bytes());
            campo(&mut h, b"env-valor", v.as_encoded_bytes());
            let k = k.to_string_lossy();
            if [
                "CARGO_BUILD_",
                "CARGO_TARGET_",
                "CARGO_PROFILE_",
                "CARGO_ALIAS_",
                "CARGO_SOURCE_",
                "CARGO_REGISTRIES_",
                "CARGO_UNSTABLE_",
                "CARGO_ENCODED_RUSTFLAGS",
            ]
            .iter()
            .any(|p| k.starts_with(p))
                || matches!(
                    k.as_ref(),
                    "RUSTFLAGS"
                        | "RUSTDOCFLAGS"
                        | "RUSTC"
                        | "RUSTC_WRAPPER"
                        | "RUSTC_WORKSPACE_WRAPPER"
                        | "RUST_TARGET_PATH"
                )
            {
                motivos.insert(format!("ambiente de compilação: {k}"));
            }
        }
        if let Some(home) = self.cargo_home() {
            if !home.is_absolute() {
                motivos.insert("CARGO_HOME relativo".into());
            }
            for nome in ["config", "config.toml"] {
                let p = home.join(nome);
                self.arquivo(&mut h, &p, false)?;
                if self.ler(&p)?.is_some() {
                    motivos.insert(format!("configuração Cargo: {}", p.display()));
                }
            }
        }
        if self.cargo_home().is_none() {
            motivos.insert("CARGO_HOME não resolvido: configuração global não rastreável".into());
        }
        self.arquivo(&mut h, &raiz.join("Cargo.toml"), true)?;
        self.arquivo(&mut h, &raiz.join("Cargo.lock"), false)?;
        // Outros membros podem alterar resolução/unificação de features. A árvore
        // da raiz é compartilhada; não tenta reimplementar o resolver Cargo.
        let (raiz_digest, riscos_raiz) = self.arvore(&raiz)?;
        motivos.extend(
            riscos_raiz
                .into_iter()
                .filter(|m| m.starts_with("link simbólico:") || m.starts_with("arquivo especial:")),
        );
        campo(&mut h, b"arvore-workspace", &raiz_digest);
        let raiz_toml = self.manifesto(&raiz.join("Cargo.toml"))?;
        let mut pendentes = BTreeSet::from([dir]);
        // O fork carrega o workspace: entradas de outros membros podem participar
        // da resolução de features. Fechar todos os membros é conservador.
        if let Some(ws) = raiz_toml.get("workspace") {
            if ws
                .get("members")
                .and_then(|v| v.as_array())
                .is_some_and(|patterns| {
                    patterns.iter().any(|v| {
                        v.as_str().is_none_or(|p| {
                            p.contains(['?', '[', ']'])
                                || p.split('/').any(|c| c.matches('*').count() > 1)
                        })
                    })
                })
            {
                motivos.insert("glob de membros fora do subconjunto suportado".into());
            }
            if !self.membros.contains_key(&raiz) {
                let dirs = crate::enumerar_membros(&raiz)?
                    .into_iter()
                    .map(|m| m.dir)
                    .collect();
                self.membros.insert(raiz.clone(), dirs);
            }
            pendentes.extend(self.membros[&raiz].iter().cloned());
        }
        if raiz_toml.get("package").is_some() {
            pendentes.insert(raiz.clone());
        }
        coletar_paths_especiais(&raiz_toml, &raiz, &mut pendentes, &mut motivos);
        let mut vistos = BTreeSet::new();
        while let Some(p) = pendentes.pop_first() {
            let p = canonico(&p)?;
            if !vistos.insert(p.clone()) {
                continue;
            }
            self.configuracao(&mut h, &p, &mut motivos)?;
            let manifesto = self.manifesto(&p.join("Cargo.toml"))?;
            if !p.starts_with(&raiz) {
                for ancestral in p.ancestors().skip(1) {
                    let cargo = ancestral.join("Cargo.toml");
                    self.arquivo(&mut h, &cargo, false)?;
                    self.arquivo(&mut h, &ancestral.join("Cargo.lock"), false)?;
                    if self.ler(&cargo)?.is_some()
                        && self.manifesto(&cargo)?.get("workspace").is_some()
                    {
                        motivos.insert(format!(
                            "workspace ancestral de dependência externa: {}",
                            cargo.display()
                        ));
                    }
                }
                if manifesto.to_string().contains("workspace") {
                    motivos.insert(format!("herança em dependência externa: {}", p.display()));
                }
            }
            self.arquivo(&mut h, &p.join("Cargo.lock"), false)?;
            let (digest, m) = self.arvore(&p)?;
            campo(&mut h, b"pacote-local", p.as_os_str().as_encoded_bytes());
            campo(&mut h, b"arvore", &digest);
            motivos.extend(m);
            let pacote = manifesto.get("package");
            if pacote.is_none() {
                motivos.insert(format!("manifesto sem pacote: {}", p.display()));
            }
            if pacote
                .and_then(|v| v.get("build"))
                .is_some_and(|v| v.as_bool() != Some(false))
                || self.ler(&p.join("build.rs"))?.is_some()
            {
                motivos.insert(format!("build script: {}", p.display()));
            }
            if manifesto.get("lib").is_some_and(|lib| {
                lib.get("proc-macro").and_then(|v| v.as_bool()) == Some(true)
                    || lib
                        .get("crate-type")
                        .and_then(|v| v.as_array())
                        .is_some_and(|tipos| tipos.iter().any(|v| v.as_str() == Some("proc-macro")))
            }) {
                motivos.insert(format!("proc macro: {}", p.display()));
            }
            if manifesto.get("cargo-features").is_some()
                || pacote.and_then(|v| v.get("links")).is_some()
            {
                motivos.insert(format!("compilação especial/instável: {}", p.display()));
            }
            // Um workspace externo pode fornecer herança/configuração adicional:
            // seguir os paths explícitos, mas não autorizar cache nesse domínio.
            if pacote.and_then(|v| v.get("workspace")).is_some()
                || (p != raiz && manifesto.get("workspace").is_some())
            {
                motivos.insert(format!("workspace de dependência externa: {}", p.display()));
            }
            coletar_dependencias(
                &manifesto,
                &p,
                if p.starts_with(&raiz)
                    && pacote.and_then(|v| v.get("workspace")).is_none()
                    && (p == raiz || manifesto.get("workspace").is_none())
                {
                    Some(&raiz_toml)
                } else {
                    None
                },
                &raiz,
                &mut pendentes,
                &mut motivos,
            );
            coletar_paths_especiais(&manifesto, &p, &mut pendentes, &mut motivos);
            verificar_alvos(&manifesto, &p, &mut motivos)?;
        }
        let (fork, confiavel) = self.identidade_fork()?;
        campo(&mut h, b"fork", &fork);
        if !confiavel {
            motivos.insert("identidade do executável do fork não comprovada".into());
        }
        Ok(ValidadeCache {
            chave: format!("{:x}", h.finalize()),
            motivos_bypass: motivos,
        })
    }
}

fn coletar_dependencias(
    v: &toml::Value,
    base: &Path,
    raiz_toml: Option<&toml::Value>,
    raiz: &Path,
    pendentes: &mut BTreeSet<PathBuf>,
    motivos: &mut BTreeSet<String>,
) {
    for tabela in [
        "dependencies",
        "dev-dependencies",
        "build-dependencies",
        "dev_dependencies",
        "build_dependencies",
    ] {
        if let Some(deps) = v.get(tabela).and_then(|v| v.as_table()) {
            for (nome, dep) in deps {
                let herdada = dep.get("workspace").and_then(|v| v.as_bool()) == Some(true);
                let (dep, base) = if herdada {
                    let Some(raiz_toml) = raiz_toml else {
                        motivos.insert(format!("herança de outro workspace não resolvida: {nome}"));
                        continue;
                    };
                    (
                        raiz_toml
                            .get("workspace")
                            .and_then(|v| v.get("dependencies"))
                            .and_then(|v| v.get(nome))
                            .unwrap_or(dep),
                        raiz,
                    )
                } else {
                    (dep, base)
                };
                if let Some(path) = dep.get("path").and_then(|v| v.as_str()) {
                    pendentes.insert(base.join(path));
                } else {
                    motivos.insert(format!(
                        "dependência não local ou herança não resolvida: {nome}"
                    ));
                }
            }
        }
    }
    if let Some(targets) = v.get("target").and_then(|v| v.as_table()) {
        for t in targets.values() {
            coletar_dependencias(t, base, raiz_toml, raiz, pendentes, motivos);
        }
    }
}

fn coletar_paths_especiais(
    v: &toml::Value,
    base: &Path,
    pendentes: &mut BTreeSet<PathBuf>,
    motivos: &mut BTreeSet<String>,
) {
    for key in ["patch", "replace"] {
        if let Some(t) = v.get(key) {
            motivos.insert(format!("resolução Cargo por {key}: {}", base.display()));
            fn paths(v: &toml::Value, base: &Path, pendentes: &mut BTreeSet<PathBuf>) {
                if let Some(t) = v.as_table() {
                    if let Some(p) = t.get("path").and_then(|p| p.as_str()) {
                        pendentes.insert(base.join(p));
                    }
                    for v in t.values() {
                        paths(v, base, pendentes);
                    }
                }
            }
            paths(t, base, pendentes);
        }
    }
}

fn verificar_alvos(v: &toml::Value, base: &Path, motivos: &mut BTreeSet<String>) -> Resultado<()> {
    for key in ["lib", "bin", "example", "test", "bench"] {
        if let Some(v) = v.get(key) {
            let alvos: Vec<_> = match v.as_array() {
                Some(a) => a.iter().collect(),
                None => vec![v],
            };
            for alvo in alvos {
                if let Some(path) = alvo.get("path").and_then(|v| v.as_str()) {
                    let p = canonico(&base.join(path))?;
                    if p.extension().is_none_or(|e| e != "rs") {
                        motivos
                            .insert(format!("fonte com extensão não rastreada: {}", p.display()));
                    }
                    if !p.starts_with(base)
                        || p.strip_prefix(base)
                            .unwrap()
                            .components()
                            .any(|c| c.as_os_str() == "target" || c.as_os_str() == ".git")
                    {
                        motivos.insert(format!("alvo fora da árvore rastreada: {}", p.display()));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        base: PathBuf,
        raiz: PathBuf,
        membro: MembroWorkspace,
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.base);
        }
    }
    fn escrever(p: &Path, b: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b).unwrap();
    }
    fn pacote(p: &Path, nome: &str, extra: &str) {
        escrever(
            &p.join("Cargo.toml"),
            &format!("[package]\nname={nome:?}\nversion=\"0.0.0\"\n{extra}"),
        );
        escrever(&p.join("src/lib.rs"), "pub fn x() {}\n");
    }
    fn fixture() -> Fixture {
        let base = std::env::temp_dir().join(format!(
            "lente-validade-{}-{}",
            std::process::id(),
            SEQ.fetch_add(1, Ordering::Relaxed)
        ));
        let raiz = base.join("ws");
        escrever(&raiz.join("Cargo.toml"), "[workspace]\nmembers=[\"a\"]\n");
        let membro = MembroWorkspace {
            nome: "a".into(),
            dir: raiz.join("a"),
        };
        pacote(&membro.dir, "a", "");
        Fixture { base, raiz, membro }
    }
    fn rodada() -> CacheExtracao {
        CacheExtracao {
            ambiente: BTreeMap::from([(
                OsString::from("CARGO_HOME"),
                std::env::temp_dir()
                    .join(format!("lente-cargo-home-ausente-{}", std::process::id()))
                    .into_os_string(),
            )]),
            fork: Some((b"fork-A".to_vec(), true)),
            ..CacheExtracao::default()
        }
    }
    fn avaliar(f: &Fixture) -> ValidadeCache {
        rodada().avaliar(&f.membro, &f.raiz, "rustc teste").unwrap()
    }

    #[test]
    fn estabilidade_raiz_outro_membro_e_ausencia_diferente_de_vazio() {
        let f = fixture();
        let a = avaliar(&f);
        assert!(a.motivos_bypass.is_empty(), "{:?}", a.motivos_bypass);
        assert_eq!(a.chave, avaliar(&f).chave);
        escrever(
            &f.raiz.join("Cargo.toml"),
            "[workspace]\nmembers=[\"a\"]\nresolver=\"2\"\n",
        );
        let b = avaliar(&f).chave;
        assert_ne!(a.chave, b);
        pacote(&f.raiz.join("outro"), "outro", "");
        let c = avaliar(&f).chave;
        assert_ne!(b, c);
        escrever(&f.raiz.join("Cargo.lock"), "");
        assert_ne!(c, avaliar(&f).chave);
    }

    #[test]
    fn dependencia_transitiva_externa_e_herdada_invalida_consumidor() {
        let f = fixture();
        escrever(
            &f.raiz.join("Cargo.toml"),
            "[workspace]\nmembers=[\"a\",\"b\"]\n[workspace.dependencies]\nb={path=\"b\"}\n",
        );
        pacote(&f.membro.dir, "a", "[dependencies]\nb={workspace=true}\n");
        pacote(
            &f.raiz.join("b"),
            "b",
            "[target.'cfg(unix)'.dependencies]\nc={path=\"../../externa\"}\n",
        );
        pacote(&f.base.join("externa"), "c", "");
        let antes = avaliar(&f);
        assert!(
            antes.motivos_bypass.is_empty(),
            "{:?}",
            antes.motivos_bypass
        );
        escrever(&f.base.join("externa/src/lib.rs"), "pub struct Mudou;\n");
        assert_ne!(antes.chave, avaliar(&f).chave);
        let antes = avaliar(&f).chave;
        pacote(&f.base.join("externa"), "c", "[features]\nnova=[]\n");
        assert_ne!(antes, avaliar(&f).chave);
    }

    #[test]
    fn dependencias_de_outro_membro_tambem_fecham_resolucao() {
        let f = fixture();
        escrever(
            &f.raiz.join("Cargo.toml"),
            "[workspace]\nmembers=[\"a\",\"b\"]\n",
        );
        pacote(
            &f.raiz.join("b"),
            "b",
            "[dependencies]\nc={path=\"../../externa\"}\n",
        );
        pacote(&f.base.join("externa"), "c", "");
        let a = avaliar(&f);
        escrever(&f.base.join("externa/src/lib.rs"), "pub struct Mudou;\n");
        assert_ne!(a.chave, avaliar(&f).chave);
        pacote(
            &f.raiz.join("b"),
            "b",
            "[dependencies]\nregistry_dep=\"1\"\n",
        );
        assert!(!avaliar(&f).motivos_bypass.is_empty());
    }

    #[test]
    fn heranca_externa_nao_usa_tabela_do_workspace_errado() {
        let f = fixture();
        escrever(
            &f.raiz.join("Cargo.toml"),
            "[workspace]\nmembers=[\"a\"]\n[workspace.dependencies]\nshared={path=\"nao-existe\"}\n",
        );
        pacote(
            &f.membro.dir,
            "a",
            "[dependencies]\nb={path=\"../../externa\"}\n",
        );
        pacote(
            &f.base.join("externa"),
            "b",
            "[dependencies]\nshared={workspace=true}\n",
        );
        let validade = avaliar(&f);
        assert!(
            validade
                .motivos_bypass
                .iter()
                .any(|m| m.contains("herança de outro workspace"))
        );
    }

    #[test]
    fn checkout_copiado_nao_reutiliza_posicoes() {
        let a = fixture();
        let b = fixture();
        assert_ne!(avaliar(&a).chave, avaliar(&b).chave);
        // A mesma identidade canônica continua estável com componentes redundantes.
        let m = MembroWorkspace {
            nome: "a".into(),
            dir: a.membro.dir.join("../a"),
        };
        assert_eq!(
            avaliar(&a).chave,
            rodada().avaliar(&m, &a.raiz, "rustc teste").unwrap().chave
        );
    }

    #[test]
    fn fonte_fora_de_src_dentro_da_arvore_invalida_e_externa_bypassa() {
        let f = fixture();
        escrever(
            &f.membro.dir.join("codigo/custom.rs"),
            "pub struct Inicial;\n",
        );
        pacote(&f.membro.dir, "a", "[lib]\npath=\"codigo/custom.rs\"\n");
        let a = avaliar(&f);
        assert!(a.motivos_bypass.is_empty());
        escrever(
            &f.membro.dir.join("codigo/custom.rs"),
            "pub struct Mudou;\n",
        );
        assert_ne!(a.chave, avaliar(&f).chave);
        escrever(&f.base.join("externa.rs"), "pub struct Fora;\n");
        pacote(&f.membro.dir, "a", "[lib]\npath=\"../../externa.rs\"\n");
        assert!(
            avaliar(&f)
                .motivos_bypass
                .iter()
                .any(|m| m.contains("alvo fora"))
        );
    }

    #[test]
    fn toolchain_fork_e_ambiente_invalida() {
        let f = fixture();
        let a = avaliar(&f).chave;
        assert_ne!(
            a,
            rodada()
                .avaliar(&f.membro, &f.raiz, "rustc outro")
                .unwrap()
                .chave
        );
        let mut r = rodada();
        r.fork = Some((b"fork-B".to_vec(), true));
        assert_ne!(
            a,
            r.avaliar(&f.membro, &f.raiz, "rustc teste").unwrap().chave
        );
        let mut r = rodada();
        r.ambiente.insert("RUSTUP_TOOLCHAIN".into(), "outro".into());
        assert_ne!(
            a,
            r.avaliar(&f.membro, &f.raiz, "rustc teste").unwrap().chave
        );
        escrever(
            &f.membro.dir.join("rust-toolchain.toml"),
            "[toolchain]\nchannel=\"nightly\"\n",
        );
        assert_ne!(a, avaliar(&f).chave);
    }

    #[test]
    fn executavel_fork_identificado_por_conteudo_nao_so_versao() {
        let f = fixture();
        let bin = f.base.join("bin");
        let exe = bin.join(format!("cargo-modules{}", std::env::consts::EXE_SUFFIX));
        escrever(&exe, "fork revision A");
        let novo = || {
            let mut r = rodada();
            r.fork = None;
            r.ambiente.insert("PATH".into(), bin.as_os_str().into());
            r
        };
        let a = novo().avaliar(&f.membro, &f.raiz, "tc").unwrap();
        assert!(a.motivos_bypass.is_empty());
        escrever(&exe, "fork revision B");
        let b = novo().avaliar(&f.membro, &f.raiz, "tc").unwrap();
        assert_ne!(a.chave, b.chave);
    }

    #[test]
    fn entradas_dinamicas_tem_bypass_explicito() {
        for extra in ["[dependencies]\nserde=\"1\"\n", "[lib]\nproc-macro=true\n"] {
            let f = fixture();
            pacote(&f.membro.dir, "a", extra);
            assert!(!avaliar(&f).motivos_bypass.is_empty());
        }
        for (nome, conteudo) in [
            ("build.rs", "fn main() {}"),
            ("src/lib.rs", "include!(\"../../fora.rs\");"),
            ("src/lib.rs", "const X: &str = env!(\"X\");"),
            ("src/lib.rs", "#[path=\"../../fora.rs\"] mod fora;"),
            (".cargo/config.toml", "[build]\nrustflags=[\"--cfg=foo\"]"),
        ] {
            let f = fixture();
            escrever(&f.membro.dir.join(nome), conteudo);
            assert!(!avaliar(&f).motivos_bypass.is_empty(), "{nome}");
        }
        let f = fixture();
        let mut r = rodada();
        r.ambiente.insert("RUSTC_WRAPPER".into(), "/wrapper".into());
        assert!(
            !r.avaliar(&f.membro, &f.raiz, "tc")
                .unwrap()
                .motivos_bypass
                .is_empty()
        );
    }

    #[test]
    fn configuracao_ancestral_e_cargo_home_bypass_e_invalida() {
        let f = fixture();
        let a = avaliar(&f).chave;
        escrever(&f.base.join(".cargo/config"), "[net]\noffline=true\n");
        let b = avaliar(&f);
        assert_ne!(a, b.chave);
        assert!(!b.motivos_bypass.is_empty());
        let mut r = rodada();
        r.ambiente
            .insert("CARGO_HOME".into(), f.base.join("home").into_os_string());
        escrever(&f.base.join("home/config.toml"), "[build]\njobs=1\n");
        assert!(
            !r.avaliar(&f.membro, &f.raiz, "tc")
                .unwrap()
                .motivos_bypass
                .is_empty()
        );
    }

    #[test]
    fn leituras_e_digests_compartilhados_entre_membros() {
        let f = fixture();
        pacote(
            &f.raiz.join("b"),
            "b",
            "[dependencies]\na={path=\"../a\"}\n",
        );
        let b = MembroWorkspace {
            nome: "b".into(),
            dir: f.raiz.join("b"),
        };
        let mut r = rodada();
        r.avaliar(&f.membro, &f.raiz, "tc").unwrap();
        let fonte = f.membro.dir.join("src/lib.rs");
        let bytes = r.arquivos[&fonte].clone().unwrap();
        let n = r.digests.len();
        r.avaliar(&b, &f.raiz, "tc").unwrap();
        assert!(Arc::ptr_eq(&bytes, r.arquivos[&fonte].as_ref().unwrap()));
        assert_eq!(n, r.digests.len());
    }

    #[test]
    fn campos_nao_ambiguos_com_nul_e_fronteiras() {
        let mut a = Sha256::new();
        let mut b = Sha256::new();
        campo(&mut a, b"x", b"a\0b");
        campo(&mut a, b"y", b"c");
        campo(&mut b, b"x", b"a");
        campo(&mut b, b"y", b"b\0c");
        assert_ne!(a.finalize(), b.finalize());
    }

    #[test]
    fn falhas_relevantes_nao_viram_vazio() {
        let f = fixture();
        std::fs::create_dir_all(f.raiz.join("Cargo.lock")).unwrap();
        let erro = rodada().avaliar(&f.membro, &f.raiz, "tc").unwrap_err();
        assert!(erro.to_string().contains("Cargo.lock"));
        std::fs::remove_dir(f.raiz.join("Cargo.lock")).unwrap();
        pacote(
            &f.membro.dir,
            "a",
            "[dependencies]\nb={path=\"../ausente\"}\n",
        );
        assert!(rodada().avaliar(&f.membro, &f.raiz, "tc").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_circular_bypass_sem_recursao() {
        let f = fixture();
        std::os::unix::fs::symlink(&f.membro.dir, f.membro.dir.join("volta")).unwrap();
        assert!(
            avaliar(&f)
                .motivos_bypass
                .iter()
                .any(|m| m.contains("link simbólico"))
        );
    }
}
