//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/mcp.md
//! @prompt-hash f6b96945
//! @layer L4
//! @updated 2026-09-26
//!
//! Boca **MCP** da lente (prompt 0070; Momento B da proposta §4): um servidor
//! por **stdio** que anuncia ferramentas e as executa, para um agente perguntar
//! "o que quebra se eu mexer aqui?" antes de propor, e o humano ver antes de
//! aprovar. Ponto de entrada **L4** (precedente 0057): compõe os pipelines do
//! `lente_wiring` com a montagem JSON do `lente_cli` — **o mesmo contrato** que
//! a CLI emite, sem duplicar.
//!
//! Protocolo: **JSON-RPC 2.0 à mão** sobre stdio (linhas), sem SDK nem async —
//! a superfície mínima que passa um cliente real, e síncrona como os pipelines
//! (cada chamada roda o fork; um runtime async não compraria nada). Razão
//! completa no laudo 0070.
//!
//! **stdout é sagrado**: só mensagens do protocolo. Qualquer log vai a stderr.

use serde_json::{Value, json};
use std::io::{self, BufRead, Write};

use lente_cli::saida::{AlvoPedido, Modo, formatar, formatar_diff, formatar_ranking};
use lente_core::domain::consulta::{AlvoBusca, Escopo, FonteGrafo};
use lente_core::entities::grafo::Path as PathGrafo;
use lente_wiring::{analisar_diff, calcular_raio_de_alvo, rankear_pacote};

/// Versão de protocolo que anunciamos quando o cliente não pede uma.
const PROTOCOLO_PADRAO: &str = "2025-06-18";

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for linha in stdin.lock().lines() {
        let linha = match linha {
            Ok(l) => l,
            Err(_) => break, // EOF / stream fechado: o cliente encerrou (shutdown stdio).
        };
        if linha.trim().is_empty() {
            continue;
        }
        let req: Value = match serde_json::from_str(&linha) {
            Ok(v) => v,
            Err(e) => {
                // Erro de parse JSON-RPC (sem id recuperável): -32700.
                let resp = erro(Value::Null, -32700, &format!("JSON inválido: {}", e));
                escrever(&mut out, &resp);
                continue;
            }
        };
        if let Some(resp) = tratar(&req) {
            escrever(&mut out, &resp);
        }
    }
}

fn escrever(out: &mut impl Write, resp: &Value) {
    // Uma mensagem por linha (framing do stdio MCP). stdout só leva protocolo.
    let _ = writeln!(out, "{}", resp);
    let _ = out.flush();
}

/// Roteia uma mensagem JSON-RPC. `None` = nada a responder (notificação).
fn tratar(req: &Value) -> Option<Value> {
    let metodo = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
    // Notificação: sem `id`, não se responde (ex.: `notifications/initialized`).
    let id = req.get("id")?.clone();

    match metodo {
        "initialize" => Some(ok(id, resultado_initialize(req))),
        "tools/list" => Some(ok(id, lista_ferramentas())),
        "tools/call" => Some(ok(id, chamar_ferramenta(req))),
        "ping" => Some(ok(id, json!({}))),
        outro => Some(erro(
            id,
            -32601,
            &format!("método não suportado: {}", outro),
        )),
    }
}

fn resultado_initialize(req: &Value) -> Value {
    // Negociação de versão (spec): se suportamos a pedida, ecoamos a mesma;
    // não havendo pedida, anunciamos a nossa. Nossa superfície não depende da
    // versão, então ecoar a do cliente é o caminho mais compatível.
    let versao = req
        .get("params")
        .and_then(|p| p.get("protocolVersion"))
        .and_then(|v| v.as_str())
        .unwrap_or(PROTOCOLO_PADRAO);
    json!({
        "protocolVersion": versao,
        "capabilities": { "tools": {} },
        "serverInfo": { "name": "lente-mcp", "version": env!("CARGO_PKG_VERSION") }
    })
}

fn ok(id: Value, resultado: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": resultado })
}

fn erro(id: Value, codigo: i64, msg: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": codigo, "message": msg } })
}

