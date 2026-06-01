//! Lineage: prompt 00_nucleo/prompt/0020-l2-cli.md
//! Camada:  L2 — Casca (CLI). Nasce sob o Tekt ADR-0002: zero literais de
//!          apresentação fora do `lente_catalogo`.
//!
//! Binário `lente`. O `main` é fino — só chama `Cli::parse()` e `run()`,
//! traduz o resultado em código de saída e canal certo (stdout/stderr).
//! Toda a lógica de composição está em `run`.

mod args;
mod erro;
mod saida;

use std::process::ExitCode;

use clap::Parser;
use lente_catalogo as cat;
use lente_core::entities::grafo::Path as PathGrafo;
use lente_wiring::{AlvoBusca, FonteGrafo};

fn main() -> ExitCode {
    let cli = args::Cli::parse();
    match run(cli) {
        Ok(s) => {
            println!("{}", s);
            ExitCode::SUCCESS
        }
        Err(SaidaErro { codigo, mensagem }) => {
            eprintln!("{}", mensagem);
            ExitCode::from(codigo)
        }
    }
}

/// Saída de erro estruturada: código (1=pipeline, 2=args/validação) +
/// mensagem já traduzida.
#[derive(Debug)]
pub struct SaidaErro {
    pub codigo: u8,
    pub mensagem: String,
}

fn err_args(t: lente_catalogo::Template) -> SaidaErro {
    SaidaErro {
        codigo: 2,
        mensagem: t.render(&[]),
    }
}

fn err_arquivo(path: &std::path::Path, e: std::io::Error) -> SaidaErro {
    SaidaErro {
        codigo: 1,
        mensagem: cat::ERRO_LER_ARQUIVO.render(&[
            ("arquivo", &path.to_string_lossy()),
            ("detalhe", &e.to_string()),
        ]),
    }
}

/// Composição testável: a partir de `Cli`, decide e produz stdout ou erro.
fn run(cli: args::Cli) -> Result<String, SaidaErro> {
    let fonte = construir_fonte(&cli)?;
    let (alvo_busca, alvo_pedido) = construir_alvo(&cli)?;

    let contexto = erro::ContextoErro {
        alvo_informado: alvo_pedido_texto(&alvo_pedido),
    };

    match lente_wiring::calcular_raio_de_alvo(fonte, alvo_busca) {
        Ok(raio) => {
            let modo = saida::Modo {
                text: cli.text,
                verbose: cli.verbose,
            };
            Ok(saida::formatar(&raio, &alvo_pedido, &modo))
        }
        Err(e) => Err(SaidaErro {
            codigo: 1,
            mensagem: erro::traduzir(&e, &contexto),
        }),
    }
}

fn construir_fonte(cli: &args::Cli) -> Result<FonteGrafo, SaidaErro> {
    match (&cli.grafo, &cli.pacote) {
        (Some(path), None) => {
            let conteudo = std::fs::read_to_string(path)
                .map_err(|e| err_arquivo(path, e))?;
            Ok(FonteGrafo::Json(conteudo))
        }
        (None, Some(p)) => Ok(FonteGrafo::Pacote(p.clone())),
        (None, None) => Err(err_args(cat::ERRO_FONTE_NAO_INFORMADA)),
        // clap impede (--grafo + --pacote) via conflicts_with — defesa em profundidade
        (Some(_), Some(_)) => Err(err_args(cat::ERRO_FONTE_NAO_INFORMADA)),
    }
}

fn construir_alvo(cli: &args::Cli) -> Result<(AlvoBusca, saida::AlvoPedido), SaidaErro> {
    match (&cli.alvo, cli.alvo_id) {
        (Some(p), None) => Ok((
            AlvoBusca::PorPath(PathGrafo::from(p.as_str())),
            saida::AlvoPedido::Path(p.clone()),
        )),
        (None, Some(id)) => Ok((AlvoBusca::PorId(id), saida::AlvoPedido::Id(id))),
        (None, None) => Err(err_args(cat::ERRO_ALVO_NAO_INFORMADO)),
        (Some(_), Some(_)) => Err(err_args(cat::ERRO_ALVO_NAO_INFORMADO)),
    }
}

