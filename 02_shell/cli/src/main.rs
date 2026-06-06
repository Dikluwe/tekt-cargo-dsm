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
use lente_wiring::{AlvoBusca, Escopo, FonteGrafo, ModoUses};

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

/// Mapeia a flag CLI para o enum forte do wiring (prompt 0030).
fn escolher_escopo(cli: &args::Cli) -> Escopo {
    if cli.filtrar_stdlib {
        Escopo::SeuCodigo
    } else {
        Escopo::Completo
    }
}

/// Mapeia a flag `--so-referencia` para o enum forte do wiring (prompt 0034).
fn escolher_modo_uses(cli: &args::Cli) -> ModoUses {
    if cli.so_referencia {
        ModoUses::SoReferencia
    } else {
        ModoUses::Todas
    }
}

/// Composição testável: a partir de `Cli`, decide e produz stdout ou erro.
fn run(cli: args::Cli) -> Result<String, SaidaErro> {
    // Modo diff (prompt 0047): opera na raiz do repo, não usa `--grafo`/
    // `--pacote`. Roteado antes de `construir_fonte` (não há fonte a construir).
    if cli.diff {
        return run_diff(&cli);
    }

    let fonte = construir_fonte(&cli)?;
    let escopo = escolher_escopo(&cli);

    // Roteamento entre modos (per-nó, ranking, estrutura). O `conflict_with`
    // do clap garante mutuamente-exclusivo; aqui só desempata o ramo.
    // O escopo (prompt 0030) é ortogonal a todos os modos.
    if cli.estrutura {
        return run_estrutura(fonte, escopo, &cli);
    }
    if cli.ranking {
        return run_ranking(fonte, escopo, &cli);
    }

    let (alvo_busca, alvo_pedido) = construir_alvo(&cli)?;
    let contexto = erro::ContextoErro {
        alvo_informado: alvo_pedido_texto(&alvo_pedido),
    };
    match lente_wiring::calcular_raio_de_alvo(fonte, alvo_busca, escopo) {
        Ok(raio) => {
            let modo = saida::Modo {
                text: cli.text,
                verbose: cli.verbose,
            };
            Ok(saida::formatar(&raio, &alvo_pedido, escopo, &modo))
        }
        Err(e) => Err(SaidaErro {
            codigo: 1,
            mensagem: erro::traduzir(&e, &contexto),
        }),
    }
}

/// Pipeline do modo estrutura (prompt 0031, ampliado pelo 0034). Sem alvo
/// (a vista é global); erros do wiring viram `SaidaErro` pela mesma
/// tradução do per-nó.
fn run_estrutura(fonte: FonteGrafo, escopo: Escopo, cli: &args::Cli) -> Result<String, SaidaErro> {
    let modo_uses = escolher_modo_uses(cli);
    let contexto = erro::ContextoErro {
        alvo_informado: String::new(),
    };
    match lente_wiring::analisar_estrutura(fonte, escopo, modo_uses) {
        Ok(estrut) => {
            let modo = saida::Modo {
                text: cli.text,
                verbose: cli.verbose,
            };
            Ok(saida::formatar_estrutura(&estrut, escopo, modo_uses, &modo))
        }
        Err(e) => Err(SaidaErro {
            codigo: 1,
            mensagem: erro::traduzir(&e, &contexto),
        }),
    }
}

/// Pipeline do modo ranking. Erros do wiring viram `SaidaErro` pela mesma
/// tradução do per-nó (`erro::traduzir`), com contexto vazio para alvo
/// (não há alvo informado no modo ranking).
fn run_ranking(fonte: FonteGrafo, escopo: Escopo, cli: &args::Cli) -> Result<String, SaidaErro> {
    let contexto = erro::ContextoErro {
        alvo_informado: String::new(),
    };
    match lente_wiring::rankear_pacote(fonte, cli.top, escopo) {
        Ok(itens) => {
            let modo = saida::Modo {
                text: cli.text,
                verbose: cli.verbose,
            };
            Ok(saida::formatar_ranking(&itens, escopo, &modo))
        }
        Err(e) => Err(SaidaErro {
            codigo: 1,
            mensagem: erro::traduzir(&e, &contexto),
        }),
    }
}

