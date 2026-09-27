//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/infra-extracao.md
//! @prompt-hash d5dc2bc3
//! @layer L3
//! @updated 2026-09-26
//! Adaptadores explícitos; seleção e composição pertencem ao L4 (ADR-0007).

use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::fork::ErroFork;
use crate::{CacheExtracao, ErroAdaptador, ErroWorkspace, MembroWorkspace};
use lente_core::domain::extracao::{ExtracaoGrafo, IdentidadeProvedor};
use lente_core::entities::grafo::Grafo;

/// Detalhes de infraestrutura ficam nesta camada, nunca no núcleo.
pub enum SolicitacaoExtracao<'a> {
    Ferramenta(&'a crate::provedores::PedidoFerramenta),
    Json(&'a str),
    Memoria(&'a Grafo),
    RustPacote(&'a str),
    RustDiretorio(&'a Path),
    RustMembro {
        membro: &'a MembroWorkspace,
        raiz: &'a Path,
        contexto_rust: &'a str,
    },
}

impl SolicitacaoExtracao<'_> {
    fn contexto(&self) -> String {
        match self {
            Self::Ferramenta(p) => format!("ferramenta {}", p.provedor),
            Self::Json(_) => "JSON fornecido".into(),
            Self::Memoria(g) => format!("grafo em memória: {}", g.crate_name),
            Self::RustPacote(p) => format!("pacote {p}"),
            Self::RustDiretorio(p) => format!("diretório {}", p.display()),
            Self::RustMembro { membro, raiz, .. } => format!(
                "membro {} em {} (workspace {})",
                membro.nome,
                membro.dir.display(),
                raiz.display(),
            ),
        }
    }
}

/// Interface estática injetável; não carrega bibliotecas nem descobre plugins.
pub trait ExtratorGrafo {
    fn extrair(
        &mut self,
        solicitacao: SolicitacaoExtracao<'_>,
    ) -> Result<ExtracaoGrafo, ErroProvedor>;
}

#[derive(Debug)]
pub enum CausaProvedor {
    Ferramenta(crate::provedores::ErroFerramenta),
    Fork(ErroFork),
    Adaptador(ErroAdaptador),
    Workspace(ErroWorkspace),
    SolicitacaoIncompativel,
}

impl fmt::Display for CausaProvedor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ferramenta(e) => e.fmt(f),
            Self::Fork(e) => e.fmt(f),
            Self::Adaptador(e) => e.fmt(f),
            Self::Workspace(e) => e.fmt(f),
            Self::SolicitacaoIncompativel => f.write_str("solicitação incompatível com o provedor"),
        }
    }
}

impl Error for CausaProvedor {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Ferramenta(e) => Some(e),
            Self::Fork(e) => Some(e),
            Self::Adaptador(e) => Some(e),
            Self::Workspace(e) => Some(e),
            Self::SolicitacaoIncompativel => None,
        }
    }
}

#[derive(Debug)]
pub struct ErroProvedor {
    pub provedor: IdentidadeProvedor,
    pub contexto: String,
    pub causa: CausaProvedor,
}

impl fmt::Display for ErroProvedor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "provedor {:?}, {}: {}",
            self.provedor, self.contexto, self.causa
        )
    }
}

impl Error for ErroProvedor {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.causa)
    }
}

fn concluir(
    provedor: IdentidadeProvedor,
    contexto: String,
    resultado: Result<Grafo, CausaProvedor>,
) -> Result<ExtracaoGrafo, ErroProvedor> {
    resultado
        .map(|grafo| ExtracaoGrafo {
            grafo,
            provedor,
            diagnosticos: vec![],
        })
        .map_err(|causa| ErroProvedor {
            provedor,
            contexto,
            causa,
        })
}

#[derive(Default)]
pub struct ExtratorRust {
    cache: CacheExtracao,
}

impl ExtratorGrafo for ExtratorRust {
    fn extrair(
        &mut self,
        solicitacao: SolicitacaoExtracao<'_>,
    ) -> Result<ExtracaoGrafo, ErroProvedor> {
        executar_rust(solicitacao, |pedido| match pedido {
            SolicitacaoExtracao::RustPacote(p) => {
                let json = crate::fork::invocar_fork(p).map_err(CausaProvedor::Fork)?;
                crate::desserializar_grafo(&json).map_err(CausaProvedor::Adaptador)
            }
            SolicitacaoExtracao::RustDiretorio(p) => {
                crate::extrair_grafo(p).map_err(CausaProvedor::Adaptador)
            }
            SolicitacaoExtracao::RustMembro {
                membro,
                raiz,
                contexto_rust,
            } => self
                .cache
                .extrair(membro, raiz, contexto_rust)
                .map_err(CausaProvedor::Workspace),
            _ => unreachable!("executar_rust valida antes de delegar"),
        })
    }
}