fn alvo_pedido_texto(alvo: &saida::AlvoPedido) -> String {
    match alvo {
        saida::AlvoPedido::Path(p) => p.clone(),
        saida::AlvoPedido::Id(id) => format!("id={}", id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JSON sintético com colisão Display+Debug (mesma estrutura do teste do
    /// L4 wiring, mas como string para a CLI consumir).
    fn json_sintetico() -> &'static str {
        r#"{"crate":"t","nodes":[
            {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
            {"id":10,"path":"t::T","name":"T","kind":"struct","visibility":"pub"},
            {"id":20,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"priv","trait":"Display","trait_ref":"Display"},
            {"id":21,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"pub","trait":"Debug","trait_ref":"Debug"},
            {"id":30,"path":"t::user_a","name":"user_a","kind":"fn","visibility":"pub"},
            {"id":31,"path":"t::user_b","name":"user_b","kind":"fn","visibility":"pub"}
        ],"edges":[
            {"from":"t","id_from":1,"to":"t::T","id_to":10,"relation":"owns"},
            {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":20,"relation":"owns"},
            {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":21,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::user_a","id_to":30,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::user_b","id_to":31,"relation":"owns"},
            {"from":"t::user_a","id_from":30,"to":"t::T::fmt","id_to":20,"relation":"uses"},
            {"from":"t::user_b","id_from":31,"to":"t::T::fmt","id_to":21,"relation":"uses"}
        ]}"#
    }

    /// Gera path único por chamada — testes paralelos não colidem.
    fn escrever_json_temp(conteudo: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        let seq = SEQ.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "lente_cli_test_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("g_{}.json", seq));
        std::fs::write(&path, conteudo).unwrap();
        path
    }

    fn cli_padrao(grafo: std::path::PathBuf) -> args::Cli {
        args::Cli {
            grafo: Some(grafo),
            pacote: None,
            alvo: None,
            alvo_id: None,
            text: false,
            verbose: false,
        }
    }

    #[test]
    fn run_alvo_por_path_resolvido_gera_json_com_alvo_simples() {
        let p = escrever_json_temp(json_sintetico());
        let mut cli = cli_padrao(p);
        cli.alvo = Some("t::T::<Display>::fmt".to_string());
        let s = run(cli).expect("run deve retornar Ok");
        assert!(s.contains("\"alvo\":\"t::T::<Display>::fmt\""));
        assert!(!s.contains("alvo_pedido"));
    }

    #[test]
    fn run_alvo_por_id_mostra_traducao_id_para_path() {
        let p = escrever_json_temp(json_sintetico());
        let mut cli = cli_padrao(p);
        cli.alvo_id = Some(20);
        let s = run(cli).expect("run deve retornar Ok");
        // O id 20 (Display) foi resolvido para o path renomeado.
        assert!(s.contains("\"alvo_pedido\":\"id=20\""));
        assert!(s.contains("\"alvo_resolvido\":\"t::T::<Display>::fmt\""));
    }

    #[test]
    fn run_text_verbose_inclui_secao_de_impactados() {
        let p = escrever_json_temp(json_sintetico());
        let mut cli = cli_padrao(p);
        cli.alvo_id = Some(20);
        cli.text = true;
        cli.verbose = true;
        let s = run(cli).expect("run deve retornar Ok");
        assert!(s.contains("Alvo pedido:\tid=20"));
        assert!(s.contains("Alvo resolvido:\tt::T::<Display>::fmt"));
        assert!(s.contains("Impactados:"));
    }

    #[test]
    fn run_alvo_inexistente_propaga_erro_com_mensagem_amigavel() {
        let p = escrever_json_temp(json_sintetico());
        let mut cli = cli_padrao(p);
        cli.alvo = Some("nao::existe".to_string());
        let err = run(cli).unwrap_err();
        assert_eq!(err.codigo, 1);
        assert!(err.mensagem.contains("Alvo 'nao::existe' não existe no grafo"));
    }

    #[test]
    fn run_sem_alvo_e_sem_alvo_id_da_erro_de_validacao() {
        let p = escrever_json_temp(json_sintetico());
        let cli = cli_padrao(p);
        let err = run(cli).unwrap_err();
        assert_eq!(err.codigo, 2);
        assert!(err.mensagem.contains("Informe --alvo"));
    }

    #[test]
    fn run_sem_grafo_e_sem_pacote_da_erro_de_validacao() {
        let mut cli = args::Cli {
            grafo: None,
            pacote: None,
            alvo: Some("foo".to_string()),
            alvo_id: None,
            text: false,
            verbose: false,
        };
        cli.alvo = Some("foo".to_string());
        let err = run(cli).unwrap_err();
        assert_eq!(err.codigo, 2);
        assert!(err.mensagem.contains("--grafo"));
        assert!(err.mensagem.contains("--pacote"));
    }

    #[test]
    fn run_arquivo_inexistente_da_erro_de_pipeline() {
        let cli = args::Cli {
            grafo: Some(std::path::PathBuf::from("/tmp/__naoexiste_lente_cli__.json")),
            pacote: None,
            alvo: Some("x".to_string()),
            alvo_id: None,
            text: false,
            verbose: false,
        };
        let err = run(cli).unwrap_err();
        assert_eq!(err.codigo, 1);
        assert!(err.mensagem.contains("Não foi possível ler"));
    }

    /// E2E real: invoca o fork e roda o pipeline contra o `lente_core`.
    /// Requer fork 0.27.0 instalado e `cargo test` rodado da raiz.
    #[test]
    #[ignore]
    fn e2e_pacote_lente_core_alvo_erro_raio_funciona() {
        let cli = args::Cli {
            grafo: None,
            pacote: Some("lente_core".to_string()),
            alvo: Some("lente_core::domain::raio::ErroRaio".to_string()),
            alvo_id: None,
            text: true,
            verbose: false,
        };
        let s = run(cli).expect("E2E deve funcionar");
        // Texto humano contém os rótulos do catálogo.
        assert!(s.contains("Alvo:") || s.contains("Alvo "));
        assert!(s.contains("Classificação:"));
    }
}
