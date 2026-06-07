//! Lineage: prompt 00_nucleo/prompt/0046-ler_diff_e_mapear.md
//! Spec:    00_nucleo/specs/forma-organizada.md
//! ADRs:    00_nucleo/adr/0002-modelagem-do-grafo.md
//! Camada:  L1 — Núcleo. Apenas stdlib. Sem I/O. Sem deps externas.
//!
//! O núcleo do modo `--diff`: mapeia um diff estruturado aos nós do grafo que
//! ele toca, e faz o **censo do untracked** (ligado / solto / não-fonte).
//!
//! ## Por que a forma do diff vive aqui (L1) e não no L3
//!
//! `mapear_diff` é **puro** (restrição dura do prompt: só stdlib, sem dep). Ele
//! consome o diff estruturado; se a forma do diff morasse no `lente_infra` (L3),
//! `mapear_diff` importaria L3 e inverteria a dependência de camada. Então os
//! **tipos de dado** do diff (`DiffEstruturado`/`ArquivoDiff`/`OrigemArquivo`/
//! `FaixaLinhas`) são L1 — dados puros, como o [`Grafo`] — e o **leitor** que os
//! materializa do `git` é L3 (`lente_infra::ler_diff`). Mesmo padrão do `Grafo`:
//! a forma é L1, a extração é L3.
//!
//! ## Reconciliação de caminho (o pulo do gato, laudo 0038)
//!
//! Os caminhos do diff são relativos ao repo; as `position.file` dos nós são
//! **absolutas** (laudo 0037). `arquivo_casa` reconcilia: bate por igualdade ou
//! por sufixo em **fronteira de segmento** (`/abs/a/src/x.rs` casa o relativo
//! `a/src/x.rs`). É a mesma abordagem que o laudo 0038 confirmou para rastreados.

use std::collections::BTreeSet;
use std::path::{Path as FsPath, PathBuf};

use crate::entities::grafo::{Grafo, Path};

/// Origem de um arquivo no diff: rastreado pelo git (`git diff HEAD`) ou novo,
/// ainda **não rastreado** (`git ls-files --others`, com hunk sintético).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrigemArquivo {
    Rastreado,
    NaoRastreado,
}

/// Faixa de linhas do **lado novo** do diff (adições/modificações). 1-based,
/// inclusiva. Para untracked, o leitor sintetiza `{ inicio: 1, fim: n_linhas }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FaixaLinhas {
    pub inicio: u32,
    pub fim: u32,
}

/// Um arquivo alterado no diff.
///
/// `caminho` é normalizado para casar com `No.position.file` — **absoluto**
/// quando vindo de `lente_infra::ler_diff` (que junta a raiz do repo ao caminho
/// relativo do diff). A reconciliação em [`mapear_diff`] também aceita caminho
/// relativo (sufixo da `position` absoluta), para testes e robustez.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArquivoDiff {
    pub caminho: PathBuf,
    pub origem: OrigemArquivo,
    /// Faixas do lado novo. Vazio para um arquivo cujo diff é só deleção
    /// (limitação documentada: deleção não mapeia — o nó já não está no grafo).
    pub linhas_alteradas: Vec<FaixaLinhas>,
}

/// O diff inteiro, estruturado (rastreados + untracked).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEstruturado {
    pub arquivos: Vec<ArquivoDiff>,
}

/// Um nó cuja `position` cruza uma faixa alterada do diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoTocado {
    pub id: usize,
    pub path: Path,
}

/// Resultado do mapeamento: os nós tocados e o censo do untracked nos 3 baldes
/// do laudo 0043.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapeamentoDiff {
    /// Nós cuja `position` cruza uma faixa alterada (rastreado + untracked
    /// ligado, via o hunk sintético). Ordenado por path, depois id.
    pub tocados: Vec<NoTocado>,
    /// Untracked que **está** no grafo (o cargo o compilou) — seus nós aparecem
    /// em `tocados`.
    pub ligados: Vec<PathBuf>,
    /// Untracked `.rs` dentro de um dir de membro mas **fora** do grafo
    /// (presente, não compilado — falta um `mod`). Sinal acionável, não erro.
    pub soltos: Vec<PathBuf>,
    /// Untracked fora de qualquer dir de membro (ou não-`.rs`): docs, lab, etc.
    pub nao_fonte: Vec<PathBuf>,
}