/// A honestidade da proposta §3 vai no **contrato** da ferramenta — o agente
/// decide chamar pelo texto, então o limite estrutural-não-comportamental está
/// aqui, não só na doc.
fn lista_ferramentas() -> Value {
    json!({
        "tools": [
            {
                "name": "analisar_artefatos",
                "description": "Executa análise ESTRUTURAL de snapshots, regras, DSM ou enriquecimento documental Doxygen sobre arquivos existentes. NÃO comprova correção comportamental. Preserva cobertura e diagnósticos. Violação e inconclusivo são resultados com codigo_saida 1/2, não erros de transporte. Não inicia servidor DSM/Doxygen. Doxygen parcial é resultado válido; doxygen-grafo dispensa executável.",
                "inputSchema": {
                    "type":"object", "additionalProperties":false,
                    "required":["timeout_ms","consulta"],
                    "properties":{
                        "executavel":{"type":"string","description":"Executável preparado, caminho absoluto."},
                        "timeout_ms":{"type":"integer","minimum":1,"maximum":3600000},
                        "consulta":{"type":"object","required":["modo"],"properties":{
                            "modo":{"type":"string","enum":["snapshots","regras","dsm","caminho-dsm","doxygen-importar","doxygen-associar","doxygen-prioridade","doxygen-explorar","doxygen-grafo"]},
                            "xml_dir":{"type":"string"},"inventory":{"type":"string"},"catalog":{"type":"string"},"matches":{"type":"string"},"impact":{"type":"string"},"grafo":{"type":"string"},"politica_nomes":{"enum":["opaque","rust-path"]},"coverage":{"enum":["unknown","partial","complete"]},"alvos_metricas":{"type":"array","items":{"type":"integer","minimum":0}},
                            "entrada":{"type":"string"},"baseline":{"type":"string"},"atual":{"type":"string"},"regras":{"type":"string"},"data":{"type":"string"},
                            "pacote":{"type":"string"},"consumidor":{"type":"integer","minimum":0},"alterado":{"type":"integer","minimum":0}
                        },"additionalProperties":false}
                    }
                }
            },
            {
                "name": "analisar_provedor",
                "description": lente_catalogo::MCP_DESCRICAO_PROVEDOR,
                "inputSchema": {
                    "type":"object", "additionalProperties":false,
                    "required":["provedor","configuracao","consulta"],
                    "properties": {
                        "provedor":{"type":"string","enum":["python","typescript","zig","c","cpp","go","pacote-analise"]},
                        "configuracao":{
                            "type":"object", "additionalProperties":false,
                            "description":lente_catalogo::HELP_CONFIG_PROVEDOR,
                            "required":["verificador","timeout_ms","parametros"],
                            "properties":{
                                "executavel":{"type":["string","null"]}, "helper":{"type":["string","null"]},
                                "verificador":{"type":"string"}, "timeout_ms":{"type":"integer","minimum":1,"maximum":3600000},
                                "permitir_metadados_sem_vinculo":{"type":"boolean"}, "incluir_ambiente":{"type":"boolean"},
                                "parametros":{"type":"object"}
                            }
                        },
                        "consulta":{"oneOf":[
                            {"type":"object","additionalProperties":false,"required":["modo","top"],"properties":{"modo":{"const":"ranking"},"top":{"type":"integer","minimum":0}}},
                            {"type":"object","additionalProperties":false,"required":["modo","alvo"],"properties":{"modo":{"const":"raio"},"alvo":{"type":"string","minLength":1}}},
                            {"type":"object","additionalProperties":false,"required":["modo","alvo_id"],"properties":{"modo":{"const":"raio-id"},"alvo_id":{"type":"integer","minimum":0}}},
                            {"type":"object","additionalProperties":false,"required":["modo"],"properties":{"modo":{"const":"estrutura"}}}
                        ]}
                    }
                }
            },
            {
                "name": "impacto_do_diff",
                "description": "Raio de impacto ESTRUTURAL do diff da árvore de trabalho \
                    (o que mudou no git). Mostra quem depende dos itens tocados, via arestas \
                    `Uses` do grafo de dependências — quem está no raio de impacto. NÃO afirma \
                    que vai quebrar nem vê impacto comportamental; o humano julga. Devolve o \
                    mesmo JSON do `lente --diff` (tocados com raio, raio combinado, censo, \
                    fantasmas, completude e falhas por crate). Se parcial, os dados disponíveis \
                    não garantem ausência de impacto; nenhum crate analisado retorna erro. Escopo: `completo` (inclui sysroot/stdlib no raio) — o recorte \
                    `seu-codigo` ainda não tem parâmetro aqui (ver laudo 0072).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provedor": { "type":"string", "enum":["rust"] },
                        "raiz": { "type": "string", "description": "Raiz do repositório. Default: diretório atual do servidor." }
                    }
                }
            },
            {
                "name": "raio_do_alvo",
                "description": "Raio de impacto ESTRUTURAL de um item (módulo/tipo/função): \
                    quem depende dele (montante) e a que profundidade. É o impacto estrutural, \
                    via `Uses` — NÃO diz se vai quebrar, nem vê o raio comportamental. Devolve \
                    o mesmo JSON do `lente --alvo`.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provedor": { "type":"string", "enum":["rust"] },
                        "pacote": { "type": "string", "description": "Nome do pacote — invoca o fork cargo-modules. Use isto OU `grafo`." },
                        "grafo": { "type": "string", "description": "Caminho de um grafo.json pronto. Use isto OU `pacote`." },
                        "alvo": { "type": "string", "description": "Path canônico do alvo (ex.: crate::mod::Item). Use isto OU `alvo_id`." },
                        "alvo_id": { "type": "integer", "description": "Id do alvo no grafo. Use isto OU `alvo`." },
                        "escopo": { "type": "string", "enum": ["completo", "seu-codigo"], "description": "seu-codigo (DEFAULT — esconde sysroot/stdlib) | completo (inclui stdlib). Passe \"completo\" para o grafo cru." }
                    }
                }
            },
            {
                "name": "ranking",
                "description": "Top-N itens de um pacote por tamanho de raio de impacto \
                    ESTRUTURAL — os mais arriscados de mexer (mais dependentes), via `Uses`. \
                    NÃO é risco comportamental. Devolve o mesmo JSON do `lente --ranking`.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provedor": { "type":"string", "enum":["rust"] },
                        "pacote": { "type": "string", "description": "Nome do pacote — invoca o fork. Use isto OU `grafo`." },
                        "grafo": { "type": "string", "description": "Caminho de um grafo.json pronto. Use isto OU `pacote`." },
                        "top": { "type": "integer", "description": "N do top-N. Default 10." },
                        "escopo": { "type": "string", "enum": ["completo", "seu-codigo"], "description": "seu-codigo (DEFAULT — esconde sysroot/stdlib) | completo (inclui stdlib)." }
                    }
                }
            }
        ]
    })
}

