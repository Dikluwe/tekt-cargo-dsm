//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-diff.md
//! @prompt-hash 509a4c17
//! @layer L3
//! @updated 2026-06-07
//! Camada:  L3 — Infraestrutura. I/O (subprocesso `git`) permitido.
//!
//! A **entrada** do modo `--diff`: lê um diff e o estrutura na forma de dados
//! L1 ([`DiffEstruturado`], de `lente_core`). Cobre os dois lados do quadro de
//! trabalho (laudo 0043):
//!
//! - **Rastreados**: `git diff HEAD --unified=0` (staged + unstaged vs o último
//!   commit), parseado em faixas do **lado novo** por [`parse_diff`] (pura).
//! - **Untracked**: `git ls-files --others --exclude-standard` (respeita
//!   `.gitignore`); para cada arquivo, um hunk sintético "tudo adicionado"
//!   (`{ inicio: 1, fim: n_linhas }`).
//!
//! Os caminhos são normalizados para **absolutos** (`raiz.join(relativo)`), para
//! casar com `No.position.file` (laudo 0037/0038) no mapeamento (L1).
//!
//! ## `git` ≠ `cargo`
//!
//! O invariante "dois subprocessos do **cargo**" (laudo 0018/0023) é sobre o
//! cargo; o `git` é outra ferramenta. Por higiene, **uma primitiva única de
//! git** — [`invocar_git`] — como o fork tem uma única para o cargo.
//!
//! ## Limitação: deleção não mapeia
//!
//! Um diff só de `-linhas` (deleção) não tem linha no lado novo, e o nó deletado
//! já não está no grafo pós-mudança. [`parse_diff`] **não** gera faixa para um
//! hunk de deleção pura (`+c,0`); não tentamos mapear deleções (o grafo atual
//! reflete adições/modificações, não remoções).

use core::error::Error;
use core::fmt;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use lente_core::domain::mapeamento::{
    ArquivoDiff, DiffEstruturado, FaixaLinhas, OrigemArquivo,
};

/// Modos de falha da leitura do diff.
#[derive(Debug)]
pub enum ErroDiff {
    /// O subprocesso `git` rodou mas terminou com status de erro.
    Git {
        codigo: Option<i32>,
        stderr: String,
    },
    /// Diff malformado (não usado hoje — `parse_diff` é tolerante; reservado
    /// para validações futuras mais estritas).
    Parse(String),
    /// Falha de I/O: iniciar o `git`, ou ler um arquivo untracked do disco.
    Io(std::io::Error),
}

impl fmt::Display for ErroDiff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErroDiff::Git { codigo, stderr } => {
                write!(f, "`git` falhou (exit {:?}): {}", codigo, stderr.trim())
            }
            ErroDiff::Parse(m) => write!(f, "diff malformado: {}", m),
            ErroDiff::Io(e) => write!(f, "falha de I/O ao ler o diff: {}", e),
        }
    }
}

impl Error for ErroDiff {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ErroDiff::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Lê o diff do repositório em `raiz`: rastreados (`git diff HEAD`) + untracked
/// (`git ls-files --others`), com caminhos normalizados para absolutos.
pub fn ler_diff(raiz: &Path) -> Result<DiffEstruturado, ErroDiff> {
    let mut arquivos: Vec<ArquivoDiff> = Vec::new();

    // 1. Rastreados: `git diff HEAD --unified=0`. `parse_diff` dá caminhos
    //    relativos; normalizamos para absolutos juntando a raiz.
    let saida = invocar_git(&["diff", "HEAD", "--unified=0", "--no-color"], raiz)?;
    for mut a in parse_diff(&saida) {
        a.caminho = raiz.join(&a.caminho);
        arquivos.push(a);
    }

    // 2. Untracked: hunk sintético "tudo adicionado", lendo o arquivo do disco.
    let lista = invocar_git(&["ls-files", "--others", "--exclude-standard"], raiz)?;
    for rel in lista.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let caminho = raiz.join(rel);
        let n = contar_linhas(&caminho)?;
        let linhas_alteradas = if n == 0 {
            Vec::new()
        } else {
            vec![FaixaLinhas { inicio: 1, fim: n }]
        };
        arquivos.push(ArquivoDiff {
            caminho,
            origem: OrigemArquivo::NaoRastreado,
            linhas_alteradas,
        });
    }

    Ok(DiffEstruturado { arquivos })
}