/// Pipeline do modo diff (prompt 0047): roda `analisar_diff` na raiz do repo
/// (`--repo` ou o diretório atual) e emite o **JSON** do resultado. Só JSON
/// neste prompt — as três vistas de texto (A/B/C) são o 0048. Erros do wiring
/// (incl. `ErroLente::Diff` do git) viram `SaidaErro` pela mesma tradução.
fn run_diff(cli: &args::Cli) -> Result<String, SaidaErro> {
    let raiz = cli
        .repo
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("."));
    let contexto = erro::ContextoErro {
        alvo_informado: String::new(),
    };
    match lente_wiring::analisar_diff(&raiz) {
        Ok(resultado) => Ok(saida::formatar_diff(&resultado)),
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
            ranking: false,
            top: 10,
            filtrar_stdlib: false,
            estrutura: false,
            so_referencia: false,
            diff: false,
            repo: None,
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
            ranking: false,
            top: 10,
            filtrar_stdlib: false,
            estrutura: false,
            so_referencia: false,
            diff: false,
            repo: None,
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
            ranking: false,
            top: 10,
            filtrar_stdlib: false,
            estrutura: false,
            so_referencia: false,
            diff: false,
            repo: None,
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
            ranking: false,
            top: 10,
            filtrar_stdlib: false,
            estrutura: false,
            so_referencia: false,
            diff: false,
            repo: None,
            text: true,
            verbose: false,
        };
        let s = run(cli).expect("E2E deve funcionar");
        // Texto humano contém os rótulos do catálogo.
        assert!(s.contains("Alvo:") || s.contains("Alvo "));
        assert!(s.contains("Classificação:"));
    }

    // ---- Modo ranking (prompt 0027) -----------------------------------------

    /// JSON sintético com sysroot misturado, igual ao do wiring — confirma
    /// que o roteamento CLI → wiring → filtro → ranking funciona ponta a
    /// ponta, e que o ranking sai sem sysroot.
    fn json_com_stdlib_e_alvo() -> &'static str {
        r#"{"crate":"t","nodes":[
            {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
            {"id":10,"path":"t::T","name":"T","kind":"struct","visibility":"pub"},
            {"id":20,"path":"t::T::fmt","name":"fmt","kind":"fn","visibility":"priv","trait":"Display","trait_ref":"Display"},
            {"id":30,"path":"t::user_a","name":"user_a","kind":"fn","visibility":"pub"},
            {"id":31,"path":"t::user_b","name":"user_b","kind":"fn","visibility":"pub"},
            {"id":32,"path":"t::user_c","name":"user_c","kind":"fn","visibility":"pub"},
            {"id":100,"path":"core::fmt::Display","name":"Display","kind":"trait","visibility":"pub"}
        ],"edges":[
            {"from":"t","id_from":1,"to":"t::T","id_to":10,"relation":"owns"},
            {"from":"t::T","id_from":10,"to":"t::T::fmt","id_to":20,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::user_a","id_to":30,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::user_b","id_to":31,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::user_c","id_to":32,"relation":"owns"},
            {"from":"t::user_a","id_from":30,"to":"t::T::fmt","id_to":20,"relation":"uses"},
            {"from":"t::user_b","id_from":31,"to":"t::T::fmt","id_to":20,"relation":"uses"},
            {"from":"t::user_c","id_from":32,"to":"t::T::fmt","id_to":20,"relation":"uses"}
        ]}"#
    }

    /// Pós-0030: `--filtrar-stdlib` no ranking esconde sysroot (cenário do laudo 0027).
    #[test]
    fn ranking_json_filtrado_sai_sem_sysroot() {
        let p = escrever_json_temp(json_com_stdlib_e_alvo());
        let mut cli = cli_padrao(p);
        cli.ranking = true;
        cli.filtrar_stdlib = true;
        cli.top = 5;
        let s = run(cli).expect("ranking deve rodar");
        assert!(s.contains("\"ranking\":"));
        assert!(s.contains("\"escopo\":\"seu-codigo\""));
        assert!(!s.contains("core::fmt::Display"));
        assert!(s.contains("\"path\":\"t::T::fmt\""));
        assert!(s.contains("\"impacto\":3"));
        assert!(s.contains("\"posicao\":1"));
    }

    /// Default novo (pós-0030): ranking sem `--filtrar-stdlib` traz sysroot.
    /// Esperado, declarado — corrige o Achado 2 do laudo 0029 tornando o
    /// escopo explícito em vez de o filtro estar embutido em silêncio.
    #[test]
    fn ranking_json_default_completo_traz_sysroot_e_escopo_declarado() {
        let p = escrever_json_temp(json_com_stdlib_e_alvo());
        let mut cli = cli_padrao(p);
        cli.ranking = true;
        cli.top = 10;
        let s = run(cli).expect("ranking default deve rodar");
        assert!(s.contains("\"ranking\":"));
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(
            s.contains("\"path\":\"core::fmt::Display\""),
            "default Completo deve trazer sysroot; veio: {}",
            s
        );
    }

    #[test]
    fn ranking_text_tem_cabecalho_e_linhas_e_escopo() {
        let p = escrever_json_temp(json_com_stdlib_e_alvo());
        let mut cli = cli_padrao(p);
        cli.ranking = true;
        cli.text = true;
        cli.top = 3;
        let s = run(cli).expect("ranking texto deve rodar");
        assert!(s.contains("Ranking de impacto"));
        // Pós-0030: o escopo aparece no cabeçalho do ranking-texto.
        assert!(s.contains("escopo: completo"));
        assert!(s.contains("Impacto"));
        assert!(s.contains("t::T::fmt"));
    }

    /// Pós-0030: o modo per-nó também declara o escopo. Verifica o caso
    /// per-path no JSON default (Completo).
    #[test]
    fn raio_por_path_default_declara_escopo_completo() {
        let p = escrever_json_temp(json_com_stdlib_e_alvo());
        let mut cli = cli_padrao(p);
        cli.alvo = Some("t::T::fmt".to_string());
        let s = run(cli).expect("raio per-nó deve rodar");
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"alvo\":\"t::T::fmt\""));
    }

    #[test]
    fn raio_por_path_filtrado_declara_escopo_seu_codigo() {
        let p = escrever_json_temp(json_com_stdlib_e_alvo());
        let mut cli = cli_padrao(p);
        cli.alvo = Some("t::T::fmt".to_string());
        cli.filtrar_stdlib = true;
        let s = run(cli).expect("raio per-nó filtrado deve rodar");
        assert!(s.contains("\"escopo\":\"seu-codigo\""));
    }

    // ---- Modo estrutura (prompt 0031) ----------------------------------------

    /// JSON sintético com ciclo `t::a ↔ t::b` (via uses entre itens nos
    /// dois módulos).
    fn json_estrutura_com_ciclo() -> &'static str {
        r#"{"crate":"t","nodes":[
            {"id":1,"path":"t","name":"t","kind":"crate","visibility":"pub"},
            {"id":10,"path":"t::a","name":"a","kind":"mod","visibility":"pub"},
            {"id":11,"path":"t::a::f","name":"f","kind":"fn","visibility":"pub"},
            {"id":20,"path":"t::b","name":"b","kind":"mod","visibility":"pub"},
            {"id":21,"path":"t::b::g","name":"g","kind":"fn","visibility":"pub"}
        ],"edges":[
            {"from":"t","id_from":1,"to":"t::a","id_to":10,"relation":"owns"},
            {"from":"t::a","id_from":10,"to":"t::a::f","id_to":11,"relation":"owns"},
            {"from":"t","id_from":1,"to":"t::b","id_to":20,"relation":"owns"},
            {"from":"t::b","id_from":20,"to":"t::b::g","id_to":21,"relation":"owns"},
            {"from":"t::a::f","id_from":11,"to":"t::b::g","id_to":21,"relation":"uses"},
            {"from":"t::b::g","id_from":21,"to":"t::a::f","id_to":11,"relation":"uses"}
        ]}"#
    }

    #[test]
    fn estrutura_json_lista_modulos_e_ciclos() {
        let p = escrever_json_temp(json_estrutura_com_ciclo());
        let mut cli = cli_padrao(p);
        cli.estrutura = true;
        let s = run(cli).expect("estrutura deve rodar");
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"modulos\":[\"t\",\"t::a\",\"t::b\"]"));
        assert!(s.contains("\"de\":\"t::a\""));
        assert!(s.contains("\"para\":\"t::b\""));
        assert!(s.contains("\"ciclos\":[[\"t::a\",\"t::b\"]]"));
    }

    #[test]
    fn estrutura_texto_destaca_ciclo() {
        let p = escrever_json_temp(json_estrutura_com_ciclo());
        let mut cli = cli_padrao(p);
        cli.estrutura = true;
        cli.text = true;
        let s = run(cli).expect("estrutura texto deve rodar");
        assert!(s.contains("Estrutura de módulos"));
        assert!(s.contains("Ciclos:"));
        assert!(s.contains("t::a, t::b"));
        assert!(s.contains("Dependências módulo → módulo:"));
        assert!(s.contains("t::a → t::b"));
    }

    /// E2E real (prompt 0031): `lente --pacote lente_core --estrutura --text`
    /// roda ponta-a-ponta e mostra "nenhum ciclo" (lente_core é
    /// cuidadoso — não tem ciclos entre módulos).
    #[test]
    #[ignore]
    fn e2e_estrutura_lente_core_texto() {
        let cli = args::Cli {
            grafo: None,
            pacote: Some("lente_core".to_string()),
            alvo: None,
            alvo_id: None,
            ranking: false,
            top: 10,
            filtrar_stdlib: false,
            estrutura: true,
            so_referencia: false,
            diff: false,
            repo: None,
            text: true,
            verbose: false,
        };
        let s = run(cli).expect("E2E estrutura deve funcionar");
        assert!(s.contains("Estrutura de módulos"));
        assert!(s.contains("nenhum ciclo"));
    }

    /// E2E real: roda o ranking ponta-a-ponta contra o `lente_core` no
    /// escopo `SeuCodigo` (pós-0030 precisa de `--filtrar-stdlib` explícito
    /// para que sysroot saia — antes do 0030 o filtro era o default).
    /// Confirma o caminho CLI → wiring → filtro → ranking → saída.
    #[test]
    #[ignore]
    fn e2e_ranking_lente_core_texto() {
        let cli = args::Cli {
            grafo: None,
            pacote: Some("lente_core".to_string()),
            alvo: None,
            alvo_id: None,
            ranking: true,
            top: 10,
            filtrar_stdlib: true,
            estrutura: false,
            so_referencia: false,
            diff: false,
            repo: None,
            text: true,
            verbose: false,
        };
        let s = run(cli).expect("E2E ranking deve funcionar");
        assert!(s.contains("Ranking de impacto"));
        // Sysroot fora. Comparamos o **path** de cada linha de ranking (último
        // campo após o alinhamento), não substring solta — `lente_core::*` tem
        // "core::" como substring, então `s.contains("core::")` é falso
        // positivo. Pegamos o que vem depois dos 4 espaços de indentação do
        // formato `"  {pos:>2}  {imp:>7}  {classif:<15}  {path}"`.
        for linha in s.lines() {
            // Linhas de dado começam com 2 espaços + número.
            let trim = linha.trim_start();
            if trim.is_empty() || !trim.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            // O path é o último "token" da linha (após várias colunas).
            let path = linha.split_whitespace().last().unwrap_or("");
            let primeiro = path.split("::").next().unwrap_or("");
            assert!(
                !matches!(primeiro, "core" | "std" | "alloc" | "proc_macro" | "test"),
                "sysroot vazou no ranking: linha {:?}",
                linha
            );
        }
    }
}