/// Despacha `tools/call`. Erros de ferramenta voltam como resultado com
/// `isError: true` (padrão MCP: o agente vê a mensagem), não como erro JSON-RPC.
fn chamar_ferramenta(req: &Value) -> Value {
    let params = req.get("params");
    let nome = params
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("");
    let vazio = json!({});
    let args = params.and_then(|p| p.get("arguments")).unwrap_or(&vazio);

    let res = match nome {
        "analisar_artefatos" => lente_wiring::analises::analisar_artefatos(&args.to_string())
            .map(|r| r.json)
            .map_err(|e| e.to_string()),
        "analisar_provedor" => ferramenta_provedor(args),
        "impacto_do_diff" => ferramenta_impacto_do_diff(args),
        "raio_do_alvo" => ferramenta_raio_do_alvo(args),
        "ranking" => ferramenta_ranking(args),
        outro => Err(format!("ferramenta desconhecida: {}", outro)),
    };
    resposta_ferramenta(res)
}

fn resposta_ferramenta(res: Result<String, String>) -> Value {
    match res {
        Ok(texto) => json!({ "content": [{ "type": "text", "text": texto }], "isError": false }),
        Err(msg) => json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
    }
}

fn ferramenta_provedor(args: &Value) -> Result<String, String> {
    let obj = args.as_object().ok_or("parametros: esperado objeto")?;
    if obj
        .keys()
        .any(|k| !["provedor", "configuracao", "consulta"].contains(&k.as_str()))
    {
        return Err("parametros: campo desconhecido".into());
    }
    let provedor = args["provedor"]
        .as_str()
        .ok_or("parametros: falta provedor")?;
    let consulta =
        serde_json::from_value(args["consulta"].clone()).map_err(|e| format!("parametros: {e}"))?;
    lente_wiring::provedores::analisar_provedor(
        provedor,
        &args["configuracao"].to_string(),
        consulta,
    )
    .map_err(|e| e.to_string())
}