/// **Primitiva única de git** do crate: roda `git <args>` em `dir` e devolve o
/// stdout. (Como `fork::invocar_em` é a única do cargo, laudo 0018.)
fn invocar_git(args: &[&str], dir: &Path) -> Result<String, ErroDiff> {
    let saida = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(ErroDiff::Io)?;
    if !saida.status.success() {
        return Err(ErroDiff::Git {
            codigo: saida.status.code(),
            stderr: String::from_utf8_lossy(&saida.stderr).into_owned(),
        });
    }
    Ok(String::from_utf8_lossy(&saida.stdout).into_owned())
}

/// Conta as linhas de um arquivo por bytes (`\n`), tolerante a não-UTF-8 — um
/// untracked pode ser binário (cai em `nao_fonte` no mapeamento; a contagem é
/// inócua ali). Última linha sem `\n` final conta como uma linha.
fn contar_linhas(arquivo: &Path) -> Result<u32, ErroDiff> {
    let bytes = std::fs::read(arquivo).map_err(ErroDiff::Io)?;
    if bytes.is_empty() {
        return Ok(0);
    }
    let mut n = bytes.iter().filter(|&&b| b == b'\n').count() as u32;
    if *bytes.last().unwrap() != b'\n' {
        n += 1;
    }
    Ok(n)
}

