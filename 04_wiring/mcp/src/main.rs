//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/mcp.md
//! @prompt-hash 7298f739
//! @layer L4
//! @updated 2026-06-10
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
        outro => Some(erro(id, -32601, &format!("método não suportado: {}", outro))),
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
                "name": "impacto_do_diff",
                "description": "Raio de impacto ESTRUTURAL do diff da árvore de trabalho \
                    (o que mudou no git). Mostra quem depende dos itens tocados, via arestas \
                    `Uses` do grafo de dependências — quem está no raio de impacto. NÃO afirma \
                    que vai quebrar nem vê impacto comportamental; o humano julga. Devolve o \
                    mesmo JSON do `lente --diff` (tocados com raio, raio combinado, censo, \
                    fantasmas).",
                "inputSchema": {
                    "type": "object",
                    "properties": {
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
                        "pacote": { "type": "string", "description": "Nome do pacote — invoca o fork cargo-modules. Use isto OU `grafo`." },
                        "grafo": { "type": "string", "description": "Caminho de um grafo.json pronto. Use isto OU `pacote`." },
                        "alvo": { "type": "string", "description": "Path canônico do alvo (ex.: crate::mod::Item). Use isto OU `alvo_id`." },
                        "alvo_id": { "type": "integer", "description": "Id do alvo no grafo. Use isto OU `alvo`." },
                        "escopo": { "type": "string", "enum": ["completo", "seu-codigo"], "description": "completo (default): inclui stdlib. seu-codigo: esconde sysroot." }
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
                        "pacote": { "type": "string", "description": "Nome do pacote — invoca o fork. Use isto OU `grafo`." },
                        "grafo": { "type": "string", "description": "Caminho de um grafo.json pronto. Use isto OU `pacote`." },
                        "top": { "type": "integer", "description": "N do top-N. Default 10." },
                        "escopo": { "type": "string", "enum": ["completo", "seu-codigo"], "description": "completo (default) | seu-codigo." }
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
        "impacto_do_diff" => ferramenta_impacto_do_diff(args),
        "raio_do_alvo" => ferramenta_raio_do_alvo(args),
        "ranking" => ferramenta_ranking(args),
        outro => Err(format!("ferramenta desconhecida: {}", outro)),
    };
    match res {
        Ok(texto) => json!({ "content": [{ "type": "text", "text": texto }], "isError": false }),
        Err(msg) => json!({ "content": [{ "type": "text", "text": msg }], "isError": true }),
    }
}

fn ferramenta_impacto_do_diff(args: &Value) -> Result<String, String> {
    let raiz = match args.get("raiz").and_then(|v| v.as_str()) {
        Some(p) => std::path::PathBuf::from(p),
        None => std::env::current_dir().map_err(|e| format!("não obtive o cwd: {}", e))?,
    };
    analisar_diff(&raiz)
        .map(|r| formatar_diff(&r))
        .map_err(|e| format!("{}", e))
}

fn ferramenta_raio_do_alvo(args: &Value) -> Result<String, String> {
    let fonte = construir_fonte(args)?;
    let escopo = parse_escopo(args)?;
    let (alvo, pedido) = construir_alvo(args)?;
    calcular_raio_de_alvo(fonte, alvo, escopo)
        .map(|raio| {
            formatar(
                &raio,
                &pedido,
                escopo,
                &Modo { text: false, verbose: true },
            )
        })
        .map_err(|e| format!("{}", e))
}

fn ferramenta_ranking(args: &Value) -> Result<String, String> {
    let fonte = construir_fonte(args)?;
    let escopo = parse_escopo(args)?;
    let top = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    rankear_pacote(fonte, top, escopo)
        .map(|itens| formatar_ranking(&itens, escopo, &Modo { text: false, verbose: false }))
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
    match args.get("escopo").and_then(|v| v.as_str()) {
        None | Some("completo") => Ok(Escopo::Completo),
        Some("seu-codigo") => Ok(Escopo::SeuCodigo),
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
    fn tools_list_traz_as_tres_com_limite_estrutural_na_descricao() {
        let req = json!({"jsonrpc":"2.0","id":2,"method":"tools/list"});
        let resp = tratar(&req).unwrap();
        let tools = resp["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 3);
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
        assert!(matches!(parse_escopo(&json!({})), Ok(Escopo::Completo)));
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
}