fn validar_provedor_legado(args: &Value) -> Result<(), String> {
    if args.get("provedor").is_some_and(|p| p != "rust") || args.get("configuracao").is_some() {
        return Err(lente_catalogo::MCP_ERRO_PROVEDOR_LEGADO.into());
    }
    Ok(())
}

fn ferramenta_impacto_do_diff(args: &Value) -> Result<String, String> {
    validar_provedor_legado(args)?;
    let raiz = match args.get("raiz").and_then(|v| v.as_str()) {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().map_err(|e| format!("não obtive o cwd: {}", e))?,
    };
    serializar_impacto_do_diff(analisar_diff(&raiz))
}

fn serializar_impacto_do_diff(
    resultado: Result<lente_wiring::ResultadoDiff, lente_wiring::ErroLente>,
) -> Result<String, String> {
    resultado
        .map(|r| formatar_diff(&r))
        .map_err(|e| format!("{}", e))
}

fn ferramenta_raio_do_alvo(args: &Value) -> Result<String, String> {
    validar_provedor_legado(args)?;
    let fonte = construir_fonte(args)?;
    let escopo = parse_escopo(args)?;
    let (alvo, pedido) = construir_alvo(args)?;
    calcular_raio_de_alvo(fonte, alvo, escopo)
        .map(|raio| {
            formatar(
                &raio,
                &pedido,
                escopo,
                &Modo {
                    text: false,
                    verbose: true,
                },
            )
        })
        .map_err(|e| format!("{}", e))
}

fn ferramenta_ranking(args: &Value) -> Result<String, String> {
    validar_provedor_legado(args)?;
    let fonte = construir_fonte(args)?;
    let escopo = parse_escopo(args)?;
    let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    rankear_pacote(fonte, top, escopo)
        .map(|itens| {
            formatar_ranking(
                &itens,
                escopo,
                &Modo {
                    text: false,
                    verbose: false,
                },
            )
        })
        .map_err(|e| format!("{}", e))
}

/// Mesma regra do `lente_app::construir_fonte`: `grafo` (lê o arquivo) OU
/// `pacote` (invoca o fork), exatamente um.
fn construir_fonte(args: &Value) -> Result<FonteGrafo, String> {
    let pacote = args.get("pacote").and_then(|v| v.as_str());
    let grafo = args.get("grafo").and_then(|v| v.as_str());
    match (grafo, pacote) {
        (Some(p), None) => std::fs::read_to_string(p)
            .map(FonteGrafo::Json)
            .map_err(|e| format!("não consegui ler o grafo `{}`: {}", p, e)),
        (None, Some(n)) => Ok(FonteGrafo::Pacote(n.to_string())),
        (Some(_), Some(_)) => Err("informe `pacote` OU `grafo`, não ambos".to_string()),
        (None, None) => {
            Err("informe a fonte: `pacote` (nome) ou `grafo` (arquivo.json)".to_string())
        }
    }
}

fn parse_escopo(args: &Value) -> Result<Escopo, String> {
    // Prompt 0072: superfície de consumo do agente — o default sem `escopo`
    // virou `seu-codigo` (sysroot fora; o 0070 mediu o jusante afogado em stdlib).
    // `completo` continua a um parâmetro de distância.
    match args.get("escopo").and_then(|v| v.as_str()) {
        None | Some("seu-codigo") => Ok(Escopo::SeuCodigo),
        Some("completo") => Ok(Escopo::Completo),
        Some(outro) => Err(format!(
            "escopo inválido: `{}` (use `completo` ou `seu-codigo`)",
            outro
        )),
    }
}