/// Uma única operação, sem sondar Cargo para descobrir/selecionar o provedor.
fn executar_rust(
    solicitacao: SolicitacaoExtracao<'_>,
    executar: impl FnOnce(SolicitacaoExtracao<'_>) -> Result<Grafo, CausaProvedor>,
) -> Result<ExtracaoGrafo, ErroProvedor> {
    let contexto = solicitacao.contexto();
    let resultado = match solicitacao {
        SolicitacaoExtracao::RustPacote(_)
        | SolicitacaoExtracao::RustDiretorio(_)
        | SolicitacaoExtracao::RustMembro { .. } => executar(solicitacao),
        _ => Err(CausaProvedor::SolicitacaoIncompativel),
    };
    concluir(IdentidadeProvedor::RustCargo, contexto, resultado)
}

pub struct ExtratorJson;

impl ExtratorGrafo for ExtratorJson {
    fn extrair(
        &mut self,
        solicitacao: SolicitacaoExtracao<'_>,
    ) -> Result<ExtracaoGrafo, ErroProvedor> {
        let contexto = solicitacao.contexto();
        let resultado = match solicitacao {
            SolicitacaoExtracao::Json(json) => {
                crate::desserializar_grafo(json).map_err(CausaProvedor::Adaptador)
            }
            _ => Err(CausaProvedor::SolicitacaoIncompativel),
        };
        concluir(IdentidadeProvedor::Json, contexto, resultado)
    }
}

/// O chamador fornece um Grafo que satisfaz os invariantes de entrada da spec.
pub struct ExtratorMemoria;

impl ExtratorGrafo for ExtratorMemoria {
    fn extrair(
        &mut self,
        solicitacao: SolicitacaoExtracao<'_>,
    ) -> Result<ExtracaoGrafo, ErroProvedor> {
        let contexto = solicitacao.contexto();
        let resultado = match solicitacao {
            SolicitacaoExtracao::Memoria(grafo) => Ok(grafo.clone()),
            _ => Err(CausaProvedor::SolicitacaoIncompativel),
        };
        concluir(IdentidadeProvedor::Memoria, contexto, resultado)
    }
}

pub fn ler_texto(caminho: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(caminho)
}

pub fn canonicalizar(caminho: &Path) -> std::io::Result<PathBuf> {
    caminho.canonicalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn grafo() -> Grafo {
        Grafo {
            crate_name: "canônico".into(),
            nodes: vec![],
            edges: vec![],
        }
    }

    #[test]
    fn json_e_memoria_preservam_grafo_e_identidade() {
        let json = r#"{"crate":"canônico","nodes":[],"edges":[]}"#;
        let j = ExtratorJson
            .extrair(SolicitacaoExtracao::Json(json))
            .unwrap();
        let m = ExtratorMemoria
            .extrair(SolicitacaoExtracao::Memoria(&grafo()))
            .unwrap();
        assert_eq!(j.grafo, m.grafo);
        assert_eq!(j.provedor, IdentidadeProvedor::Json);
        assert_eq!(m.provedor, IdentidadeProvedor::Memoria);
    }

    #[test]
    fn rust_delega_uma_vez_preservando_cada_solicitacao() {
        let membro = MembroWorkspace {
            nome: "meu-pacote".into(),
            dir: "/raiz/membro".into(),
        };
        for pedido in [
            SolicitacaoExtracao::RustPacote("meu-pacote"),
            SolicitacaoExtracao::RustDiretorio(Path::new("/raiz/membro")),
            SolicitacaoExtracao::RustMembro {
                membro: &membro,
                raiz: Path::new("/raiz"),
                contexto_rust: "rust-local",
            },
        ] {
            let mut chamadas = 0;
            let e = executar_rust(pedido, |p| {
                chamadas += 1;
                match p {
                    SolicitacaoExtracao::RustPacote(n) => assert_eq!(n, "meu-pacote"),
                    SolicitacaoExtracao::RustDiretorio(d) => assert_eq!(d, membro.dir),
                    SolicitacaoExtracao::RustMembro {
                        membro: m,
                        raiz,
                        contexto_rust,
                    } => {
                        assert_eq!(m.nome, membro.nome);
                        assert_eq!(raiz, Path::new("/raiz"));
                        assert_eq!(contexto_rust, "rust-local");
                    }
                    _ => panic!("pedido alterado"),
                }
                Ok(grafo())
            })
            .unwrap();
            assert_eq!(chamadas, 1);
            assert_eq!(e.grafo, grafo());
            assert_eq!(e.provedor, IdentidadeProvedor::RustCargo);
        }
    }

    #[test]
    fn erro_preserva_provedor_contexto_e_causa_tipados() {
        let e = executar_rust(SolicitacaoExtracao::RustPacote("ausente"), |_| {
            Err(CausaProvedor::Fork(ErroFork::StatusErro {
                codigo: Some(7),
                stderr: "causa original".into(),
            }))
        })
        .unwrap_err();
        assert_eq!(e.provedor, IdentidadeProvedor::RustCargo);
        assert!(e.to_string().contains("pacote ausente"));
        assert!(e.to_string().contains("causa original"));
        assert!(e.source().unwrap().source().unwrap().is::<ErroFork>());
        assert!(matches!(
            e.causa,
            CausaProvedor::Fork(ErroFork::StatusErro {
                codigo: Some(7),
                ..
            })
        ));
        let e = ExtratorJson
            .extrair(SolicitacaoExtracao::Json("inválido"))
            .unwrap_err();
        assert!(matches!(
            e.causa,
            CausaProvedor::Adaptador(ErroAdaptador::JsonInvalido(_))
        ));
    }

    #[test]
    fn pedido_incompativel_nao_executa_operacao() {
        let e = executar_rust(SolicitacaoExtracao::Json("{}"), |_| {
            panic!("não deve executar")
        })
        .unwrap_err();
        assert!(matches!(e.causa, CausaProvedor::SolicitacaoIncompativel));
        assert!(
            ExtratorJson
                .extrair(SolicitacaoExtracao::RustPacote("x"))
                .is_err()
        );
    }
}