/// Parseia um diff unificado nas faixas do **lado novo** de cada arquivo.
/// Função **pura** (sem git, sem I/O) — testável diretamente.
///
/// Os caminhos saem **relativos** (como o diff os traz, sem o prefixo `b/`);
/// `origem = Rastreado` (o diff só contém rastreados). `ler_diff` os normaliza
/// para absolutos. Um arquivo cujo único hunk é deleção pura (`+c,0`) **não**
/// aparece (sem faixa nova — limitação documentada).
fn parse_diff(texto: &str) -> Vec<ArquivoDiff> {
    let mut por_arquivo: BTreeMap<String, Vec<FaixaLinhas>> = BTreeMap::new();
    let mut corrente: Option<String> = None;
    for linha in texto.lines() {
        if let Some(resto) = linha.strip_prefix("+++ ") {
            let resto = resto.trim();
            if resto == "/dev/null" {
                corrente = None;
                continue;
            }
            corrente = Some(resto.strip_prefix("b/").unwrap_or(resto).to_string());
        } else if linha.starts_with("@@") {
            let Some(arquivo) = corrente.as_ref() else {
                continue;
            };
            // O `@@ -a,b +c,d @@`: a faixa nova começa após o primeiro `+`.
            let Some(parte_mais) = linha.split('+').nth(1) else {
                continue;
            };
            let parte_mais = parte_mais.split(' ').next().unwrap_or("");
            let (c, d) = match parte_mais.split_once(',') {
                Some((c, d)) => (c.parse::<u32>().unwrap_or(0), d.parse::<u32>().unwrap_or(1)),
                None => (parte_mais.parse::<u32>().unwrap_or(0), 1),
            };
            // c == 0: arquivo novo num diff vazio; d == 0: deleção pura. Nenhum
            // gera faixa no lado novo.
            if c == 0 || d == 0 {
                continue;
            }
            por_arquivo
                .entry(arquivo.clone())
                .or_default()
                .push(FaixaLinhas {
                    inicio: c,
                    fim: c + d - 1,
                });
        }
    }
    por_arquivo
        .into_iter()
        .map(|(caminho, linhas_alteradas)| ArquivoDiff {
            caminho: PathBuf::from(caminho),
            origem: OrigemArquivo::Rastreado,
            linhas_alteradas,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_diff_extrai_faixas_do_lado_novo() {
        let d = "diff --git a/x.rs b/x.rs\n\
                 --- a/x.rs\n\
                 +++ b/x.rs\n\
                 @@ -10,2 +10,3 @@\n\
                 +nova\n";
        let arqs = parse_diff(d);
        assert_eq!(arqs.len(), 1);
        assert_eq!(arqs[0].caminho, PathBuf::from("x.rs"));
        assert_eq!(arqs[0].origem, OrigemArquivo::Rastreado);
        assert_eq!(
            arqs[0].linhas_alteradas,
            vec![FaixaLinhas { inicio: 10, fim: 12 }]
        );
    }

    #[test]
    fn parse_diff_hunk_sem_virgula_e_uma_linha() {
        // `@@ -2 +2 @@` (uma linha de cada lado): faixa nova 2..2.
        let d = "--- a/x.rs\n+++ b/x.rs\n@@ -2 +2 @@\n+x\n";
        let arqs = parse_diff(d);
        assert_eq!(
            arqs[0].linhas_alteradas,
            vec![FaixaLinhas { inicio: 2, fim: 2 }]
        );
    }

    #[test]
    fn parse_diff_delecao_pura_nao_gera_faixa() {
        // `@@ -10,3 +9,0 @@`: d == 0 (nada no lado novo). Arquivo sem faixa →
        // não aparece (limitação documentada).
        let d = "--- a/x.rs\n+++ b/x.rs\n@@ -10,3 +9,0 @@\n-foi\n-embora\n-mesmo\n";
        let arqs = parse_diff(d);
        assert!(arqs.is_empty(), "deleção pura não gera ArquivoDiff");
    }

    #[test]
    fn parse_diff_arquivo_removido_dev_null_e_ignorado() {
        // `+++ /dev/null` (arquivo todo removido): sem lado novo.
        let d = "--- a/x.rs\n+++ /dev/null\n@@ -1,3 +0,0 @@\n-a\n-b\n-c\n";
        let arqs = parse_diff(d);
        assert!(arqs.is_empty());
    }

    #[test]
    fn parse_diff_multiplos_hunks_e_arquivos() {
        let d = "--- a/a.rs\n+++ b/a.rs\n@@ -1 +1,2 @@\n+x\n@@ -10 +11,3 @@\n+y\n\
                 --- a/b.rs\n+++ b/b.rs\n@@ -5,0 +6,1 @@\n+z\n";
        let arqs = parse_diff(d);
        assert_eq!(arqs.len(), 2);
        let a = arqs.iter().find(|x| x.caminho == PathBuf::from("a.rs")).unwrap();
        assert_eq!(
            a.linhas_alteradas,
            vec![
                FaixaLinhas { inicio: 1, fim: 2 },
                FaixaLinhas { inicio: 11, fim: 13 },
            ]
        );
        let b = arqs.iter().find(|x| x.caminho == PathBuf::from("b.rs")).unwrap();
        assert_eq!(b.linhas_alteradas, vec![FaixaLinhas { inicio: 6, fim: 6 }]);
    }

    #[test]
    fn erro_diff_implementa_display() {
        let v = [
            ErroDiff::Git {
                codigo: Some(128),
                stderr: "not a git repository".to_string(),
            },
            ErroDiff::Parse("eof".to_string()),
            ErroDiff::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "x")),
        ];
        for e in &v {
            assert!(!format!("{}", e).is_empty());
        }
    }

    /// E2E (requer `git`): repo temporário com um arquivo rastreado modificado
    /// e um arquivo novo não rastreado. Confirma a faixa do rastreado, o hunk
    /// "tudo adicionado" do untracked, e a normalização para caminho absoluto.
    #[test]
    #[ignore]
    fn e2e_ler_diff_rastreado_e_untracked() {
        let dir = std::env::temp_dir().join(format!("lente_diff_e2e_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .args(args)
                .current_dir(&dir)
                .output()
                .expect("git");
        };
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t.test"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("a.txt"), "l1\nl2\nl3\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "base"]);

        // Modifica o rastreado (linha 2) e cria um untracked com 2 linhas.
        std::fs::write(dir.join("a.txt"), "l1\nMODIFICADA\nl3\n").unwrap();
        std::fs::write(dir.join("novo.rs"), "fn x() {}\nfn y() {}\n").unwrap();

        let diff = ler_diff(&dir).expect("ler_diff deve funcionar");

        let a = diff
            .arquivos
            .iter()
            .find(|f| f.caminho.ends_with("a.txt"))
            .expect("a.txt no diff");
        assert_eq!(a.origem, OrigemArquivo::Rastreado);
        assert_eq!(a.linhas_alteradas, vec![FaixaLinhas { inicio: 2, fim: 2 }]);
        assert!(a.caminho.is_absolute(), "caminho normalizado p/ absoluto");

        let n = diff
            .arquivos
            .iter()
            .find(|f| f.caminho.ends_with("novo.rs"))
            .expect("novo.rs no diff");
        assert_eq!(n.origem, OrigemArquivo::NaoRastreado);
        assert_eq!(n.linhas_alteradas, vec![FaixaLinhas { inicio: 1, fim: 2 }]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