fn construir_alvo(args: &Value) -> Result<(AlvoBusca, AlvoPedido), String> {
    let alvo = args.get("alvo").and_then(|v| v.as_str());
    let id = args.get("alvo_id").and_then(|v| v.as_u64());
    match (alvo, id) {
        (Some(p), None) => Ok((
            AlvoBusca::PorPath(PathGrafo::new(p)),
            AlvoPedido::Path(p.to_string()),
        )),
        (None, Some(id)) => Ok((AlvoBusca::PorId(id as usize), AlvoPedido::Id(id as usize))),
        (Some(_), Some(_)) => Err("informe `alvo` OU `alvo_id`, não ambos".to_string()),
        (None, None) => Err("informe o alvo: `alvo` (path) ou `alvo_id` (número)".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Envelope JSON-RPC (puro, sem subprocesso/fork) ---

    #[test]
    fn impacto_diff_preserva_completude_e_erro_no_envelope_mcp() {
        use lente_wiring::{ErroLente, FalhaCrate, RaioCombinado, ResultadoDiff};
        let falha = FalhaCrate {
            crate_name: "b".into(),
            motivo: "falha do fork".into(),
        };
        for falhas in [vec![], vec![falha.clone()]] {
            let r = ResultadoDiff {
                falhas: falhas.clone(),
                tocados: vec![],
                combinado: RaioCombinado {
                    montante: vec![],
                    jusante: vec![],
                },
                ligados: vec!["/a/src/lib.rs".into()],
                soltos: vec![],
                nao_fonte: vec![],
                fantasmas: vec![],
            };
            let esperado: Value = serde_json::from_str(&formatar_diff(&r)).unwrap();
            let resposta = resposta_ferramenta(serializar_impacto_do_diff(Ok(r)));
            assert_eq!(resposta["isError"], false);
            let dados: Value =
                serde_json::from_str(resposta["content"][0]["text"].as_str().unwrap()).unwrap();
            assert_eq!(dados, esperado);
            assert_eq!(
                dados["completude"],
                if falhas.is_empty() {
                    "completa"
                } else {
                    "parcial"
                }
            );
            assert_eq!(dados["falhas"].as_array().unwrap().len(), falhas.len());
            if !falhas.is_empty() {
                assert_eq!(dados["falhas"][0]["motivo"], falha.motivo);
            }
        }
        let resposta = resposta_ferramenta(serializar_impacto_do_diff(Err(
            ErroLente::NenhumCrateAnalisado(vec![falha]),
        )));
        assert_eq!(resposta["isError"], true);
        assert!(
            resposta["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("nenhum crate pôde ser analisado; b: falha do fork")
        );
    }

    #[test]
    fn initialize_ecoa_a_versao_do_cliente() {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2025-03-26"}});
        let resp = tratar(&req).expect("initialize responde");
        assert_eq!(resp["result"]["protocolVersion"], "2025-03-26");
        assert!(resp["result"]["capabilities"]["tools"].is_object());
        assert_eq!(resp["result"]["serverInfo"]["name"], "lente-mcp");
        assert_eq!(resp["id"], 1);
    }

    #[test]
    fn initialize_sem_versao_anuncia_a_padrao() {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{}});
        let resp = tratar(&req).unwrap();
        assert_eq!(resp["result"]["protocolVersion"], PROTOCOLO_PADRAO);
    }

    #[test]
    fn notificacao_sem_id_nao_responde() {
        let req = json!({"jsonrpc":"2.0","method":"notifications/initialized"});
        assert!(tratar(&req).is_none());
    }

    #[test]
    fn metodo_desconhecido_da_erro_32601() {
        let req = json!({"jsonrpc":"2.0","id":7,"method":"bananas"});
        let resp = tratar(&req).unwrap();
        assert_eq!(resp["error"]["code"], -32601);
        assert_eq!(resp["id"], 7);
    }

    #[test]
    fn tools_list_traz_as_cinco_com_limite_estrutural_na_descricao() {
        let req = json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
        let resp = tratar(&req).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 5);
        assert!(tools.iter().any(|t| t["name"] == "analisar_artefatos"));
        let nomes: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        assert!(nomes.contains(&"impacto_do_diff"));
        assert!(nomes.contains(&"raio_do_alvo"));
        assert!(nomes.contains(&"ranking"));
        // A honestidade estrutural-não-comportamental é interface: tem de estar
        // no texto de toda ferramenta.
        for t in tools {
            let d = t["description"].as_str().unwrap();
            assert!(
                d.to_uppercase().contains("ESTRUTURAL") && d.contains("NÃO"),
                "descrição de {} não declara o limite estrutural: {}",
                t["name"],
                d
            );
            assert!(t["inputSchema"]["type"] == "object");
        }
    }

    // --- Validação de argumentos (sem pipeline/fork) ---

    #[test]
    fn fonte_exige_exatamente_uma() {
        assert!(construir_fonte(&json!({})).is_err());
        assert!(construir_fonte(&json!({"pacote":"egui","grafo":"g.json"})).is_err());
        assert!(matches!(
            construir_fonte(&json!({"pacote":"egui"})),
            Ok(FonteGrafo::Pacote(p)) if p == "egui"
        ));
    }

    #[test]
    fn escopo_default_e_invalido() {
        // Re-ancorado pelo prompt 0072: o DEFAULT (sem `escopo`) virou seu-codigo
        // nesta superfície de consumo. `completo` segue a um parâmetro de distância.
        assert!(matches!(parse_escopo(&json!({})), Ok(Escopo::SeuCodigo)));
        assert!(matches!(
            parse_escopo(&json!({"escopo":"completo"})),
            Ok(Escopo::Completo)
        ));
        assert!(matches!(
            parse_escopo(&json!({"escopo":"seu-codigo"})),
            Ok(Escopo::SeuCodigo)
        ));
        assert!(parse_escopo(&json!({"escopo":"xpto"})).is_err());
    }

    #[test]
    fn alvo_exige_exatamente_um() {
        assert!(construir_alvo(&json!({})).is_err());
        assert!(construir_alvo(&json!({"alvo":"crate::a","alvo_id":3})).is_err());
        assert!(matches!(
            construir_alvo(&json!({"alvo_id":5})),
            Ok((AlvoBusca::PorId(5), AlvoPedido::Id(5)))
        ));
    }

    #[test]
    fn ferramenta_desconhecida_vira_iserror() {
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"inexistente","arguments":{}}});
        let resp = tratar(&req).unwrap();
        assert_eq!(resp["result"]["isError"], true);
        let txt = resp["result"]["content"][0]["text"].as_str().unwrap();
        assert!(txt.contains("desconhecida"));
    }

    #[test]
    fn raio_sem_fonte_vira_iserror_nao_panica() {
        // Sem fonte → erro de validação ANTES de tocar o pipeline (sem fork).
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call",
            "params":{"name":"raio_do_alvo","arguments":{"alvo":"crate::a"}}});
        let resp = tratar(&req).unwrap();
        assert_eq!(resp["result"]["isError"], true);
    }

    #[test]
    fn provedor_rejeita_argumentos_antes_de_executar() {
        let e = ferramenta_provedor(&json!({"provedor":"python","configuracao":{},"consulta":{"modo":"ranking","top":1},"grafo":"ignorado"})).unwrap_err();
        assert!(e.contains("campo desconhecido"));
        assert!(
            ferramenta_ranking(&json!({"provedor":"go"}))
                .unwrap_err()
                .contains("analisar_provedor")
        );
    }

    fn real_config(nome: &str) -> (std::path::PathBuf, Value) {
        let dir = std::env::var_os("LENTE_TEST_CONFIG_DIR")
            .expect("configure LENTE_TEST_CONFIG_DIR; veja docs/adaptadores.md");
        let file = std::path::PathBuf::from(dir).join(format!("{nome}.json"));
        let config = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        (file, config)
    }
    fn real_equivalencia(nome: &str) {
        let (file, config) = real_config(nome);
        let cli = std::env::var_os("LENTE_TEST_CLI").expect("configure LENTE_TEST_CLI");
        let output = std::process::Command::new(cli)
            .args(["--provedor", nome, "--config-provedor"])
            .arg(file)
            .args(["--ranking", "--top", "10"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let esperado: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            !esperado["analise"]["ranking"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let req = json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"analisar_provedor","arguments":{"provedor":nome,"configuracao":config,"consulta":{"modo":"ranking","top":10}}}});
        let resposta = tratar(&req).unwrap();
        assert_eq!(resposta["result"]["isError"], false, "{resposta}");
        let recebido: Value =
            serde_json::from_str(resposta["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(recebido, esperado, "CLI/MCP divergem para {nome}");
        assert_eq!(recebido["extracao"]["verificacao"]["integrity"], "verified");
        assert!(recebido["extracao"]["metadados_originais"].is_object());
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_python() {
        real_equivalencia("python");
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_typescript() {
        real_equivalencia("typescript");
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_zig() {
        real_equivalencia("zig");
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_c() {
        real_equivalencia("c");
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_cpp() {
        real_equivalencia("cpp");
    }
    #[test]
    #[ignore = "ferramenta opcional: preparar configs e executar explicitamente"]
    fn real_go() {
        real_equivalencia("go");
    }
    #[test]
    #[ignore = "ferramenta opcional Python e verificador; preparar configs"]
    fn real_parcial_preserva_dados_e_motivos() {
        let (_, config) = real_config("python-parcial");
        let res = ferramenta_provedor(&json!({"provedor":"python","configuracao":config,"consulta":{"modo":"ranking","top":10}})).unwrap();
        let r: Value = serde_json::from_str(&res).unwrap();
        assert_eq!(
            r["extracao"]["verificacao"]["summary"]["execution"]["value"],
            "partial"
        );
        assert_eq!(r["extracao"]["codigo_saida_adaptador"], 1);
        assert!(
            !r["extracao"]["metadados_originais"]["diagnostics"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert_eq!(r["analise"]["ranking"].as_array().unwrap().len(), 2);
    }

    #[test]
    #[ignore = "verificador opcional: pacote válido, adulterado e CLI/MCP reais"]
    fn real_pacote_verificado_e_adulterado() {
        let (file, config) = real_config("python");
        let dir = file.parent().unwrap();
        let pacote = dir.join(format!("pacote-teste-{}", std::process::id()));
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../lab/pacote-analise/fixtures");
        let status = std::process::Command::new(config["verificador"].as_str().unwrap())
            .arg("create")
            .arg("--graph")
            .arg(fixture.join("graph.json"))
            .arg("--metadata")
            .arg(fixture.join("metadata.json"))
            .arg("--out")
            .arg(&pacote)
            .output()
            .unwrap();
        assert!(
            status.status.success(),
            "{}",
            String::from_utf8_lossy(&status.stderr)
        );
        let config = json!({"verificador":config["verificador"],"timeout_ms":30000,"parametros":{"diretorio":pacote}});
        std::fs::write(dir.join("pacote-analise.json"), config.to_string()).unwrap();
        real_equivalencia("pacote-analise");
        std::fs::write(pacote.join("graph.json"), "{}").unwrap();
        let e = ferramenta_provedor(&json!({"provedor":"pacote-analise","configuracao":config,"consulta":{"modo":"ranking","top":10}})).unwrap_err();
        assert!(e.contains("saida_invalida"), "{e}");
        std::fs::remove_dir_all(pacote).unwrap();
    }

    #[test]
    #[ignore = "Python/verificador opcionais: consultas de raio e estrutura"]
    fn real_raio_estrutura() {
        let (file, config) = real_config("python");
        for (flags, consulta) in [
            (
                vec!["--alvo-id", "0"],
                json!({"modo":"raio-id","alvo_id":0}),
            ),
            (vec!["--estrutura"], json!({"modo":"estrutura"})),
        ] {
            let output = std::process::Command::new(std::env::var_os("LENTE_TEST_CLI").unwrap())
                .args(["--provedor", "python", "--config-provedor"])
                .arg(&file)
                .args(flags)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let cli: Value = serde_json::from_slice(&output.stdout).unwrap();
            let mcp = ferramenta_provedor(
                &json!({"provedor":"python","configuracao":config,"consulta":consulta}),
            )
            .unwrap();
            assert_eq!(cli, serde_json::from_str::<Value>(&mcp).unwrap());
        }
    }
}