/// Mapeia um diff estruturado ao grafo: nós tocados por cruzamento de posição +
/// o censo do untracked. Puro e determinístico (saídas ordenadas).
///
/// `membros_dirs` (os diretórios dos crates-membro, vindos do
/// `enumerar_membros` do 0044, fornecidos pela orquestração do 0047) é o que
/// separa `soltos` (`.rs` em membro, fora do grafo) de `nao_fonte` (fora de
/// membro).
pub fn mapear_diff(
    diff: &DiffEstruturado,
    grafo: &Grafo,
    membros_dirs: &[PathBuf],
) -> MapeamentoDiff {
    // Tocados: para cada arquivo, os nós cujo `position.file` casa o caminho e
    // cuja faixa de linhas cruza uma faixa alterada. Dedup por id (laudo 0038).
    let mut tocados: Vec<NoTocado> = Vec::new();
    let mut vistos: BTreeSet<usize> = BTreeSet::new();
    for arq in &diff.arquivos {
        for n in &grafo.nodes {
            let Some(pos) = n.position.as_ref() else {
                continue;
            };
            if !arquivo_casa(&pos.file, &arq.caminho) {
                continue;
            }
            let cruza = arq
                .linhas_alteradas
                .iter()
                .any(|f| intersecta(pos.start_line, pos.end_line, f.inicio, f.fim));
            if cruza && vistos.insert(n.id) {
                tocados.push(NoTocado {
                    id: n.id,
                    path: n.path.clone(),
                });
            }
        }
    }
    tocados.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()).then(a.id.cmp(&b.id)));

    // Censo do untracked (os 3 baldes do laudo 0043).
    let mut ligados: Vec<PathBuf> = Vec::new();
    let mut soltos: Vec<PathBuf> = Vec::new();
    let mut nao_fonte: Vec<PathBuf> = Vec::new();
    for arq in &diff.arquivos {
        if arq.origem != OrigemArquivo::NaoRastreado {
            continue;
        }
        if esta_no_grafo(&arq.caminho, grafo) {
            ligados.push(arq.caminho.clone());
        } else if eh_rust(&arq.caminho) && sob_membro(&arq.caminho, membros_dirs) {
            soltos.push(arq.caminho.clone());
        } else {
            nao_fonte.push(arq.caminho.clone());
        }
    }
    ligados.sort();
    soltos.sort();
    nao_fonte.sort();

    MapeamentoDiff {
        tocados,
        ligados,
        soltos,
        nao_fonte,
    }
}

/// Os intervalos `[a, b]` e `[c, d]` (inclusivos) se cruzam?
fn intersecta(a: u32, b: u32, c: u32, d: u32) -> bool {
    a <= d && c <= b
}

/// O `position.file` (absoluto) corresponde ao `caminho` do diff? Casa por
/// igualdade ou por sufixo em **fronteira de segmento** — reconcilia a
/// `position` absoluta (0037) com um caminho relativo do diff (0038).
fn arquivo_casa(pos_file: &str, caminho: &FsPath) -> bool {
    let cam = caminho.to_string_lossy();
    if pos_file == cam {
        return true;
    }
    pos_file
        .strip_suffix(cam.as_ref())
        .map(|prefixo| prefixo.is_empty() || prefixo.ends_with('/'))
        .unwrap_or(false)
}

/// Algum nó do grafo tem `position.file` casando o caminho? (= o cargo
/// compilou esse arquivo — a verdade-de-campo de "ligado", laudo 0043).
fn esta_no_grafo(caminho: &FsPath, grafo: &Grafo) -> bool {
    grafo
        .nodes
        .iter()
        .filter_map(|n| n.position.as_ref())
        .any(|p| arquivo_casa(&p.file, caminho))
}

fn eh_rust(caminho: &FsPath) -> bool {
    caminho.extension().map(|e| e == "rs").unwrap_or(false)
}

fn sob_membro(caminho: &FsPath, membros_dirs: &[PathBuf]) -> bool {
    membros_dirs.iter().any(|d| caminho.starts_with(d))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::grafo::{Kind, Modificadores, Posicao, Visibility};

    /// Nó com `position` num arquivo e faixa de linhas dadas.
    fn no_pos(id: usize, path: &str, file: &str, ini: u32, fim: u32) -> crate::entities::grafo::No {
        crate::entities::grafo::No {
            id,
            path: Path::from(path),
            name: path.rsplit("::").next().unwrap_or(path).to_string(),
            kind: Kind::Fn,
            modificadores: Modificadores::default(),
            visibility: Visibility::Pub,
            crate_name: path.split("::").next().unwrap_or("").to_string(),
            trait_: None,
            trait_ref: None,
            cfg: None,
            macro_kind: None,
            is_non_exhaustive: false,
            position: Some(Posicao {
                file: file.to_string(),
                start_line: ini,
                end_line: fim,
            }),
        }
    }

    fn grafo_de(nodes: Vec<crate::entities::grafo::No>) -> Grafo {
        Grafo {
            crate_name: "ws".to_string(),
            nodes,
            edges: Vec::new(),
        }
    }

    fn rastreado(caminho: &str, faixas: &[(u32, u32)]) -> ArquivoDiff {
        ArquivoDiff {
            caminho: PathBuf::from(caminho),
            origem: OrigemArquivo::Rastreado,
            linhas_alteradas: faixas
                .iter()
                .map(|(i, f)| FaixaLinhas { inicio: *i, fim: *f })
                .collect(),
        }
    }

    fn untracked(caminho: &str, n: u32) -> ArquivoDiff {
        ArquivoDiff {
            caminho: PathBuf::from(caminho),
            origem: OrigemArquivo::NaoRastreado,
            linhas_alteradas: vec![FaixaLinhas { inicio: 1, fim: n }],
        }
    }

    #[test]
    fn no_e_tocado_quando_a_faixa_cruza_a_position() {
        // A::foo na 10..20; diff altera 12..14 → cruza.
        let grafo = grafo_de(vec![no_pos(1, "a::foo", "/abs/a/src/lib.rs", 10, 20)]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("/abs/a/src/lib.rs", &[(12, 14)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        assert_eq!(m.tocados.len(), 1);
        assert_eq!(m.tocados[0].path.as_str(), "a::foo");
    }

    #[test]
    fn modulo_arquivo_que_abrange_a_faixa_tambem_e_tocado() {
        // O módulo-arquivo (1..40) e a fn (10..20) ambos cruzam 12..14.
        let grafo = grafo_de(vec![
            no_pos(1, "a", "/abs/a/src/lib.rs", 1, 40),
            no_pos(2, "a::foo", "/abs/a/src/lib.rs", 10, 20),
        ]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("/abs/a/src/lib.rs", &[(12, 14)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        let paths: Vec<&str> = m.tocados.iter().map(|t| t.path.as_str()).collect();
        assert_eq!(paths, vec!["a", "a::foo"]);
    }

    #[test]
    fn fora_da_faixa_nao_e_tocado() {
        let grafo = grafo_de(vec![no_pos(1, "a::foo", "/abs/a/src/lib.rs", 10, 20)]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("/abs/a/src/lib.rs", &[(30, 31)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        assert!(m.tocados.is_empty());
    }

    #[test]
    fn reconciliacao_relativo_do_diff_casa_position_absoluta() {
        // Caminho do diff é RELATIVO; a position do grafo é ABSOLUTA. Casam.
        let grafo = grafo_de(vec![no_pos(1, "a::foo", "/repo/a/src/lib.rs", 10, 20)]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("a/src/lib.rs", &[(11, 11)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        assert_eq!(m.tocados.len(), 1, "relativo↔absoluto deve reconciliar");
    }

    #[test]
    fn sufixo_nao_casa_em_fronteira_parcial() {
        // "/repo/outro_lib.rs" NÃO deve casar "lib.rs" (fronteira de segmento).
        let grafo = grafo_de(vec![no_pos(1, "a::foo", "/repo/a/outro_lib.rs", 1, 9)]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("lib.rs", &[(1, 9)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        assert!(m.tocados.is_empty(), "sufixo parcial não pode casar");
    }

    #[test]
    fn untracked_no_grafo_vai_para_ligados_e_seus_nos_para_tocados() {
        // novo.rs está no grafo (tem nó com position naquele arquivo).
        let grafo = grafo_de(vec![no_pos(1, "a::novo", "/repo/a/src/novo.rs", 1, 5)]);
        let diff = DiffEstruturado {
            arquivos: vec![untracked("/repo/a/src/novo.rs", 5)],
        };
        let m = mapear_diff(&diff, &grafo, &[PathBuf::from("/repo/a")]);
        assert_eq!(m.ligados, vec![PathBuf::from("/repo/a/src/novo.rs")]);
        assert!(m.soltos.is_empty() && m.nao_fonte.is_empty());
        // O nó dele entra em tocados via o hunk sintético (1..5).
        assert_eq!(m.tocados.len(), 1);
        assert_eq!(m.tocados[0].path.as_str(), "a::novo");
    }

    #[test]
    fn untracked_rs_em_membro_fora_do_grafo_vai_para_soltos() {
        // solto.rs é .rs dentro de /repo/a mas NÃO está no grafo (sem `mod`).
        let grafo = grafo_de(vec![no_pos(1, "a::existe", "/repo/a/src/lib.rs", 1, 5)]);
        let diff = DiffEstruturado {
            arquivos: vec![untracked("/repo/a/src/solto.rs", 9)],
        };
        let m = mapear_diff(&diff, &grafo, &[PathBuf::from("/repo/a")]);
        assert_eq!(m.soltos, vec![PathBuf::from("/repo/a/src/solto.rs")]);
        assert!(m.ligados.is_empty() && m.nao_fonte.is_empty());
        assert!(m.tocados.is_empty(), "solto não está no grafo → não toca nada");
    }

    #[test]
    fn untracked_fora_de_membro_ou_nao_rs_vai_para_nao_fonte() {
        let grafo = grafo_de(vec![no_pos(1, "a::x", "/repo/a/src/lib.rs", 1, 5)]);
        let diff = DiffEstruturado {
            arquivos: vec![
                untracked("/repo/docs/README.md", 3), // não-.rs
                untracked("/repo/fora/y.rs", 4),       // .rs fora de membro
            ],
        };
        let m = mapear_diff(&diff, &grafo, &[PathBuf::from("/repo/a")]);
        assert_eq!(
            m.nao_fonte,
            vec![
                PathBuf::from("/repo/docs/README.md"),
                PathBuf::from("/repo/fora/y.rs"),
            ]
        );
        assert!(m.ligados.is_empty() && m.soltos.is_empty());
    }

    #[test]
    fn mapeamento_e_deterministico() {
        let grafo = grafo_de(vec![
            no_pos(2, "a::b", "/repo/a/src/lib.rs", 10, 20),
            no_pos(1, "a", "/repo/a/src/lib.rs", 1, 40),
        ]);
        let diff = DiffEstruturado {
            arquivos: vec![
                rastreado("/repo/a/src/lib.rs", &[(12, 14)]),
                untracked("/repo/a/src/solto.rs", 2),
            ],
        };
        let dirs = [PathBuf::from("/repo/a")];
        let m1 = mapear_diff(&diff, &grafo, &dirs);
        let m2 = mapear_diff(&diff, &grafo, &dirs);
        assert_eq!(m1, m2);
    }

    #[test]
    fn no_sem_position_nunca_e_tocado() {
        let mut n = no_pos(1, "a::foo", "/repo/a/src/lib.rs", 10, 20);
        n.position = None;
        let grafo = grafo_de(vec![n]);
        let diff = DiffEstruturado {
            arquivos: vec![rastreado("/repo/a/src/lib.rs", &[(10, 20)])],
        };
        let m = mapear_diff(&diff, &grafo, &[]);
        assert!(m.tocados.is_empty());
    }
}
