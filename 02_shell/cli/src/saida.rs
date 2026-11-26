//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/cli-saida.md
//! @prompt-hash b62f9ebf
//! @layer L2
//! @updated 2026-11-26
//!
//! Formatação do `Raio` para stdout. Quatro modos (matriz `--text` ×
//! `--verbose`); todos os literais visíveis ao usuário vêm do
//! `lente_catalogo` (ADR-0002).
//!
//! Inclui também a formatação do **modo ranking** (prompt 0027):
//! `formatar_ranking(&[ItemRanking], Escopo, &Modo)`.
//!
//! Pós-prompt 0030: a saída (JSON e texto) **declara o escopo** em ambos
//! os modos — campo `escopo` no JSON e linha/cabeçalho no texto.

use std::collections::{BTreeMap, BTreeSet};

use lente_catalogo as cat;
use lente_core::domain::raio::{Classificacao, Raio};
use lente_core::entities::grafo::{Path as PathGrafo, UsesKind};
// Estágio 1 (0055) + Estágio 2 (0056): todo este vocabulário vem agora do L1 —
// a CLi não importa mais tipos da fachada `lente_wiring` aqui.
use lente_comparacao::{ArestaComparada, Comparacao};
use lente_core::domain::consulta::{Escopo, ModoUses};
use lente_core::domain::fontes_indexadas::FontesIndexadas;
use lente_core::domain::resultado_diff::{CompletudeDiff, ResultadoDiff, TocadoComRaio};
use lente_estrutura::{AgregacaoDetalhada, DependenciaModulo, EstruturaModulos, RaioModulo};
use lente_ranking::ItemRanking;

use crate::args::Vista;

/// Mapeia `Escopo` para o texto estável publicado pela CLI (catálogo).
fn escopo_texto(e: Escopo) -> &'static str {
    match e {
        Escopo::Completo => cat::ESCOPO_COMPLETO,
        Escopo::SeuCodigo => cat::ESCOPO_SEU_CODIGO,
    }
}

/// Mapeia `ModoUses` para o texto estável publicado pela CLI (prompt 0034).
fn modo_uses_texto(m: ModoUses) -> &'static str {
    match m {
        ModoUses::Todas => cat::MODO_USES_TODAS,
        ModoUses::SoReferencia => cat::MODO_USES_SO_REFERENCIA,
    }
}

/// Como o alvo foi pedido pelo usuário — afeta se a saída mostra tradução
/// id→path ou só o alvo simples.
pub enum AlvoPedido {
    Path(String),
    Id(usize),
}

pub struct Modo {
    pub text: bool,
    pub verbose: bool,
}

pub fn formatar(raio: &Raio, alvo_pedido: &AlvoPedido, escopo: Escopo, modo: &Modo) -> String {
    if modo.text {
        formatar_texto(raio, alvo_pedido, escopo, modo.verbose)
    } else {
        formatar_json(raio, alvo_pedido, escopo, modo.verbose)
    }
}

/// Tamanho do raio em itens diretos (vizinhos imediatos) e transitivos
/// (montante completo). "Direto" = arestas Uses entrando.
fn diretos(raio: &Raio) -> usize {
    raio.uses_entrada
}
fn transitivos(raio: &Raio) -> usize {
    raio.montante.len()
}

fn classificacao_texto(c: Classificacao) -> &'static str {
    match c {
        Classificacao::Isolado => cat::CLASSIFICACAO_ISOLADO,
        Classificacao::Folha => cat::CLASSIFICACAO_FOLHA,
        Classificacao::Base => cat::CLASSIFICACAO_BASE,
        Classificacao::Intermediario => cat::CLASSIFICACAO_INTERMEDIARIO,
    }
}

/// True quando vale mostrar `alvo_pedido` + `alvo_resolvido` separados:
/// só quando o usuário pediu por id (sempre é interessante mostrar o path
/// resolvido), OU quando o path pedido bate o resolvido (caso normal: só
/// um campo `alvo`).
fn tem_traducao(alvo_pedido: &AlvoPedido) -> bool {
    matches!(alvo_pedido, AlvoPedido::Id(_))
}

fn alvo_pedido_texto(alvo: &AlvoPedido) -> String {
    match alvo {
        AlvoPedido::Path(p) => p.clone(),
        AlvoPedido::Id(id) => format!("id={}", id),
    }
}

fn formatar_json(raio: &Raio, alvo_pedido: &AlvoPedido, escopo: Escopo, verbose: bool) -> String {
    let alvo_resolvido = raio.alvo.as_str();
    let mut obj = serde_json::Map::new();

    if tem_traducao(alvo_pedido) {
        obj.insert(
            cat::JSON_ALVO_PEDIDO.to_string(),
            serde_json::Value::String(alvo_pedido_texto(alvo_pedido)),
        );
        obj.insert(
            cat::JSON_ALVO_RESOLVIDO.to_string(),
            serde_json::Value::String(alvo_resolvido.to_string()),
        );
    } else {
        obj.insert(
            cat::JSON_ALVO.to_string(),
            serde_json::Value::String(alvo_resolvido.to_string()),
        );
    }
    obj.insert(
        cat::JSON_ESCOPO.to_string(),
        serde_json::Value::String(escopo_texto(escopo).to_string()),
    );
    obj.insert(
        cat::JSON_CLASSIFICACAO.to_string(),
        serde_json::Value::String(classificacao_texto(raio.classificacao).to_string()),
    );
    obj.insert(
        cat::JSON_DIRETOS.to_string(),
        serde_json::Value::Number(diretos(raio).into()),
    );
    obj.insert(
        cat::JSON_TRANSITIVOS.to_string(),
        serde_json::Value::Number(transitivos(raio).into()),
    );
    if verbose {
        let mut paths: Vec<String> = raio
            .montante
            .keys()
            .map(|p| p.as_str().to_string())
            .collect();
        paths.sort();
        obj.insert(
            cat::JSON_IMPACTADOS.to_string(),
            serde_json::Value::Array(paths.into_iter().map(serde_json::Value::String).collect()),
        );
    }

    serde_json::Value::Object(obj).to_string()
}

fn formatar_texto(raio: &Raio, alvo_pedido: &AlvoPedido, escopo: Escopo, verbose: bool) -> String {
    let mut s = String::new();
    if tem_traducao(alvo_pedido) {
        s.push_str(&format!(
            "{}:\t{}\n",
            cat::ROTULO_ALVO_PEDIDO,
            alvo_pedido_texto(alvo_pedido)
        ));
        s.push_str(&format!("{}:\t{}\n", cat::ROTULO_ALVO_RESOLVIDO, raio.alvo));
    } else {
        s.push_str(&format!("{}:\t{}\n", cat::ROTULO_ALVO, raio.alvo));
    }
    s.push_str(&format!(
        "{}:\t{}\n",
        cat::ROTULO_ESCOPO,
        escopo_texto(escopo)
    ));
    s.push_str(&format!(
        "{}:\t{}\n",
        cat::ROTULO_CLASSIFICACAO,
        classificacao_texto(raio.classificacao)
    ));
    s.push_str(&format!(
        "{}:\t{} {}\n",
        cat::ROTULO_DIRETOS,
        diretos(raio),
        cat::SUFIXO_ITENS
    ));
    s.push_str(&format!(
        "{}:\t{} {}\n",
        cat::ROTULO_TRANSITIVOS,
        transitivos(raio),
        cat::SUFIXO_ITENS
    ));
    if verbose {
        s.push_str(&format!("\n{}:\n", cat::ROTULO_IMPACTADOS));
        let mut paths: Vec<&str> = raio.montante.keys().map(|p| p.as_str()).collect();
        paths.sort();
        for p in paths {
            s.push_str(&format!("  {}\n", p));
        }
    }
    s
}

// =============================================================================
// Modo ranking — prompt 0027
// =============================================================================

pub fn formatar_ranking(itens: &[ItemRanking], escopo: Escopo, modo: &Modo) -> String {
    if modo.text {
        formatar_ranking_texto(itens, escopo)
    } else {
        formatar_ranking_json(itens, escopo)
    }
}

fn formatar_ranking_json(itens: &[ItemRanking], escopo: Escopo) -> String {
    let mut arr = Vec::with_capacity(itens.len());
    for (i, it) in itens.iter().enumerate() {
        let mut obj = serde_json::Map::new();
        obj.insert(
            cat::JSON_POSICAO.to_string(),
            serde_json::Value::Number((i + 1).into()),
        );
        obj.insert(
            cat::JSON_PATH.to_string(),
            serde_json::Value::String(it.path.as_str().to_string()),
        );
        obj.insert(
            cat::JSON_IMPACTO.to_string(),
            serde_json::Value::Number(it.impacto.into()),
        );
        obj.insert(
            cat::JSON_CLASSIFICACAO.to_string(),
            serde_json::Value::String(classificacao_texto(it.classificacao).to_string()),
        );
        arr.push(serde_json::Value::Object(obj));
    }
    let mut root = serde_json::Map::new();
    root.insert(
        cat::JSON_ESCOPO.to_string(),
        serde_json::Value::String(escopo_texto(escopo).to_string()),
    );
    root.insert(cat::JSON_RANKING.to_string(), serde_json::Value::Array(arr));
    serde_json::Value::Object(root).to_string()
}

fn formatar_ranking_texto(itens: &[ItemRanking], escopo: Escopo) -> String {
    let mut s = String::new();
    s.push_str(&cat::RANKING_CABECALHO.render(&[
        ("escopo", escopo_texto(escopo)),
        ("n", &itens.len().to_string()),
    ]));
    s.push('\n');
    s.push_str(cat::RANKING_COLUNAS);
    s.push('\n');
    for (i, it) in itens.iter().enumerate() {
        s.push_str(&format!(
            "  {:>2}  {:>7}  {:<15}  {}\n",
            i + 1,
            it.impacto,
            classificacao_texto(it.classificacao),
            it.path.as_str()
        ));
    }
    s
}

// =============================================================================
// Modo estrutura — prompt 0031
// =============================================================================

pub fn formatar_estrutura(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    modo: &Modo,
) -> String {
    if modo.text {
        formatar_estrutura_texto(estrut, escopo, modo_uses)
    } else {
        formatar_estrutura_json(estrut, escopo, modo_uses)
    }
}

fn formatar_estrutura_json(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
) -> String {
    serde_json::Value::Object(estrutura_json_map(estrut, escopo, modo_uses)).to_string()
}

/// Monta o **mapa JSON** da estrutura (compartilhado entre `--json` e `--html`).
/// O `--html` acrescenta `pacote`/`limite` sobre este mesmo dado — uma forma.
fn estrutura_json_map(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
) -> serde_json::Map<String, serde_json::Value> {
    let mut root = serde_json::Map::new();
    root.insert(
        cat::JSON_ESCOPO.to_string(),
        serde_json::Value::String(escopo_texto(escopo).to_string()),
    );
    root.insert(
        cat::JSON_MODO_USES.to_string(),
        serde_json::Value::String(modo_uses_texto(modo_uses).to_string()),
    );
    root.insert(
        cat::JSON_MODULOS.to_string(),
        serde_json::Value::Array(
            estrut
                .modulos
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect(),
        ),
    );
    let deps: Vec<serde_json::Value> = estrut
        .dependencias
        .iter()
        .map(|d| {
            let mut o = serde_json::Map::new();
            o.insert(
                cat::JSON_DE.to_string(),
                serde_json::Value::String(d.de.as_str().to_string()),
            );
            o.insert(
                cat::JSON_PARA.to_string(),
                serde_json::Value::String(d.para.as_str().to_string()),
            );
            // Prompt 0071: peso de acoplamento (aditivo — consumidores antigos
            // ignoram o campo novo).
            o.insert(
                cat::JSON_PESO.to_string(),
                serde_json::Value::Number(d.peso.into()),
            );
            serde_json::Value::Object(o)
        })
        .collect();
    root.insert(
        cat::JSON_DEPENDENCIAS.to_string(),
        serde_json::Value::Array(deps),
    );
    let ciclos: Vec<serde_json::Value> = estrut
        .ciclos
        .iter()
        .map(|c| {
            serde_json::Value::Array(
                c.modulos
                    .iter()
                    .map(|p| serde_json::Value::String(p.as_str().to_string()))
                    .collect(),
            )
        })
        .collect();
    root.insert(
        cat::JSON_CICLOS.to_string(),
        serde_json::Value::Array(ciclos),
    );
    // Prompt 0035: a DSM como dado — ordem topológica + blocos.
    root.insert(
        cat::JSON_ORDEM.to_string(),
        serde_json::Value::Array(
            estrut
                .ordem
                .iter()
                .map(|p| serde_json::Value::String(p.as_str().to_string()))
                .collect(),
        ),
    );
    let blocos: Vec<serde_json::Value> = estrut
        .blocos
        .iter()
        .map(|b| {
            serde_json::Value::Array(
                b.iter()
                    .map(|p| serde_json::Value::String(p.as_str().to_string()))
                    .collect(),
            )
        })
        .collect();
    root.insert(
        cat::JSON_BLOCOS.to_string(),
        serde_json::Value::Array(blocos),
    );
    root
}

/// Converte os raios por módulo (paths, do L1) em **índices na `ordem`**, na
/// posição de cada módulo em `ordem` (prompt 0073): `raios[i] = {m, j}` onde
/// `m`/`j` são listas de índices. Compacto — a tela resolve índice → path pela
/// própria `ordem`.
fn raios_para_indices(estrut: &EstruturaModulos) -> serde_json::Value {
    use std::collections::HashMap;
    let idx: HashMap<&str, usize> = estrut
        .ordem
        .iter()
        .enumerate()
        .map(|(i, p)| (p.as_str(), i))
        .collect();
    let raio_de: HashMap<&str, &RaioModulo> = estrut
        .raios
        .iter()
        .map(|r| (r.modulo.as_str(), r))
        .collect();
    let to_idx = |ps: &[PathGrafo]| -> serde_json::Value {
        serde_json::Value::Array(
            ps.iter()
                .filter_map(|p| idx.get(p.as_str()))
                .map(|&i| serde_json::Value::Number(i.into()))
                .collect(),
        )
    };
    let arr: Vec<serde_json::Value> = estrut
        .ordem
        .iter()
        .map(|p| {
            let mut o = serde_json::Map::new();
            match raio_de.get(p.as_str()) {
                Some(r) => {
                    o.insert("m".to_string(), to_idx(&r.montante));
                    o.insert("j".to_string(), to_idx(&r.jusante));
                }
                None => {
                    o.insert("m".to_string(), serde_json::Value::Array(vec![]));
                    o.insert("j".to_string(), serde_json::Value::Array(vec![]));
                }
            }
            serde_json::Value::Object(o)
        })
        .collect();
    serde_json::Value::Array(arr)
}

/// Vista DSM em **HTML autocontido** (prompt 0071): injeta o MESMO dado do
/// `--json` (mais `pacote`/`limite`) no template embutido (`include_str!`). Um
/// arquivo, sem rede/CDN/deps de runtime. A tela é vista: o cálculo (ordem,
/// ciclos, peso, raio) já veio do L1.
pub fn formatar_estrutura_html(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    pacote: &str,
) -> String {
    renderizar_estrutura_html(estrut, escopo, modo_uses, pacote, None, None)
}

/// A mesma vista, acompanhada dos registros de uso disponíveis para os cartões.
pub fn formatar_estrutura_html_com_evidencias(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    pacote: &str,
    evidencias: &AgregacaoDetalhada,
) -> String {
    renderizar_estrutura_html(estrut, escopo, modo_uses, pacote, Some(evidencias), None)
}

fn posicao(local: &lente_core::domain::fontes_indexadas::LocalFonte) -> serde_json::Value {
    serde_json::json!([
        local.arquivo,
        local.linha,
        local.coluna,
        local.linha_fim,
        local.coluna_fim
    ])
}

fn acrescentar_chamadas(
    dados: &mut serde_json::Value,
    grafo: &lente_core::domain::grafo_chamadas::GrafoChamadas,
) {
    if let Some(cobertura) = &grafo.cobertura_nativa {
        dados["cobertura_chamadas_nativas"] = serde_json::json!({
            "analisadas": cobertura.analisadas,
            "falhas": cobertura.falhas,
        });
    }
    if grafo.funcoes.is_empty() {
        return;
    }
    let funcoes: Vec<_> = grafo
        .funcoes
        .iter()
        .map(|f| serde_json::json!([f.nome, posicao(&f.declaracao)]))
        .collect();
    let arestas: Vec<_> = grafo
        .chamadas
        .iter()
        .map(|c| serde_json::json!([c.de, c.para, posicao(&c.uso), c.origens]))
        .collect();
    use lente_core::domain::grafo_chamadas::MotivoSemSeta;
    let diagnosticos: Vec<_> = grafo
        .diagnosticos
        .iter()
        .map(|d| {
            let motivo = match d.motivo {
                MotivoSemSeta::SemReferencia => "sem_referencia",
                MotivoSemSeta::SemDeclaracao => "sem_declaracao",
                MotivoSemSeta::Ambigua => "ambigua",
                MotivoSemSeta::ForaDoRecorte => "fora_do_recorte",
            };
            let candidatos: Vec<_> = d
                .candidatos
                .iter()
                .map(|(local, origens)| serde_json::json!([posicao(local), origens]))
                .collect();
            serde_json::json!([d.de, posicao(&d.uso), motivo, d.origens, candidatos])
        })
        .collect();
    dados["grafo_chamadas"] = serde_json::json!({
        "funcoes": funcoes, "arestas": arestas, "diagnosticos": diagnosticos,
    });
}

/// Acrescenta ao HTML um índice de referências Go com associação explícita
/// de cada arquivo a um módulo da estrutura. A vista usa o índice para navegar.
pub fn formatar_estrutura_html_com_referencias_go(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    pacote: &str,
    evidencias: &AgregacaoDetalhada,
    indice: &str,
) -> Result<String, String> {
    formatar_estrutura_html_com_chamadas_go(
        estrut,
        escopo,
        modo_uses,
        pacote,
        evidencias,
        indice,
        &Default::default(),
    )
}

pub fn formatar_estrutura_html_com_chamadas_go(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    pacote: &str,
    evidencias: &AgregacaoDetalhada,
    indice: &str,
    grafo: &lente_core::domain::grafo_chamadas::GrafoChamadas,
) -> Result<String, String> {
    let mut referencias = referencias_go_para_html(indice, estrut)?;
    acrescentar_chamadas(&mut referencias, grafo);
    Ok(renderizar_estrutura_html(
        estrut,
        escopo,
        modo_uses,
        pacote,
        Some(evidencias),
        Some(("referencias_go", referencias)),
    ))
}

/// DSM por arquivo com referências provenientes de índices SCIP.
pub fn formatar_estrutura_html_scip(estrut: &EstruturaModulos, indice: &FontesIndexadas) -> String {
    formatar_estrutura_html_scip_com_chamadas(estrut, indice, &Default::default())
}

pub fn formatar_estrutura_html_scip_com_chamadas(
    estrut: &EstruturaModulos,
    indice: &FontesIndexadas,
    grafo: &lente_core::domain::grafo_chamadas::GrafoChamadas,
) -> String {
    formatar_indice_scip(estrut, indice, grafo, None)
}

pub fn formatar_estrutura_html_scip_com_fluxos(
    estrut: &EstruturaModulos,
    indice: &FontesIndexadas,
    grafo: &lente_core::domain::grafo_chamadas::GrafoChamadas,
    texto: &str,
) -> Result<String, String> {
    let fluxo: serde_json::Value = serde_json::from_str(texto).map_err(|e| e.to_string())?;
    let entradas = fluxo.as_array().ok_or("esperada uma lista de fluxos")?;
    let mut nomes = std::collections::BTreeSet::new();
    let local = |valor: &serde_json::Value| {
        valor.as_array().is_some_and(|partes| {
            partes.len() == 5
                && partes[0]
                    .as_str()
                    .is_some_and(|arquivo| !arquivo.is_empty())
                && partes[1..]
                    .iter()
                    .all(|p| p.as_u64().is_some_and(|v| v < (1 << 53)))
        })
    };
    for entrada in entradas {
        let nome = entrada["nome"].as_str().ok_or("fluxo sem nome")?;
        if nome.trim().is_empty() || !nomes.insert(nome) {
            return Err("nome vazio ou duplicado".into());
        }
        if !local(&entrada["raiz"]) || !matches!(entrada["lado"].as_u64(), Some(0 | 1)) {
            return Err(format!("raiz ou direção inválida em {nome}"));
        }
        let passos = entrada["passos"].as_array().ok_or("fluxo sem passos")?;
        if passos.is_empty()
            || !passos.iter().all(|passo| {
                passo
                    .as_array()
                    .is_some_and(|partes| partes.len() == 3 && partes.iter().all(local))
            })
        {
            return Err(format!("passos inválidos em {nome}"));
        }
    }
    Ok(formatar_indice_scip(estrut, indice, grafo, Some(fluxo)))
}

fn formatar_indice_scip(
    estrut: &EstruturaModulos,
    indice: &FontesIndexadas,
    grafo: &lente_core::domain::grafo_chamadas::GrafoChamadas,
    fluxos: Option<serde_json::Value>,
) -> String {
    use lente_core::domain::fontes_indexadas::OrigemFfi;
    use serde_json::json;
    let chamadas = |chamadas: &lente_core::domain::fontes_indexadas::ChamadasFonte| {
        chamadas
            .iter()
            .map(|(local, funcao)| {
                json!([
                    posicao(local),
                    funcao.as_ref().map(|f| &f.nome),
                    funcao.as_ref().map(|f| posicao(&f.declaracao)),
                ])
            })
            .collect::<Vec<_>>()
    };
    let simbolos: Vec<_> = indice
        .simbolos
        .iter()
        .map(|simbolo| {
            json!({
                "nome": simbolo.nome,
                "declaracoes": simbolo.declaracoes.iter().map(posicao).collect::<Vec<_>>(),
                "usos": simbolo.usos.iter().map(posicao).collect::<Vec<_>>(),
                "ffi": simbolo.ffi.as_ref().map(|vinculo| {
                    let mut dados = json!({
                        "nome_linker": vinculo.nome_linker,
                        "implementacao": posicao(&vinculo.implementacao),
                        "chamadas_fronteira": chamadas(&vinculo.chamadas_fronteira),
                    });
                    if vinculo.origem == OrigemFfi::IndexStore {
                        dados["origem"] = json!("indexstore");
                    }
                    dados
                }),
            })
        })
        .collect();
    let pares: Vec<_> = indice
        .pares()
        .into_iter()
        .map(|((de, para), referencias)| json!([de, para, referencias]))
        .collect();
    let mut dados = json!({
        "fontes": indice.fontes, "simbolos": simbolos, "pares": pares,
        "indexadores": indice.indexadores, "sem_declaracao": indice.usos_sem_declaracao,
        "posicoes_invalidas": indice.posicoes_invalidas,
        "artefato_ffi": indice.artefato_ffi,
        "chamadores": indice.chamadores.iter()
            .map(|(local, registros)| json!([posicao(local), chamadas(registros)]))
            .collect::<Vec<_>>(),
        "definicoes_nativas": indice.definicoes_nativas.iter()
            .map(|(local, nome)| json!([posicao(local), nome]))
            .collect::<Vec<_>>(),
    });
    acrescentar_chamadas(&mut dados, grafo);
    if !indice.prompts.is_empty() {
        dados["prompts"] = json!(
            indice
                .prompts
                .iter()
                .map(|(arquivo, prompt)| {
                    (
                        arquivo.clone(),
                        json!({"caminho": prompt.caminho, "texto": prompt.texto}),
                    )
                })
                .collect::<serde_json::Map<_, _>>()
        );
    }
    if let Some(fluxos) = fluxos {
        dados["fluxos_publicados"] = fluxos;
    }
    renderizar_estrutura_html(
        estrut,
        Escopo::Completo,
        ModoUses::Todas,
        "índices SCIP",
        None,
        Some(("scip", dados)),
    )
}

fn referencias_go_para_html(
    indice: &str,
    estrutura: &EstruturaModulos,
) -> Result<serde_json::Value, String> {
    use serde_json::Value;

    let indice: Value = serde_json::from_str(indice).map_err(|e| e.to_string())?;
    if indice["schema_version"].as_u64() != Some(1) {
        return Err("schema_version deve ser 1".into());
    }
    let files = indice["files"]
        .as_object()
        .ok_or("files deve ser um mapa de arquivos para pacotes")?;
    let modulos: BTreeSet<_> = estrutura.modulos.iter().map(|p| p.as_str()).collect();
    for (arquivo, modulo) in files {
        if !caminho_indice_go_valido(arquivo) {
            return Err(format!("caminho Go relativo inválido: {arquivo}"));
        }
        if !modulo.as_str().is_some_and(|m| modulos.contains(m)) {
            return Err(format!(
                "pacote de {arquivo} ausente na estrutura: {modulo}"
            ));
        }
    }

    let mut indexados: BTreeSet<_> = files.keys().map(String::as_str).collect();
    if let Some(extras) = indice.get("search_files") {
        let extras = extras
            .as_array()
            .ok_or("search_files deve ser uma lista de caminhos")?;
        for item in extras {
            let arquivo = item
                .as_str()
                .ok_or("search_files exige caminhos Go relativos")?;
            if !caminho_indice_go_valido(arquivo) || !indexados.insert(arquivo) {
                return Err(format!(
                    "caminho inválido ou duplicado em search_files: {arquivo}"
                ));
            }
        }
    }

    let fontes = indice
        .get("sources")
        .map(|fontes| {
            fontes
                .as_object()
                .ok_or("sources deve ser um mapa de arquivos para texto")
        })
        .transpose()?;
    if let Some(fontes) = fontes {
        for (arquivo, texto) in fontes {
            if !indexados.contains(arquivo.as_str()) || !texto.is_string() {
                return Err(format!("fonte inválida para o arquivo indexado {arquivo}"));
            }
        }
    }

    let ligacoes = ligacoes_go_para_html(&indice["links"], |arquivo| files.contains_key(arquivo))?;
    let mut dados = serde_json::json!({
        "files": files,
        "links": ligacoes,
        "configuration": indice["configuration"].as_str()
    });
    if let Some(fontes) = fontes {
        dados["sources"] = serde_json::json!(fontes);
    }
    let mut caminhos_mutantes = BTreeSet::new();
    if let Some(mutantes) = indice.get("mutant_files") {
        let mutantes = mutantes
            .as_array()
            .ok_or("mutant_files deve ser uma lista de caminhos")?;
        for item in mutantes {
            let arquivo = item
                .as_str()
                .ok_or("mutant_files exige caminhos Go relativos")?;
            if !indexados.contains(arquivo)
                || files.contains_key(arquivo)
                || !caminhos_mutantes.insert(arquivo)
            {
                return Err(format!(
                    "caminho inválido ou duplicado em mutant_files: {arquivo}"
                ));
            }
        }
        dados["mutant_files"] = serde_json::json!(caminhos_mutantes);
    }
    if let Some(comparacoes) = indice.get("mutant_comparisons") {
        let fontes = fontes.ok_or("mutant_comparisons exige as duas fontes no índice")?;
        validar_comparacoes_go(comparacoes, fontes, &caminhos_mutantes)?;
        dados["mutant_comparisons"] = comparacoes.clone();
    }
    if let Some(matriz) = indice.get("mutant_test_matrix") {
        let fontes = fontes.ok_or("mutant_test_matrix exige fontes no índice")?;
        validar_matriz_testes_go(matriz, &indice["mutant_comparisons"], fontes)?;
        dados["mutant_test_matrix"] = matriz.clone();
    }
    if let Some(links) = indice.get("search_links") {
        let referencias = ligacoes_go_para_html(links, |arquivo| indexados.contains(arquivo))?;
        dados["search_links"] = serde_json::json!(referencias);
    }
    if let Some(locais) = indice.get("local_references") {
        let locais = locais
            .as_object()
            .ok_or("local_references deve ser um mapa de arquivos para referências")?;
        let mut registros_locais = Vec::with_capacity(locais.len());
        for (arquivo, referencias) in locais {
            if !indexados.contains(arquivo.as_str()) {
                return Err(format!(
                    "arquivo ausente no índice de referências locais: {arquivo}"
                ));
            }
            let registros = registros_go_para_html(referencias, arquivo, arquivo)?;
            registros_locais.push(serde_json::json!([arquivo, arquivo, registros]));
        }
        dados["local_references"] = serde_json::json!(registros_locais);
    }
    if let Some(definicoes) = indice.get("definitions") {
        let definicoes = definicoes
            .as_object()
            .ok_or("definitions deve ser um mapa de arquivos para declarações")?;
        let mut arquivos = Vec::with_capacity(definicoes.len());
        for (arquivo, declaracoes) in definicoes {
            if !indexados.contains(arquivo.as_str()) {
                return Err(format!(
                    "arquivo ausente no índice de declarações: {arquivo}"
                ));
            }
            let declaracoes = declaracoes
                .as_array()
                .ok_or_else(|| format!("lista de declarações inválida em {arquivo}"))?;
            let mut posicoes = Vec::with_capacity(declaracoes.len());
            for d in declaracoes {
                let (Some(simbolo), Some(linha), Some(coluna)) = (
                    simbolo_indice_go(&d["symbol"]),
                    coordenada_indice_go(&d["line"]),
                    coordenada_indice_go(&d["column"]),
                ) else {
                    return Err(format!(
                        "símbolo ou posição de declaração inválida em {arquivo}"
                    ));
                };
                posicoes.push(serde_json::json!([simbolo, linha, coluna]));
            }
            arquivos.push(serde_json::json!([arquivo, posicoes]));
        }
        dados["definitions"] = serde_json::json!(arquivos);
    }
    Ok(dados)
}

fn validar_comparacoes_go(
    comparacoes: &serde_json::Value,
    fontes: &serde_json::Map<String, serde_json::Value>,
    mutantes: &BTreeSet<&str>,
) -> Result<(), String> {
    let comparacoes = comparacoes
        .as_array()
        .ok_or("mutant_comparisons deve ser uma lista de variantes")?;
    let mut rotulos = BTreeSet::new();
    let mut usados = BTreeSet::new();
    for grupo in comparacoes {
        let rotulo = simbolo_indice_go(&grupo["label"]).ok_or("variante sem label")?;
        let pares = grupo["files"]
            .as_array()
            .filter(|pares| !pares.is_empty())
            .ok_or_else(|| format!("variante sem par de arquivos: {rotulo}"))?;
        if !rotulos.insert(rotulo) {
            return Err(format!("variante duplicada: {rotulo}"));
        }
        for par in pares {
            let (Some(referencia), Some(mutante)) =
                (par["reference"].as_str(), par["mutant"].as_str())
            else {
                return Err(format!("par de arquivos inválido em {rotulo}"));
            };
            if mutantes.contains(referencia)
                || !mutantes.contains(mutante)
                || !usados.insert(mutante)
            {
                return Err(format!(
                    "referência ou mutante inválido em {rotulo}: {referencia} → {mutante}"
                ));
            }
            let fonte = |arquivo: &str| fontes.get(arquivo).and_then(serde_json::Value::as_str);
            let (Some(antes), Some(depois)) = (fonte(referencia), fonte(mutante)) else {
                return Err(format!(
                    "fonte ausente na comparação {referencia} → {mutante}"
                ));
            };
            if !blocos_go_validos(antes, depois, &par["changes"]) {
                return Err(format!(
                    "blocos não correspondem às fontes de {referencia} → {mutante}"
                ));
            }
        }
        if let Some(testes) = grupo.get("tests") {
            validar_testes_comparacao_go(testes, fontes)
                .map_err(|motivo| format!("testes de {rotulo}: {motivo}"))?;
        }
    }
    Ok(())
}

fn validar_testes_comparacao_go(
    testes: &serde_json::Value,
    fontes: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let (Some(na_referencia), Some(_), Some(falham_no_mutante)) = (
        testes["reference_passed"].as_u64(),
        testes["mutant_passed"].as_u64(),
        testes["mutant_failed"].as_u64(),
    ) else {
        return Err("contagens dos resultados inválidas".into());
    };
    if simbolo_indice_go(&testes["suite"]).is_none()
        || simbolo_indice_go(&testes["run_label"]).is_none()
    {
        return Err("suite ou origem da execução ausente".into());
    }
    let falhas = testes["failures"]
        .as_array()
        .ok_or("failures deve ser uma lista de subtestes")?;
    if falhas.len() as u64 > na_referencia.min(falham_no_mutante) {
        return Err("mais ocorrências do que os resultados permitem".into());
    }
    let mut nomes = BTreeSet::new();
    for falha in falhas {
        let (Some(nome), Some(fonte), Some(linha), Some(coluna)) = (
            simbolo_indice_go(&falha["name"]),
            falha["file"]
                .as_str()
                .and_then(|arquivo| fontes.get(arquivo))
                .and_then(serde_json::Value::as_str),
            coordenada_indice_go(&falha["line"]),
            coordenada_indice_go(&falha["column"]),
        ) else {
            return Err("subteste sem nome ou posição na fonte".into());
        };
        let linhas: Vec<_> = fonte.lines().collect();
        let texto = linhas
            .get(linha as usize - 1)
            .and_then(|l| l.get(coluna as usize - 1..));
        if !nomes.insert(nome) || !texto.is_some_and(|t| t.starts_with(nome)) {
            return Err(format!("posição do subteste inválida ou duplicada: {nome}"));
        }
        if let Some(linha_falha) = falha.get("failure_line") {
            if !coordenada_indice_go(linha_falha)
                .is_some_and(|n| n >= linha && n as usize <= linhas.len())
            {
                return Err(format!("linha da falha fora da fonte: {nome}"));
            }
        }
    }
    Ok(())
}

fn validar_matriz_testes_go(
    matriz: &serde_json::Value,
    comparacoes: &serde_json::Value,
    fontes: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let suite = simbolo_indice_go(&matriz["suite"]).ok_or("matriz sem suite")?;
    let origem = simbolo_indice_go(&matriz["run_label"]).ok_or("matriz sem origem")?;
    let casos = matriz["cases"]
        .as_array()
        .and_then(|casos| {
            casos
                .iter()
                .map(simbolo_indice_go)
                .collect::<Option<Vec<_>>>()
        })
        .filter(|casos| !casos.is_empty())
        .ok_or("matriz sem subtestes válidos")?;
    if casos.iter().collect::<BTreeSet<_>>().len() != casos.len() {
        return Err("subteste duplicado na matriz".into());
    }
    if let Some(locais) = matriz.get("case_locations") {
        validar_locais_subtestes_go(locais, &casos, fontes)?;
    }
    let referencia = estados_matriz_go(&matriz["reference"], casos.len())
        .ok_or("linha da referência inválida na matriz")?;
    let aprovados_referencia = referencia.iter().filter(|&&s| s == "pass").count() as u64;
    let variantes = matriz["variants"]
        .as_object()
        .ok_or("matriz sem linhas por variante")?;
    let grupos: Vec<_> = comparacoes
        .as_array()
        .ok_or("matriz exige comparações com resultados")?
        .iter()
        .filter(|grupo| grupo["tests"]["suite"].as_str() == Some(suite))
        .collect();
    if grupos.is_empty() || grupos.len() != variantes.len() {
        return Err("linhas da matriz não correspondem às variantes da suite".into());
    }
    for grupo in grupos {
        let rotulo = grupo["label"]
            .as_str()
            .ok_or("variante sem rótulo na matriz")?;
        let resultados = variantes
            .get(rotulo)
            .and_then(|row| estados_matriz_go(row, casos.len()))
            .ok_or_else(|| format!("linha inválida ou ausente para {rotulo}"))?;
        let testes = &grupo["tests"];
        let divergencias: BTreeSet<_> = casos
            .iter()
            .copied()
            .zip(referencia.iter().zip(&resultados))
            .filter_map(|(nome, (&antes, &depois))| {
                (antes == "pass" && depois == "fail").then_some(nome)
            })
            .collect();
        let observadas: BTreeSet<_> = testes["failures"]
            .as_array()
            .ok_or_else(|| format!("falhas não registradas para {rotulo}"))?
            .iter()
            .filter_map(|f| f["name"].as_str())
            .collect();
        if testes["run_label"].as_str() != Some(origem)
            || testes["reference_passed"].as_u64() != Some(aprovados_referencia)
            || testes["mutant_passed"].as_u64()
                != Some(resultados.iter().filter(|&&s| s == "pass").count() as u64)
            || testes["mutant_failed"].as_u64()
                != Some(resultados.iter().filter(|&&s| s == "fail").count() as u64)
            || divergencias != observadas
        {
            return Err(format!(
                "matriz diverge dos resultados registrados de {rotulo}"
            ));
        }
    }
    Ok(())
}

fn validar_locais_subtestes_go(
    locais: &serde_json::Value,
    casos: &[&str],
    fontes: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let locais = locais
        .as_object()
        .ok_or("case_locations deve ser um mapa")?;
    for (nome, local) in locais {
        if !casos.contains(&nome.as_str()) {
            return Err(format!("posição para subteste ausente da matriz: {nome}"));
        }
        let texto = local["file"]
            .as_str()
            .and_then(|arquivo| fontes.get(arquivo))
            .and_then(serde_json::Value::as_str);
        let linha =
            coordenada_indice_go(&local["line"]).and_then(|n| texto?.lines().nth(n as usize - 1));
        let literal = coordenada_indice_go(&local["column"])
            .filter(|&n| n > 1)
            .and_then(|n| linha?.get(n as usize - 2..));
        let coincide = ['"', '`'].iter().any(|&aspas| {
            literal
                .and_then(|s| s.strip_prefix(aspas)?.strip_prefix(nome))
                .is_some_and(|s| s.starts_with(aspas))
        });
        if !coincide {
            return Err(format!("posição inválida no código do subteste: {nome}"));
        }
    }
    Ok(())
}

fn estados_matriz_go(linha: &serde_json::Value, colunas: usize) -> Option<Vec<&str>> {
    let linha = linha.as_array().filter(|linha| linha.len() == colunas)?;
    linha
        .iter()
        .map(|valor| {
            valor
                .as_str()
                .filter(|s| matches!(*s, "pass" | "fail" | "skip"))
        })
        .collect()
}

fn blocos_go_validos(antes: &str, depois: &str, blocos: &serde_json::Value) -> bool {
    let Some(blocos) = blocos.as_array() else {
        return false;
    };
    let antes: Vec<_> = antes.lines().collect();
    let depois: Vec<_> = depois.lines().collect();
    let (mut pos_ref, mut pos_mut) = (0, 0);
    for bloco in blocos {
        let Ok([ini_ref, qtd_ref, ini_mut, qtd_mut]) =
            serde_json::from_value::<[usize; 4]>(bloco.clone())
        else {
            return false;
        };
        let iguais = antes
            .get(pos_ref..ini_ref)
            .zip(depois.get(pos_mut..ini_mut));
        let (Some(fim_ref), Some(fim_mut)) =
            (ini_ref.checked_add(qtd_ref), ini_mut.checked_add(qtd_mut))
        else {
            return false;
        };
        let alteradas = antes
            .get(ini_ref..fim_ref)
            .zip(depois.get(ini_mut..fim_mut));
        if !iguais.is_some_and(|(a, b)| a == b) || !alteradas.is_some_and(|(a, b)| a != b) {
            return false;
        }
        (pos_ref, pos_mut) = (fim_ref, fim_mut);
    }
    antes[pos_ref..] == depois[pos_mut..]
}

fn caminho_indice_go_valido(arquivo: &str) -> bool {
    arquivo.ends_with(".go")
        && !arquivo.contains(['\\', ':'])
        && !arquivo.chars().any(char::is_control)
        && arquivo
            .split('/')
            .all(|segmento| !matches!(segmento, "" | "." | ".."))
}

fn ligacoes_go_para_html(
    links: &serde_json::Value,
    indexado: impl Fn(&str) -> bool,
) -> Result<Vec<serde_json::Value>, String> {
    let links = links
        .as_array()
        .ok_or("links deve ser uma lista de ligações")?;
    let mut pares = BTreeSet::new();
    let mut ligacoes = Vec::with_capacity(links.len());
    for link in links {
        let (Some(de), Some(para)) = (link["from"].as_str(), link["to"].as_str()) else {
            return Err("ligação exige from e to".into());
        };
        if de == para || !indexado(de) || !indexado(para) {
            return Err(format!("arquivos inválidos na ligação {de} → {para}"));
        }
        if !pares.insert((de, para)) {
            return Err(format!("ligação duplicada: {de} → {para}"));
        }
        let registros = registros_go_para_html(&link["references"], de, para)?;
        ligacoes.push(serde_json::json!([de, para, registros]));
    }
    Ok(ligacoes)
}

fn registros_go_para_html(
    referencias: &serde_json::Value,
    de: &str,
    para: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let referencias = referencias
        .as_array()
        .filter(|r| !r.is_empty())
        .ok_or_else(|| format!("ligação sem referências: {de} → {para}"))?;
    let mut registros = Vec::with_capacity(referencias.len());
    for r in referencias {
        let (Some(simbolo), Some(linha), Some(coluna), Some(declaracao)) = (
            simbolo_indice_go(&r["symbol"]),
            coordenada_indice_go(&r["use_line"]),
            coordenada_indice_go(&r["use_column"]),
            coordenada_indice_go(&r["definition_line"]),
        ) else {
            return Err(format!("símbolo ou posição inválida em {de} → {para}"));
        };
        let registro = if r.get("definition_column").is_some() {
            let coluna_declaracao = coordenada_indice_go(&r["definition_column"])
                .ok_or_else(|| format!("coluna da declaração inválida em {de} → {para}"))?;
            serde_json::json!([simbolo, linha, coluna, declaracao, coluna_declaracao])
        } else {
            serde_json::json!([simbolo, linha, coluna, declaracao])
        };
        registros.push(registro);
    }
    Ok(registros)
}

fn simbolo_indice_go(valor: &serde_json::Value) -> Option<&str> {
    valor
        .as_str()
        .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
}

fn coordenada_indice_go(valor: &serde_json::Value) -> Option<u32> {
    valor
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
}

fn evidencias_para_html(detalhe: &AgregacaoDetalhada) -> serde_json::Value {
    const LIMITE_POR_PAR: usize = 8;
    let por_indice: BTreeMap<_, _> = detalhe
        .relacoes
        .iter()
        .map(|r| (r.indice_snapshot, &r.aresta))
        .collect();
    let pares: Vec<_> = detalhe
        .dependencias
        .iter()
        .map(|dep| {
            let registros: Vec<_> = dep
                .evidencias
                .iter()
                .take(LIMITE_POR_PAR)
                .filter_map(|indice| por_indice.get(indice))
                .map(|aresta| {
                    let subtipo = match aresta.uses_kind {
                        Some(UsesKind::Import) => Some("import"),
                        Some(UsesKind::Reference) => Some("referência"),
                        None => None,
                    };
                    serde_json::json!([aresta.from.as_str(), aresta.to.as_str(), subtipo])
                })
                .collect();
            serde_json::json!({
                "de": dep.de.as_str(),
                "para": dep.para.as_str(),
                "quantidade": dep.evidencias.len(),
                "registros": registros
            })
        })
        .collect();
    serde_json::Value::Array(pares)
}

fn renderizar_estrutura_html(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
    pacote: &str,
    evidencias: Option<&AgregacaoDetalhada>,
    navegacao: Option<(&str, serde_json::Value)>,
) -> String {
    let mut map = estrutura_json_map(estrut, escopo, modo_uses);
    map.insert(
        cat::JSON_PACOTE.to_string(),
        serde_json::Value::String(pacote.to_string()),
    );
    map.insert(
        cat::JSON_LIMITE.to_string(),
        serde_json::Value::String(cat::DSM_LIMITE_HTML.to_string()),
    );
    // Prompt 0072: quando o escopo filtrado (seu-codigo) está ativo — o default
    // da vista — o cabeçalho destaca o recorte e como desfazê-lo.
    let dica = match escopo {
        Escopo::SeuCodigo => cat::DSM_ESCOPO_DICA,
        Escopo::Completo => "",
    };
    map.insert(
        cat::JSON_ESCOPO_DICA.to_string(),
        serde_json::Value::String(dica.to_string()),
    );
    // Prompt 0073: raio por módulo embutido como ÍNDICES na `ordem` (compacto —
    // a tela já tem `ordem`; paths cheios dobrariam o arquivo). Paralelo a
    // `ordem`: raios[i] = {m:[...], j:[...]} para o módulo em ordem[i].
    map.insert(cat::JSON_RAIOS.to_string(), raios_para_indices(estrut));
    map.insert(
        cat::JSON_RAIO_SEMANTICA.to_string(),
        serde_json::Value::String(cat::DSM_RAIO_SEMANTICA.to_string()),
    );
    if let Some(evidencias) = evidencias {
        map.insert("evidencias".to_string(), evidencias_para_html(evidencias));
    }
    if let Some((chave, referencias)) = navegacao {
        map.insert(chave.to_string(), referencias);
    }
    let dados = serde_json::Value::Object(map)
        .to_string()
        .replace('<', "\\u003c");
    // Injeção única (a tela lê tudo de `DADOS`). Placeholder textual no template.
    include_str!("dsm_template.html").replace("__DADOS_JSON__", &dados)
}

fn formatar_estrutura_texto(
    estrut: &EstruturaModulos,
    escopo: Escopo,
    modo_uses: ModoUses,
) -> String {
    let mut s = String::new();
    s.push_str(&cat::ESTRUTURA_CABECALHO.render(&[
        ("escopo", escopo_texto(escopo)),
        ("modo_uses", modo_uses_texto(modo_uses)),
        ("n", &estrut.modulos.len().to_string()),
        ("c", &estrut.ciclos.len().to_string()),
    ]));
    s.push_str("\n\n");

    // Ciclos primeiro — o resultado-cabeçalho de uma ferramenta de
    // arquitetura (Lattix/Structure101): "onde estão os ciclos?".
    s.push_str(cat::ESTRUTURA_CICLOS_TITULO);
    s.push('\n');
    if estrut.ciclos.is_empty() {
        s.push_str("  ");
        s.push_str(cat::ESTRUTURA_SEM_CICLOS);
        s.push('\n');
    } else {
        for c in &estrut.ciclos {
            let nomes: Vec<&str> = c.modulos.iter().map(|p| p.as_str()).collect();
            s.push_str(&format!("  - {{ {} }}\n", nomes.join(", ")));
        }
    }

    s.push('\n');
    s.push_str(cat::ESTRUTURA_DEPENDENCIAS_TITULO);
    s.push('\n');
    for d in &estrut.dependencias {
        s.push_str(&format!("  {} → {}\n", d.de.as_str(), d.para.as_str()));
    }

    // Prompt 0035: ordem da DSM (módulos na ordem topológica + marcador
    // dos que pertencem a um bloco de ciclo). A "matriz como dado" do
    // produto — texto é a vista humana mínima; JSON tem o detalhe.
    s.push('\n');
    s.push_str(cat::ESTRUTURA_ORDEM_TITULO);
    s.push('\n');
    let em_bloco: std::collections::HashSet<&str> = estrut
        .blocos
        .iter()
        .flat_map(|b| b.iter().map(|p| p.as_str()))
        .collect();
    for p in &estrut.ordem {
        let prefixo = if em_bloco.contains(p.as_str()) {
            cat::ESTRUTURA_ORDEM_LINHA_BLOCO
        } else {
            cat::ESTRUTURA_ORDEM_LINHA_LIVRE
        };
        s.push_str(&format!("{} {}\n", prefixo, p.as_str()));
    }
    s
}

// =============================================================================
// Modo diff — prompt 0047 (JSON view-agnóstico; vistas de texto = 0048)
// =============================================================================

/// Mapeia o `ResultadoDiff` (L1) para o **JSON** view-agnóstico, à mão com
/// `serde_json::Map` — mesmo padrão da trilha global. As chaves vêm do catálogo
/// (ADR-0002). Só JSON neste prompt; as três vistas de texto são o 0048.
pub fn formatar_diff(resultado: &ResultadoDiff) -> String {
    let mut root = serde_json::Map::new();

    let completude = match resultado.completude() {
        CompletudeDiff::Completa => cat::DIFF_COMPLETA,
        CompletudeDiff::Parcial => cat::DIFF_PARCIAL,
    };
    root.insert(cat::JSON_COMPLETUDE.into(), serde_json::json!(completude));
    root.insert(
        cat::JSON_FALHAS.into(),
        serde_json::Value::Array(
            resultado
                .falhas
                .iter()
                .map(|f| {
                    serde_json::json!({
                        cat::JSON_CRATE_NAME: f.crate_name,
                        cat::JSON_MOTIVO: f.motivo,
                    })
                })
                .collect(),
        ),
    );

    // tocados: cada nó tocado + resumo do seu raio.
    let tocados: Vec<serde_json::Value> = resultado
        .tocados
        .iter()
        .map(|t| {
            let mut o = serde_json::Map::new();
            o.insert(
                cat::JSON_PATH.to_string(),
                serde_json::Value::String(t.tocado.path.as_str().to_string()),
            );
            o.insert(
                cat::JSON_ID.to_string(),
                serde_json::Value::Number(t.tocado.id.into()),
            );
            o.insert(
                cat::JSON_CLASSIFICACAO.to_string(),
                serde_json::Value::String(classificacao_texto(t.raio.classificacao).to_string()),
            );
            o.insert(
                cat::JSON_MONTANTE.to_string(),
                serde_json::Value::Number(t.raio.montante.len().into()),
            );
            o.insert(
                cat::JSON_JUSANTE.to_string(),
                serde_json::Value::Number(t.raio.jusante.len().into()),
            );
            serde_json::Value::Object(o)
        })
        .collect();
    root.insert(
        cat::JSON_TOCADOS.to_string(),
        serde_json::Value::Array(tocados),
    );

    // combinado: a união (path + profundidade).
    let mut comb = serde_json::Map::new();
    comb.insert(
        cat::JSON_MONTANTE.to_string(),
        pares_path_profundidade(&resultado.combinado.montante),
    );
    comb.insert(
        cat::JSON_JUSANTE.to_string(),
        pares_path_profundidade(&resultado.combinado.jusante),
    );
    root.insert(
        cat::JSON_COMBINADO.to_string(),
        serde_json::Value::Object(comb),
    );

    // censo do untracked.
    root.insert(
        cat::JSON_LIGADOS.to_string(),
        lista_de_paths(&resultado.ligados),
    );
    root.insert(
        cat::JSON_SOLTOS.to_string(),
        lista_de_paths(&resultado.soltos),
    );
    root.insert(
        cat::JSON_NAO_FONTE.to_string(),
        lista_de_paths(&resultado.nao_fonte),
    );

    // fantasmas (do grafo de workspace, 0045).
    let fantasmas: Vec<serde_json::Value> = resultado
        .fantasmas
        .iter()
        .map(|f| {
            let mut o = serde_json::Map::new();
            o.insert(
                cat::JSON_PATH.to_string(),
                serde_json::Value::String(f.path.as_str().to_string()),
            );
            o.insert(
                cat::JSON_REFERENCIADO_POR.to_string(),
                serde_json::Value::Array(
                    f.referenciado_por
                        .iter()
                        .map(|c| serde_json::Value::String(c.clone()))
                        .collect(),
                ),
            );
            serde_json::Value::Object(o)
        })
        .collect();
    root.insert(
        cat::JSON_FANTASMAS.to_string(),
        serde_json::Value::Array(fantasmas),
    );

    serde_json::Value::Object(root).to_string()
}

/// `[(path, profundidade)]` → array de `{path, profundidade}`.
fn pares_path_profundidade(pares: &[(PathGrafo, usize)]) -> serde_json::Value {
    serde_json::Value::Array(
        pares
            .iter()
            .map(|(p, d)| {
                let mut o = serde_json::Map::new();
                o.insert(
                    cat::JSON_PATH.to_string(),
                    serde_json::Value::String(p.as_str().to_string()),
                );
                o.insert(
                    cat::JSON_PROFUNDIDADE.to_string(),
                    serde_json::Value::Number((*d).into()),
                );
                serde_json::Value::Object(o)
            })
            .collect(),
    )
}

fn lista_de_paths(paths: &[std::path::PathBuf]) -> serde_json::Value {
    serde_json::Value::Array(
        paths
            .iter()
            .map(|p| serde_json::Value::String(p.to_string_lossy().into_owned()))
            .collect(),
    )
}

// =============================================================================
// Modo diff — vistas de texto (prompt 0048)
// =============================================================================
//
// Três renderizadores sobre o `ResultadoDiff` (0047), selecionados por
// `--vista`. Formatação **pura**: não recomputam nada; o crate de um nó é o 1º
// segmento do path. Todo texto vem do catálogo (ADR-0002). Determinísticas
// (ordenam o que iteram).

/// Roteia a vista escolhida. Sem `--vista`, o `run_diff` emite o JSON (0047).
pub fn formatar_diff_vista(resultado: &ResultadoDiff, vista: Vista) -> String {
    let mut s = match resultado.completude() {
        CompletudeDiff::Completa => cat::DIFF_ANALISE_COMPLETA.to_string(),
        CompletudeDiff::Parcial => cat::DIFF_ANALISE_PARCIAL.to_string(),
    };
    for falha in &resultado.falhas {
        s.push_str(
            &cat::DIFF_FALHA.render(&[("crate", &falha.crate_name), ("motivo", &falha.motivo)]),
        );
    }
    s.push_str(&match vista {
        Vista::Resumo => formatar_diff_resumo(resultado),
        Vista::Item => formatar_diff_item(resultado),
        Vista::Camadas => formatar_diff_camadas(resultado),
    });
    s
}

/// O crate de um path = o 1º segmento (antes do primeiro `::`).
fn crate_de(path: &str) -> &str {
    path.split("::").next().unwrap_or(path)
}

/// Nome de arquivo (último componente) de um caminho — para listar os soltos.
fn nome_arquivo(p: &std::path::Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

/// Conta os pares (path, _) por crate, ordenado por contagem desc, nome asc —
/// o mais impactante primeiro, determinístico.
fn contar_por_crate(pares: &[(PathGrafo, usize)]) -> Vec<(String, usize)> {
    let mut por_crate: BTreeMap<String, usize> = BTreeMap::new();
    for (p, _) in pares {
        *por_crate
            .entry(crate_de(p.as_str()).to_string())
            .or_insert(0) += 1;
    }
    let mut v: Vec<(String, usize)> = por_crate.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    v
}

/// `crate1 N1 · crate2 N2 · …` (ou `—` se vazio).
fn juntar_por_crate(contagens: &[(String, usize)]) -> String {
    if contagens.is_empty() {
        return cat::DIFF_VAZIO.to_string();
    }
    let sep = format!(" {} ", cat::DIFF_SEP);
    contagens
        .iter()
        .map(|(c, n)| format!("{} {}", c, n))
        .collect::<Vec<_>>()
        .join(&sep)
}

fn conta_ou_vazio(n: usize) -> String {
    if n == 0 {
        cat::DIFF_VAZIO.to_string()
    } else {
        n.to_string()
    }
}

/// Linha de impacto: `  {rótulo}: {total} — crate1 N1 · …` (sem o `—` se vazio).
fn linha_impacto(rotulo: &str, pares: &[(PathGrafo, usize)]) -> String {
    if pares.is_empty() {
        return format!("  {}: 0\n", rotulo);
    }
    format!(
        "  {}: {} — {}\n",
        rotulo,
        pares.len(),
        juntar_por_crate(&contar_por_crate(pares))
    )
}

/// Rodapé comum às três vistas: censo do untracked, solto listado (sinal
/// acionável, 0043), fantasmas só se > 0 (esperado 0, 0041).
fn rodape_diff(r: &ResultadoDiff) -> String {
    let mut s = String::new();
    s.push_str(&cat::DIFF_UNTRACKED.render(&[
        ("lig", &r.ligados.len().to_string()),
        ("solto", &r.soltos.len().to_string()),
        ("nf", &r.nao_fonte.len().to_string()),
        ("sep", cat::DIFF_SEP),
    ]));
    s.push('\n');
    if !r.soltos.is_empty() {
        let mut nomes: Vec<String> = r.soltos.iter().map(|p| nome_arquivo(p)).collect();
        nomes.sort();
        s.push_str(&format!("  {} {}\n", cat::DIFF_SEM_MOD, nomes.join(", ")));
    }
    if !r.fantasmas.is_empty() {
        s.push_str(&cat::DIFF_FANTASMAS.render(&[("n", &r.fantasmas.len().to_string())]));
        s.push('\n');
    }
    s
}

fn formatar_diff_resumo(r: &ResultadoDiff) -> String {
    let crates: BTreeSet<&str> = r
        .tocados
        .iter()
        .map(|t| crate_de(t.tocado.path.as_str()))
        .collect();
    let mut s = cat::DIFF_RESUMO_CABECALHO.render(&[
        ("n", &r.tocados.len().to_string()),
        ("c", &crates.len().to_string()),
    ]);
    s.push('\n');

    let montante = linha_impacto(cat::DIFF_ROTULO_MONTANTE, &r.combinado.montante);
    let jusante = linha_impacto(cat::DIFF_ROTULO_JUSANTE, &r.combinado.jusante);
    // Ênfase adaptativa (0043): diff só-arquivo-novo (montante vazio + ligados)
    // lidera com o jusante (o que o código novo passa a usar); senão, montante.
    if r.combinado.montante.is_empty() && !r.ligados.is_empty() {
        s.push_str(&jusante);
        s.push_str(&montante);
    } else {
        s.push_str(&montante);
        s.push_str(&jusante);
    }
    s.push_str(&rodape_diff(r));
    s
}

fn formatar_diff_item(r: &ResultadoDiff) -> String {
    let mut s = String::new();
    if r.tocados.is_empty() {
        s.push_str(cat::DIFF_SEM_TOCADOS);
        s.push('\n');
    } else {
        s.push_str(&cat::DIFF_ITEM_CABECALHO.render(&[("n", &r.tocados.len().to_string())]));
        s.push('\n');
        let mut tocados: Vec<&TocadoComRaio> = r.tocados.iter().collect();
        tocados.sort_by(|a, b| a.tocado.path.as_str().cmp(b.tocado.path.as_str()));
        for t in tocados {
            s.push_str(&format!(
                "  {}  [{}]\n",
                t.tocado.path.as_str(),
                classificacao_texto(t.raio.classificacao)
            ));
            s.push_str(&format!(
                "    {}: {}   {}: {}\n",
                cat::DIFF_ITEM_PODE_QUEBRAR,
                conta_ou_vazio(t.raio.montante.len()),
                cat::DIFF_ITEM_DEPENDE_DE,
                conta_ou_vazio(t.raio.jusante.len()),
            ));
        }
    }
    s.push_str(&rodape_diff(r));
    s
}

fn formatar_diff_camadas(r: &ResultadoDiff) -> String {
    let mut s = String::new();
    s.push_str(cat::DIFF_CAMADAS_TOCADOS_POR_CRATE);
    s.push('\n');

    // Agrupa os tocados por crate (BTreeMap → ordem alfabética determinística).
    let mut por_crate: BTreeMap<&str, Vec<&TocadoComRaio>> = BTreeMap::new();
    for t in &r.tocados {
        por_crate
            .entry(crate_de(t.tocado.path.as_str()))
            .or_default()
            .push(t);
    }
    for (crate_nome, tocados) in &por_crate {
        s.push_str(&format!("  {}\n", crate_nome));
        let mut itens: Vec<String> = tocados
            .iter()
            .map(|t| {
                format!(
                    "{} [{}]",
                    nome_curto(t.tocado.path.as_str(), crate_nome),
                    classificacao_texto(t.raio.classificacao)
                )
            })
            .collect();
        itens.sort();
        s.push_str(&format!("    {}\n", itens.join(", ")));
    }

    // Impacto cross-crate: o combinado.montante agrupado por crate.
    s.push_str(&format!(
        "{}: {}\n",
        cat::DIFF_CAMADAS_POR_CRATE,
        juntar_por_crate(&contar_por_crate(&r.combinado.montante))
    ));
    s.push_str(&rodape_diff(r));
    s
}

/// Remove o prefixo `crate::` de um path (para a vista camadas). Se o path é o
/// próprio crate-raiz, devolve-o inteiro.
fn nome_curto(path: &str, crate_nome: &str) -> String {
    path.strip_prefix(crate_nome)
        .and_then(|r| r.strip_prefix("::"))
        .unwrap_or(path)
        .to_string()
}

// =============================================================================
// Modo paridade (`--comparar`) — prompt 0074
// =============================================================================

/// Formata a [`Comparacao`] (prompt 0074): texto humano ou `--json` (contrato da
/// tela lado a lado). O escopo/modo são os MESMOS dos dois lados; o cabeçalho os
/// declara uma vez, junto do **limite do pareamento** (honestidade na saída).
pub fn formatar_comparacao(
    comp: &Comparacao,
    escopo: Escopo,
    modo_uses: ModoUses,
    modo: &Modo,
) -> String {
    if modo.text {
        formatar_comparacao_texto(comp, escopo, modo_uses)
    } else {
        formatar_comparacao_json(comp, escopo, modo_uses)
    }
}

fn formatar_comparacao_texto(comp: &Comparacao, escopo: Escopo, modo_uses: ModoUses) -> String {
    let mut s = String::new();
    s.push_str(&cat::COMPARAR_CABECALHO.render(&[
        ("antes", &comp.nome_antes),
        ("depois", &comp.nome_depois),
        ("escopo", escopo_texto(escopo)),
        ("uses", modo_uses_texto(modo_uses)),
    ]));
    s.push('\n');
    // Prompt 0075: declara modo/crates/fantasmas/chave de cada lado.
    s.push_str(&cat::COMPARAR_PROVENIENCIA.render(&[
        ("modo_antes", &comp.proveniencia.modo_antes),
        ("modo_depois", &comp.proveniencia.modo_depois),
        ("crates_antes", &comp.proveniencia.crates_antes.to_string()),
        (
            "crates_depois",
            &comp.proveniencia.crates_depois.to_string(),
        ),
        (
            "fantasmas_antes",
            &comp.proveniencia.fantasmas_antes.len().to_string(),
        ),
        (
            "fantasmas_depois",
            &comp.proveniencia.fantasmas_depois.len().to_string(),
        ),
        ("chave", &comp.chave),
    ]));
    s.push('\n');
    // Prompt 0076: third-party removido do censo (se houve) — visível, não mascarado.
    if comp.proveniencia.third_party_antes > 0 || comp.proveniencia.third_party_depois > 0 {
        s.push_str(&cat::COMPARAR_THIRD_PARTY.render(&[
            ("antes", &comp.proveniencia.third_party_antes.to_string()),
            ("depois", &comp.proveniencia.third_party_depois.to_string()),
        ]));
        s.push('\n');
    }
    // Prompt 0075: crates não extraídos (pulados) — declarados, não escondidos.
    let falhas: Vec<&String> = comp
        .proveniencia
        .falhas_antes
        .iter()
        .chain(comp.proveniencia.falhas_depois.iter())
        .collect();
    if !falhas.is_empty() {
        s.push_str(cat::COMPARAR_TIT_FALHAS);
        s.push('\n');
        for f in &falhas {
            s.push_str(&format!("  {}\n", f));
        }
    }
    s.push_str(cat::COMPARAR_LIMITE);
    s.push_str("\n\n");

    s.push_str(cat::COMPARAR_TIT_RESUMO);
    s.push('\n');
    s.push_str(&format!(
        "  pareados: {}  ·  só-antes: {}  ·  só-depois: {}\n",
        comp.pareados.len(),
        comp.sem_par_antes.len(),
        comp.sem_par_depois.len()
    ));
    s.push_str(&format!(
        "  arestas: comuns {}  ·  sumiram {}  ·  apareceram {}\n",
        comp.arestas_comuns.len(),
        comp.arestas_so_antes.len(),
        comp.arestas_so_depois.len()
    ));

    let lista = |s: &mut String, titulo: &str, paths: &[PathGrafo]| {
        if !paths.is_empty() {
            s.push('\n');
            s.push_str(titulo);
            s.push('\n');
            for p in paths {
                s.push_str(&format!("  {}\n", p.as_str()));
            }
        }
    };
    lista(&mut s, cat::COMPARAR_TIT_SEM_PAR_ANTES, &comp.sem_par_antes);
    lista(
        &mut s,
        cat::COMPARAR_TIT_SEM_PAR_DEPOIS,
        &comp.sem_par_depois,
    );

    let arestas = |s: &mut String, titulo: &str, ar: &[DependenciaModulo]| {
        if !ar.is_empty() {
            s.push('\n');
            s.push_str(titulo);
            s.push('\n');
            for d in ar {
                s.push_str(&format!(
                    "  {} → {} (peso {})\n",
                    d.de.as_str(),
                    d.para.as_str(),
                    d.peso
                ));
            }
        }
    };
    arestas(
        &mut s,
        cat::COMPARAR_TIT_ARESTAS_SO_ANTES,
        &comp.arestas_so_antes,
    );
    arestas(
        &mut s,
        cat::COMPARAR_TIT_ARESTAS_SO_DEPOIS,
        &comp.arestas_so_depois,
    );

    // Maiores deltas de peso entre as arestas comuns (|depois - antes|), top 10.
    let mut deltas: Vec<&ArestaComparada> = comp.arestas_comuns.iter().collect();
    deltas.sort_by(|a, b| {
        let da = (b.peso_depois as i64 - b.peso_antes as i64).abs();
        let db = (a.peso_depois as i64 - a.peso_antes as i64).abs();
        da.cmp(&db)
    });
    let deltas: Vec<&&ArestaComparada> = deltas
        .iter()
        .filter(|d| d.peso_antes != d.peso_depois)
        .take(10)
        .collect();
    if !deltas.is_empty() {
        s.push('\n');
        s.push_str(cat::COMPARAR_TIT_DELTAS_PESO);
        s.push('\n');
        for d in deltas {
            s.push_str(&format!(
                "  {} → {} : {} → {}\n",
                d.de.as_str(),
                d.para.as_str(),
                d.peso_antes,
                d.peso_depois
            ));
        }
    }

    s.push('\n');
    s.push_str(cat::COMPARAR_TIT_CICLOS);
    s.push('\n');
    s.push_str(&format!(
        "  antes:  {} ciclo(s), maior SCC {}\n",
        comp.ciclos_antes.quantidade, comp.ciclos_antes.maior
    ));
    s.push_str(&format!(
        "  depois: {} ciclo(s), maior SCC {}\n",
        comp.ciclos_depois.quantidade, comp.ciclos_depois.maior
    ));

    // Prompt 0078: nível de item (agregado — listas cruas vão no JSON).
    let it = &comp.itens;
    s.push('\n');
    s.push_str(cat::COMPARAR_TIT_ITENS);
    s.push('\n');
    s.push_str(&cat::COMPARAR_ITENS_RESUMO.render(&[
        ("pareados", &it.pareados.len().to_string()),
        ("ambiguos", &it.ambiguos.len().to_string()),
        ("sp_antes", &it.sem_par_antes.len().to_string()),
        ("sp_depois", &it.sem_par_depois.len().to_string()),
    ]));
    s.push('\n');
    let mut por_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for p in &it.pareados {
        *por_kind.entry(p.kind.as_str()).or_insert(0) += 1;
    }
    if !por_kind.is_empty() {
        s.push_str(cat::COMPARAR_TIT_ITENS_KIND);
        s.push_str(" (pareados)\n");
        for (k, n) in &por_kind {
            s.push_str(&format!("    {} {}\n", k, n));
        }
    }
    // Sem-par agregado por crate (1º segmento do path) — "o que falta migrar".
    let mut por_crate: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for i in &it.sem_par_antes {
        por_crate
            .entry(crate_de(&i.path).to_string())
            .or_default()
            .0 += 1;
    }
    for i in &it.sem_par_depois {
        por_crate
            .entry(crate_de(&i.path).to_string())
            .or_default()
            .1 += 1;
    }
    if !por_crate.is_empty() {
        s.push_str(cat::COMPARAR_TIT_SEM_PAR_CRATE);
        s.push('\n');
        for (c, (a, d)) in &por_crate {
            s.push_str(&format!("    {}: antes {} · depois {}\n", c, a, d));
        }
    }
    if !it.mapa.caminho.is_empty() {
        s.push('\n');
        s.push_str(cat::COMPARAR_TIT_MAPA);
        s.push('\n');
        s.push_str(&format!(
            "  {} · sha256 {}\n",
            it.mapa.caminho, it.mapa.sha256
        ));
        s.push_str(&format!(
            "  prefixos: {} regra(s), {} par(es) aplicado(s)\n",
            it.mapa.regras_prefixo,
            it.mapa.prefixos_aplicados.len()
        ));
        let aplicadas = it
            .mapa
            .correspondencias
            .iter()
            .filter(|c| c.estado == "aplicada")
            .count();
        let unknown = it.mapa.correspondencias.len() - aplicadas;
        s.push_str(&format!(
            "  correspondências: {} aplicada(s), {} unknown\n",
            aplicadas, unknown
        ));
        for d in &it.mapa.diagnosticos {
            s.push_str(&format!("  unknown: {d}\n"));
        }
    }
    s
}

fn formatar_comparacao_json(comp: &Comparacao, escopo: Escopo, modo_uses: ModoUses) -> String {
    use serde_json::Value;
    let s = |v: &str| Value::String(v.to_string());
    let n = |v: usize| Value::Number(v.into());
    let mut root = serde_json::Map::new();
    root.insert(cat::JSON_ESCOPO.to_string(), s(escopo_texto(escopo)));
    root.insert(
        cat::JSON_MODO_USES.to_string(),
        s(modo_uses_texto(modo_uses)),
    );
    root.insert(
        cat::JSON_LIMITE_PAREAMENTO.to_string(),
        s(cat::COMPARAR_LIMITE),
    );
    root.insert(cat::JSON_NOME_ANTES.to_string(), s(&comp.nome_antes));
    root.insert(cat::JSON_NOME_DEPOIS.to_string(), s(&comp.nome_depois));
    // Prompt 0075: proveniência declarada (aditivo ao esquema do 0074).
    let pv = &comp.proveniencia;
    root.insert(cat::JSON_CHAVE.to_string(), s(&comp.chave));
    root.insert(cat::JSON_MODO_ANTES.to_string(), s(&pv.modo_antes));
    root.insert(cat::JSON_MODO_DEPOIS.to_string(), s(&pv.modo_depois));
    root.insert(cat::JSON_CRATES_ANTES.to_string(), n(pv.crates_antes));
    root.insert(cat::JSON_CRATES_DEPOIS.to_string(), n(pv.crates_depois));
    let paths_de = |v: &[PathGrafo]| Value::Array(v.iter().map(|p| s(p.as_str())).collect());
    root.insert(
        cat::JSON_FANTASMAS_ANTES.to_string(),
        paths_de(&pv.fantasmas_antes),
    );
    root.insert(
        cat::JSON_FANTASMAS_DEPOIS.to_string(),
        paths_de(&pv.fantasmas_depois),
    );
    let strs_de = |v: &[String]| Value::Array(v.iter().map(|x| s(x)).collect());
    root.insert(
        cat::JSON_FALHAS_ANTES.to_string(),
        strs_de(&pv.falhas_antes),
    );
    root.insert(
        cat::JSON_FALHAS_DEPOIS.to_string(),
        strs_de(&pv.falhas_depois),
    );
    root.insert(
        cat::JSON_THIRD_PARTY_ANTES.to_string(),
        n(pv.third_party_antes),
    );
    root.insert(
        cat::JSON_THIRD_PARTY_DEPOIS.to_string(),
        n(pv.third_party_depois),
    );

    let pareados: Vec<Value> = comp
        .pareados
        .iter()
        .map(|(a, b)| {
            let mut o = serde_json::Map::new();
            o.insert(cat::JSON_DE.to_string(), s(a.as_str()));
            o.insert(cat::JSON_PARA.to_string(), s(b.as_str()));
            Value::Object(o)
        })
        .collect();
    root.insert(cat::JSON_PAREADOS.to_string(), Value::Array(pareados));

    let paths = |v: &[PathGrafo]| Value::Array(v.iter().map(|p| s(p.as_str())).collect());
    root.insert(
        cat::JSON_SEM_PAR_ANTES.to_string(),
        paths(&comp.sem_par_antes),
    );
    root.insert(
        cat::JSON_SEM_PAR_DEPOIS.to_string(),
        paths(&comp.sem_par_depois),
    );

    let comuns: Vec<Value> = comp
        .arestas_comuns
        .iter()
        .map(|a| {
            let mut o = serde_json::Map::new();
            o.insert(cat::JSON_DE.to_string(), s(a.de.as_str()));
            o.insert(cat::JSON_PARA.to_string(), s(a.para.as_str()));
            o.insert(cat::JSON_PESO_ANTES.to_string(), n(a.peso_antes));
            o.insert(cat::JSON_PESO_DEPOIS.to_string(), n(a.peso_depois));
            Value::Object(o)
        })
        .collect();
    root.insert(cat::JSON_ARESTAS_COMUNS.to_string(), Value::Array(comuns));

    let deps = |v: &[DependenciaModulo]| {
        Value::Array(
            v.iter()
                .map(|d| {
                    let mut o = serde_json::Map::new();
                    o.insert(cat::JSON_DE.to_string(), s(d.de.as_str()));
                    o.insert(cat::JSON_PARA.to_string(), s(d.para.as_str()));
                    o.insert(cat::JSON_PESO.to_string(), n(d.peso));
                    Value::Object(o)
                })
                .collect(),
        )
    };
    root.insert(
        cat::JSON_ARESTAS_SO_ANTES.to_string(),
        deps(&comp.arestas_so_antes),
    );
    root.insert(
        cat::JSON_ARESTAS_SO_DEPOIS.to_string(),
        deps(&comp.arestas_so_depois),
    );

    let ciclos = |c: &lente_comparacao::ResumoCiclos| {
        let mut o = serde_json::Map::new();
        o.insert(cat::JSON_QUANTIDADE.to_string(), n(c.quantidade));
        o.insert(cat::JSON_MAIOR.to_string(), n(c.maior));
        Value::Object(o)
    };
    root.insert(
        cat::JSON_CICLOS_ANTES.to_string(),
        ciclos(&comp.ciclos_antes),
    );
    root.insert(
        cat::JSON_CICLOS_DEPOIS.to_string(),
        ciclos(&comp.ciclos_depois),
    );

    // Prompt 0078: o nível de item completo (insumo da tela e do agente).
    root.insert(cat::JSON_ITENS.to_string(), itens_json(&comp.itens));

    Value::Object(root).to_string()
}

/// Serializa o nível de item (prompt 0078): as quatro categorias com paths.
fn itens_json(it: &lente_comparacao::ComparacaoItens) -> serde_json::Value {
    use serde_json::Value;
    let s = |v: &str| Value::String(v.to_string());
    let arr = |v: &[String]| Value::Array(v.iter().map(|x| s(x)).collect());
    let pareados: Vec<Value> = it
        .pareados
        .iter()
        .map(|p| {
            let mut o = serde_json::Map::new();
            o.insert(cat::JSON_KIND.to_string(), s(&p.kind));
            o.insert(cat::JSON_TRAIT.to_string(), s(&p.trait_));
            o.insert(cat::JSON_NOME.to_string(), s(&p.nome_qualificado));
            o.insert(cat::JSON_DE.to_string(), s(&p.path_antes));
            o.insert(cat::JSON_PARA.to_string(), s(&p.path_depois));
            Value::Object(o)
        })
        .collect();
    let ambiguos: Vec<Value> = it
        .ambiguos
        .iter()
        .map(|a| {
            let mut o = serde_json::Map::new();
            o.insert(cat::JSON_KIND.to_string(), s(&a.kind));
            o.insert(cat::JSON_TRAIT.to_string(), s(&a.trait_));
            o.insert(cat::JSON_NOME.to_string(), s(&a.nome_qualificado));
            o.insert(
                cat::JSON_CANDIDATOS_ANTES.to_string(),
                arr(&a.candidatos_antes),
            );
            o.insert(
                cat::JSON_CANDIDATOS_DEPOIS.to_string(),
                arr(&a.candidatos_depois),
            );
            Value::Object(o)
        })
        .collect();
    let sem_par = |v: &[lente_comparacao::ItemSemPar]| -> Value {
        Value::Array(
            v.iter()
                .map(|i| {
                    let mut o = serde_json::Map::new();
                    o.insert(cat::JSON_KIND.to_string(), s(&i.kind));
                    o.insert(cat::JSON_TRAIT.to_string(), s(&i.trait_));
                    o.insert(cat::JSON_NOME.to_string(), s(&i.nome_qualificado));
                    o.insert(cat::JSON_PATH.to_string(), s(&i.path));
                    Value::Object(o)
                })
                .collect(),
        )
    };
    let mut o = serde_json::Map::new();
    o.insert(cat::JSON_ITENS_PAREADOS.to_string(), Value::Array(pareados));
    o.insert(cat::JSON_ITENS_AMBIGUOS.to_string(), Value::Array(ambiguos));
    o.insert(
        cat::JSON_ITENS_SEM_PAR_ANTES.to_string(),
        sem_par(&it.sem_par_antes),
    );
    o.insert(
        cat::JSON_ITENS_SEM_PAR_DEPOIS.to_string(),
        sem_par(&it.sem_par_depois),
    );
    if !it.mapa.caminho.is_empty() {
        let rel = |r: lente_comparacao::RelacaoDeclarada| match r {
            lente_comparacao::RelacaoDeclarada::UmParaUm => "um-para-um",
            lente_comparacao::RelacaoDeclarada::Divisao => "divisao",
            lente_comparacao::RelacaoDeclarada::Consolidacao => "consolidacao",
            lente_comparacao::RelacaoDeclarada::Grupo => "grupo",
        };
        let aleg = |a: lente_comparacao::AlegacaoDeclarada| match a {
            lente_comparacao::AlegacaoDeclarada::DeclaradaFechada => "declarada-fechada",
            lente_comparacao::AlegacaoDeclarada::Parcial => "parcial",
            lente_comparacao::AlegacaoDeclarada::Ausente => "ausente",
            lente_comparacao::AlegacaoDeclarada::ForaDeEscopo => "fora-de-escopo",
            lente_comparacao::AlegacaoDeclarada::Unknown => "unknown",
        };
        let mut m = serde_json::Map::new();
        m.insert(cat::JSON_CAMINHO.to_string(), s(&it.mapa.caminho));
        m.insert(cat::JSON_SHA256.to_string(), s(&it.mapa.sha256));
        m.insert(
            cat::JSON_REGRAS_PREFIXO.to_string(),
            Value::Number(it.mapa.regras_prefixo.into()),
        );
        m.insert(
            cat::JSON_PREFIXOS_APLICADOS.to_string(),
            Value::Array(
                it.mapa
                    .prefixos_aplicados
                    .iter()
                    .map(|p| {
                        let mut x = serde_json::Map::new();
                        x.insert(cat::JSON_DE.to_string(), s(&p.path_antes));
                        x.insert(cat::JSON_PARA.to_string(), s(&p.path_depois));
                        Value::Object(x)
                    })
                    .collect(),
            ),
        );
        m.insert(
            cat::JSON_CORRESPONDENCIAS.to_string(),
            Value::Array(
                it.mapa
                    .correspondencias
                    .iter()
                    .map(|c| {
                        let mut x = serde_json::Map::new();
                        x.insert(cat::JSON_ID.to_string(), s(&c.id));
                        x.insert(cat::JSON_ANTES.to_string(), arr(&c.antes));
                        x.insert(cat::JSON_DEPOIS.to_string(), arr(&c.depois));
                        x.insert(cat::JSON_RELACAO.to_string(), s(rel(c.relacao)));
                        x.insert(cat::JSON_ALEGACAO.to_string(), s(aleg(c.alegacao)));
                        x.insert(cat::JSON_EVIDENCIAS.to_string(), arr(&c.evidencias));
                        x.insert(cat::JSON_NOTA.to_string(), s(&c.nota));
                        x.insert(cat::JSON_ESTADO.to_string(), s(&c.estado));
                        Value::Object(x)
                    })
                    .collect(),
            ),
        );
        m.insert(
            cat::JSON_DIAGNOSTICOS.to_string(),
            arr(&it.mapa.diagnosticos),
        );
        o.insert(cat::JSON_MAPA_CORRESPONDENCIA.to_string(), Value::Object(m));
    }
    Value::Object(o)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::domain::raio::{Classificacao, Raio};
    use lente_core::entities::grafo::Path;
    use std::collections::HashMap;

    fn raio_simples(alvo: &str) -> Raio {
        Raio {
            alvo: Path::from(alvo),
            classificacao: Classificacao::Folha,
            uses_entrada: 0,
            uses_saida: 0,
            montante: HashMap::new(),
            jusante: HashMap::new(),
            owns_pai: None,
            owns_filhos: Vec::new(),
        }
    }

    fn raio_com_impactados(alvo: &str, impactados: &[(&str, usize)]) -> Raio {
        let mut montante = HashMap::new();
        for (p, prof) in impactados {
            montante.insert(Path::from(*p), *prof);
        }
        Raio {
            alvo: Path::from(alvo),
            classificacao: Classificacao::Base,
            uses_entrada: impactados.len(),
            uses_saida: 0,
            montante,
            jusante: HashMap::new(),
            owns_pai: None,
            owns_filhos: Vec::new(),
        }
    }

    #[test]
    fn json_resumo_alvo_por_path_so_tem_campo_alvo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"alvo\":\"foo::bar\""));
        assert!(!s.contains("alvo_pedido"));
        assert!(!s.contains("alvo_resolvido"));
        assert!(s.contains("\"classificacao\":\"Folha\""));
        assert!(!s.contains("impactados"));
    }

    #[test]
    fn json_por_id_mostra_alvo_pedido_e_resolvido() {
        let r = raio_simples("M::T::<Display>::fmt");
        let s = formatar(
            &r,
            &AlvoPedido::Id(20),
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"alvo_pedido\":\"id=20\""));
        assert!(s.contains("\"alvo_resolvido\":\"M::T::<Display>::fmt\""));
        assert!(!s.contains("\"alvo\":")); // não duplica
    }

    #[test]
    fn json_verbose_inclui_impactados_em_ordem() {
        let r = raio_com_impactados("alvo", &[("z::user", 2), ("a::user", 1), ("m::user", 1)]);
        let s = formatar(
            &r,
            &AlvoPedido::Path("alvo".to_string()),
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: true,
            },
        );
        // ordem alfabética crescente:
        let i_a = s.find("a::user").unwrap();
        let i_m = s.find("m::user").unwrap();
        let i_z = s.find("z::user").unwrap();
        assert!(i_a < i_m && i_m < i_z, "ordem alfabética dos impactados");
    }

    #[test]
    fn texto_resumo_alvo_por_path_tem_uma_linha_de_alvo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::Completo,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Alvo:\tfoo::bar"));
        assert!(!s.contains("Alvo pedido"));
        assert!(s.contains("Classificação:\tFolha"));
        assert!(s.contains("Impacto direto:\t0 itens"));
        assert!(!s.contains("Impactados:"));
    }

    #[test]
    fn texto_por_id_tem_alvo_pedido_e_resolvido() {
        let r = raio_simples("M::T::<Debug>::fmt");
        let s = formatar(
            &r,
            &AlvoPedido::Id(47),
            Escopo::Completo,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Alvo pedido:\tid=47"));
        assert!(s.contains("Alvo resolvido:\tM::T::<Debug>::fmt"));
    }

    #[test]
    fn texto_verbose_lista_impactados_indentados() {
        let r = raio_com_impactados("alvo", &[("b", 1), ("a", 1)]);
        let s = formatar(
            &r,
            &AlvoPedido::Path("alvo".to_string()),
            Escopo::Completo,
            &Modo {
                text: true,
                verbose: true,
            },
        );
        assert!(s.contains("Impactados:"));
        assert!(s.contains("  a\n"));
        assert!(s.contains("  b\n"));
    }

    // ---- Modo ranking (prompt 0027) -----------------------------------------

    fn ranking_amostra() -> Vec<ItemRanking> {
        vec![
            ItemRanking {
                path: Path::from("alvo::base"),
                impacto: 42,
                classificacao: Classificacao::Base,
            },
            ItemRanking {
                path: Path::from("alvo::meio"),
                impacto: 7,
                classificacao: Classificacao::Intermediario,
            },
        ]
    }

    #[test]
    fn json_do_ranking_tem_array_com_posicao_path_impacto_classificacao() {
        let s = formatar_ranking(
            &ranking_amostra(),
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        // Sanidade: chave `ranking` + entradas estruturadas + escopo declarado.
        assert!(s.contains("\"ranking\":"));
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"posicao\":1"));
        assert!(s.contains("\"path\":\"alvo::base\""));
        assert!(s.contains("\"impacto\":42"));
        assert!(s.contains("\"classificacao\":\"Base\""));
        assert!(s.contains("\"posicao\":2"));
        assert!(s.contains("\"path\":\"alvo::meio\""));
    }

    #[test]
    fn texto_do_ranking_tem_cabecalho_colunas_e_linhas_alinhadas() {
        let s = formatar_ranking(
            &ranking_amostra(),
            Escopo::SeuCodigo,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Ranking de impacto"));
        // Pós-0030: o escopo aparece no cabeçalho.
        assert!(s.contains("escopo: seu-codigo"));
        assert!(s.contains("Impacto"));
        assert!(s.contains("alvo::base"));
        assert!(s.contains("alvo::meio"));
        assert!(s.contains("   1  ") || s.contains(" 1  "));
        assert!(s.contains("   2  ") || s.contains(" 2  "));
    }

    #[test]
    fn ranking_vazio_nao_panica() {
        let v: Vec<ItemRanking> = vec![];
        let _ = formatar_ranking(
            &v,
            Escopo::Completo,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        let _ = formatar_ranking(
            &v,
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
    }

    // ---- prompt 0030: saída do raio declara o escopo ------------------------

    #[test]
    fn json_do_raio_inclui_campo_escopo_completo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::Completo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"escopo\":\"completo\""));
    }

    #[test]
    fn json_do_raio_inclui_campo_escopo_seu_codigo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::SeuCodigo,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"escopo\":\"seu-codigo\""));
    }

    #[test]
    fn texto_do_raio_inclui_linha_de_escopo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::SeuCodigo,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Escopo:\tseu-codigo"));
    }

    // ---- Modo estrutura (prompt 0031) ----------------------------------------

    use lente_estrutura::{Ciclo, DependenciaModulo};

    fn estrutura_amostra() -> EstruturaModulos {
        EstruturaModulos {
            modulos: vec![Path::from("k"), Path::from("k::a"), Path::from("k::b")],
            dependencias: vec![
                DependenciaModulo {
                    de: Path::from("k::a"),
                    para: Path::from("k::b"),
                    peso: 3,
                },
                DependenciaModulo {
                    de: Path::from("k::b"),
                    para: Path::from("k::a"),
                    peso: 1,
                },
            ],
            ciclos: vec![Ciclo {
                modulos: vec![Path::from("k::a"), Path::from("k::b")],
            }],
            raios: vec![
                RaioModulo {
                    modulo: Path::from("k"),
                    montante: vec![],
                    jusante: vec![Path::from("k::a"), Path::from("k::b")],
                },
                RaioModulo {
                    modulo: Path::from("k::a"),
                    montante: vec![Path::from("k::b")],
                    jusante: vec![Path::from("k::b")],
                },
                RaioModulo {
                    modulo: Path::from("k::b"),
                    montante: vec![Path::from("k::a")],
                    jusante: vec![Path::from("k::a")],
                },
            ],
            // Prompt 0035: amostra de ordem + bloco para os testes de saída.
            // Ordem da DSM: k (crate, sem deps) → {k::a, k::b} (bloco).
            ordem: vec![Path::from("k"), Path::from("k::a"), Path::from("k::b")],
            blocos: vec![vec![Path::from("k::a"), Path::from("k::b")]],
        }
    }

    #[test]
    fn json_estrutura_tem_escopo_modulos_dependencias_ciclos() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"modo_uses\":\"todas\""));
        assert!(s.contains("\"modulos\":[\"k\",\"k::a\",\"k::b\"]"));
        assert!(s.contains("\"de\":\"k::a\""));
        assert!(s.contains("\"para\":\"k::b\""));
        assert!(s.contains("\"peso\":3")); // prompt 0071: peso aditivo
        assert!(s.contains("\"ciclos\":[[\"k::a\",\"k::b\"]]"));
    }

    #[test]
    fn html_estrutura_embute_dado_cabecalho_e_limite() {
        let s = formatar_estrutura_html(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            "meu_pacote",
        );
        // O placeholder foi substituído (dado embutido, sem fetch).
        assert!(!s.contains("__DADOS_JSON__"), "placeholder não substituído");
        // A Matriz abre por padrão e os Cartões consomem o mesmo DADOS.
        assert!(s.contains(r#"id="vista-matriz" aria-pressed="true""#));
        assert!(s.contains(r#"id="vista-cartoes" aria-pressed="false""#));
        assert!(s.contains(r#"id="cartoes" class="cartoes" aria-label="Módulos e grupos" hidden"#));
        assert!(s.contains(r#"id="go-central""#));
        assert!(s.contains(r#"id="go-setas""#));
        assert!(s.contains(r#"id="go-voltar-arquivos" hidden"#));
        // O dado vai inline na const DADOS.
        assert!(s.contains("const DADOS ="));
        assert!(s.contains("\"pacote\":\"meu_pacote\""));
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"peso\":3"));
        assert!(s.contains("\"ordem\":["));
        // Prompt 0073: raio por módulo embutido (índices na ordem) + semântica.
        assert!(s.contains("\"raios\":["));
        assert!(s.contains("\"m\":") && s.contains("\"j\":"));
        assert!(s.contains(cat::DSM_RAIO_SEMANTICA));
        // Cabeçalho honesto: a declaração de limite (§3) vem do catálogo.
        assert!(s.contains("estrutural"));
        assert!(s.contains(&cat::DSM_LIMITE_HTML.replace('<', "\\u003c")));
        // Autocontido: sem fetch nem carga de recurso externo (o namespace SVG
        // `http://www.w3.org/2000/svg` é identificador, não rede — não conta).
        assert!(!s.contains("fetch("));
        assert!(!s.contains("src=\"http"));
        assert!(!s.contains("href=\"http"));
        // Escopo completo → sem dica de recorte.
        assert!(s.contains("\"escopo_dica\":\"\""));
        // Determinístico.
        let s2 = formatar_estrutura_html(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            "meu_pacote",
        );
        assert_eq!(s, s2);
    }

    #[test]
    fn html_scip_admite_fluxo_publicado_com_passos_completos() {
        let estrut = estrutura_amostra();
        let indice = FontesIndexadas::default();
        let grafo = Default::default();
        let a = serde_json::json!(["a.cpp", 5, 4, 5, 8]);
        let b = serde_json::json!(["b.cpp", 8, 4, 8, 8]);
        let uso = serde_json::json!(["a.cpp", 6, 2, 6, 6]);
        let fluxo = serde_json::json!({
            "nome": "Camadas", "raiz": a, "lado": 0, "passos": [[a,b,uso]],
        });
        let html = formatar_estrutura_html_scip_com_fluxos(
            &estrut,
            &indice,
            &grafo,
            &serde_json::json!([fluxo]).to_string(),
        )
        .unwrap();
        assert!(html.contains(r#""fluxos_publicados":[{"lado":0,"nome":"Camadas""#));
        assert!(
            formatar_estrutura_html_scip_com_fluxos(
                &estrut,
                &indice,
                &grafo,
                r#"[{"nome":"Inválido","raiz":["a.cpp",1,0,1,1],"lado":0,"passos":[[]]}]"#,
            )
            .is_err()
        );
    }

    #[test]
    fn html_scip_embute_prompt_vinculado_como_texto() {
        use lente_core::domain::fontes_indexadas::PromptFonte;

        let caminho = "00_nucleo/prompts/a.md";
        let texto = "# Contrato\n\nExemplo literal: `</script>`";
        let mut indice = FontesIndexadas::default();
        indice.prompts.insert(
            "src/a.rs".into(),
            PromptFonte {
                caminho: caminho.into(),
                texto: texto.into(),
            },
        );
        let html = formatar_estrutura_html_scip(&estrutura_amostra(), &indice);
        let dados = html
            .lines()
            .find_map(|linha| linha.strip_prefix("const DADOS = "))
            .unwrap();
        assert!(!dados.contains("</script>"));
        let dados: serde_json::Value = serde_json::from_str(dados.trim_end_matches(';')).unwrap();
        assert_eq!(dados["scip"]["prompts"]["src/a.rs"]["caminho"], caminho);
        assert_eq!(dados["scip"]["prompts"]["src/a.rs"]["texto"], texto);
    }

    #[test]
    fn html_scip_preserva_posicao_da_chamada_e_identidade_do_chamador() {
        use lente_core::domain::fontes_indexadas::{FuncaoFonte, LocalFonte};
        use serde_json::json;

        let posicao = |arquivo: &str, linha, coluna| LocalFonte {
            arquivo: arquivo.into(),
            linha,
            coluna,
            linha_fim: linha,
            coluna_fim: coluna + 4,
        };
        let mut indice = FontesIndexadas::default();
        indice.chamadores.insert(
            posicao("src/lib.rs", 3, 3),
            BTreeMap::from([(
                posicao("src/main.rs", 6, 8),
                Some(FuncaoFonte {
                    nome: "main".into(),
                    declaracao: posicao("src/main.rs", 5, 3),
                }),
            )]),
        );
        let html = formatar_estrutura_html_scip(&estrutura_amostra(), &indice);
        assert!(html.contains(r#"id="scip-grafo-foco" role="status""#));
        assert!(html.contains(r#"id="scip-grafo-limpar" type="button" hidden"#));
        assert!(
            html.contains(r#"id="scip-grafo-comparar" type="button" aria-pressed="false" hidden"#)
        );
        assert!(html.contains(r#"id="scip-grafo-legenda" role="group""#));
        assert!(
            html.contains(r#"id="scip-encontro" aria-labelledby="scip-encontro-titulo" hidden"#)
        );
        assert!(html.contains(r#"id="scip-encontro-anterior" type="button" disabled"#));
        assert!(html.contains(r#"id="scip-encontro-proximo" type="button" disabled"#));
        assert!(html.contains(r#"id="scip-encontro-posicao" role="status""#));
        assert!(html.contains(r#"id="scip-encontros" hidden"#));
        assert!(html.contains(r#"id="scip-encontros-filtro" type="search""#));
        assert!(html.contains(r#"id="scip-encontros-resultado" role="status" hidden"#));
        assert!(
            html.contains(r#"id="scip-encontros-itens" aria-labelledby="scip-encontros-titulo""#)
        );
        let dados = html
            .lines()
            .find_map(|linha| linha.strip_prefix("const DADOS = "))
            .unwrap();
        let dados: serde_json::Value = serde_json::from_str(dados.trim_end_matches(';')).unwrap();
        assert_eq!(
            dados["scip"]["chamadores"],
            json!([[
                ["src/lib.rs", 3, 3, 3, 7],
                [[
                    ["src/main.rs", 6, 8, 6, 12],
                    "main",
                    ["src/main.rs", 5, 3, 5, 7]
                ]]
            ]])
        );
    }

    #[test]
    fn html_cartoes_embute_evidencias_limitadas_sem_encerrar_script() {
        use lente_core::entities::grafo::{Aresta, Relation};
        use lente_estrutura::{DependenciaDetalhada, DestinoRelacao, RelacaoDetalhada};

        let detalhe = AgregacaoDetalhada {
            dependencias: vec![DependenciaDetalhada {
                id_de: 1,
                de: "k::a".into(),
                id_para: 2,
                para: "k::b".into(),
                peso: 9,
                evidencias: (0..9).collect(),
            }],
            relacoes: (0..9)
                .map(|indice_snapshot| RelacaoDetalhada {
                    indice_snapshot,
                    aresta: Aresta {
                        from: "k::a::</script>".into(),
                        id_from: 3,
                        to: "k::b::item".into(),
                        id_to: 4,
                        relation: Relation::Uses,
                        uses_kind: Some(UsesKind::Reference),
                    },
                    destino: DestinoRelacao::Agregada {
                        id_modulo_de: 1,
                        id_modulo_para: 2,
                    },
                })
                .collect(),
            itens_sem_contenedor: vec![],
        };
        let html = formatar_estrutura_html_com_evidencias(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            "k",
            &detalhe,
        );
        assert_eq!(html.matches("</script>").count(), 1);
        let linha_dados = html
            .lines()
            .find_map(|linha| linha.strip_prefix("const DADOS = "))
            .unwrap();
        let dados: serde_json::Value =
            serde_json::from_str(linha_dados.trim_end_matches(';')).unwrap();
        let par = &dados["evidencias"][0];
        assert_eq!(par["de"], "k::a");
        assert_eq!(par["para"], "k::b");
        assert_eq!(par["quantidade"], 9);
        assert_eq!(par["registros"].as_array().unwrap().len(), 8);
        assert_eq!(
            par["registros"][0],
            serde_json::json!(["k::a::</script>", "k::b::item", "referência"])
        );
    }

    #[test]
    fn indice_go_preserva_simbolo_posicoes_e_arquivos_sem_ligacoes() {
        let indice = serde_json::json!({
            "schema_version": 1,
            "files": {"src/a.go": "k::a", "src/b.go": "k::a", "isolado.go": "k::b"},
            "links": [{
                "from": "src/a.go", "to": "src/b.go",
                "references": [{
                    "symbol":"Objeto.Campo", "use_line": 22, "use_column": 10, "definition_line": 12
                }]
            }]
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&indice.to_string(), &estrutura).unwrap();
        assert_eq!(dados["files"]["isolado.go"], "k::b");
        assert_eq!(
            dados["links"][0],
            serde_json::json!(["src/a.go", "src/b.go", [["Objeto.Campo", 22, 10, 12]]])
        );

        let mut invalido = indice.clone();
        invalido["files"]["src/a.go"] = serde_json::json!("pacote-de-outro-grafo");
        assert!(referencias_go_para_html(&invalido.to_string(), &estrutura).is_err());
        invalido = indice.clone();
        invalido["links"][0]["references"][0]["use_line"] = serde_json::json!(0);
        assert!(referencias_go_para_html(&invalido.to_string(), &estrutura).is_err());
        invalido = indice;
        invalido["links"][0]["to"] = serde_json::json!("src/ausente.go");
        assert!(referencias_go_para_html(&invalido.to_string(), &estrutura).is_err());
    }

    #[test]
    fn html_go_embute_fonte_e_coluna_da_declaracao_com_texto_literal() {
        let detalhe = AgregacaoDetalhada {
            dependencias: vec![],
            relacoes: vec![],
            itens_sem_contenedor: vec![],
        };
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a", "b.go": "k::b"},
            "sources": {"a.go": "var a = b.Método", "b.go": "func Método() { /* </script> */ }"},
            "links": [{
                "from": "a.go", "to": "b.go",
                "references": [{
                    "symbol": "b.Método", "use_line": 1, "use_column": 11,
                    "definition_line": 1, "definition_column": 6
                }]
            }]
        });
        let estrutura = estrutura_amostra();
        let html = formatar_estrutura_html_com_referencias_go(
            &estrutura,
            Escopo::Completo,
            ModoUses::Todas,
            "k",
            &detalhe,
            &entrada.to_string(),
        )
        .unwrap();
        assert!(html.contains(r#"id="go-fonte" aria-labelledby="go-fonte-titulo""#));
        assert_eq!(html.matches("</script>").count(), 1);
        let dados = html
            .lines()
            .find_map(|linha| linha.strip_prefix("const DADOS = "))
            .unwrap();
        let dados: serde_json::Value = serde_json::from_str(dados.trim_end_matches(';')).unwrap();
        assert_eq!(
            dados["referencias_go"]["sources"]["b.go"],
            entrada["sources"]["b.go"]
        );
        assert_eq!(
            dados["referencias_go"]["links"][0][2][0],
            serde_json::json!(["b.Método", 1, 11, 1, 6])
        );

        entrada["links"][0]["references"][0]["definition_column"] = serde_json::json!(0);
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        entrada["links"][0]["references"][0]["definition_column"] = serde_json::json!(6);
        entrada["sources"]["ausente.go"] = serde_json::json!("package ausente");
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
    }

    #[test]
    fn referencias_locais_go_sao_indexadas_sem_criar_ligacoes_no_grafo() {
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a"},
            "links": [],
            "local_references": {
                "a.go": [{
                    "symbol": "f", "use_line": 4, "use_column": 7,
                    "definition_line": 3, "definition_column": 6
                }]
            }
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&entrada.to_string(), &estrutura).unwrap();
        assert_eq!(dados["links"], serde_json::json!([]));
        assert_eq!(
            dados["local_references"],
            serde_json::json!([["a.go", "a.go", [["f", 4, 7, 3, 6]]]])
        );
        entrada["files"] = serde_json::json!({});
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
    }

    #[test]
    fn indice_go_preserva_declaracoes_sem_usos() {
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a"},
            "links": [],
            "definitions": {
                "a.go": [{"symbol": "Objeto.Método", "line": 3, "column": 6}]
            }
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&entrada.to_string(), &estrutura).unwrap();
        assert_eq!(dados["links"], serde_json::json!([]));
        assert_eq!(
            dados["definitions"],
            serde_json::json!([["a.go", [["Objeto.Método", 3, 6]]]])
        );
        entrada["definitions"]["a.go"][0]["column"] = serde_json::json!(0);
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        entrada["definitions"] = serde_json::json!({"ausente.go": []});
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
    }

    #[test]
    fn arquivos_exclusivos_da_busca_podem_referenciar_producao() {
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a"},
            "search_files": ["a_test.go"],
            "sources": {"a_test.go": "package a"},
            "definitions": {"a_test.go": [{"symbol": "TestF", "line": 3, "column": 6}]},
            "links": [],
            "search_links": [{
                "from": "a_test.go", "to": "a.go",
                "references": [{
                    "symbol": "f", "use_line": 4, "use_column": 7,
                    "definition_line": 3, "definition_column": 6
                }]
            }]
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&entrada.to_string(), &estrutura).unwrap();
        assert_eq!(dados["files"], serde_json::json!({"a.go": "k::a"}));
        assert_eq!(dados["links"], serde_json::json!([]));
        assert_eq!(
            dados["search_links"],
            serde_json::json!([["a_test.go", "a.go", [["f", 4, 7, 3, 6]]]])
        );
        assert_eq!(
            dados["definitions"],
            serde_json::json!([["a_test.go", [["TestF", 3, 6]]]])
        );
        entrada["links"] = entrada["search_links"].clone();
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        entrada["links"] = serde_json::json!([]);
        entrada["search_files"] = serde_json::json!(["../a_test.go"]);
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
    }

    #[test]
    fn copias_de_mutantes_se_restringem_a_arquivos_exclusivos_da_busca() {
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a"},
            "search_files": ["testdata/mutants/M01/a_test.go", "teste_normal.go"],
            "mutant_files": ["testdata/mutants/M01/a_test.go"],
            "links": []
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&entrada.to_string(), &estrutura).unwrap();
        assert_eq!(
            dados["mutant_files"],
            serde_json::json!(["testdata/mutants/M01/a_test.go"])
        );
        assert_eq!(dados["files"], serde_json::json!({"a.go": "k::a"}));
        for invalido in [
            serde_json::json!(["a.go"]),
            serde_json::json!(["ausente.go"]),
        ] {
            entrada["mutant_files"] = invalido;
            assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        }
    }

    #[test]
    fn comparacao_go_valida_o_par_e_o_alinhamento_das_fontes() {
        let mut entrada = serde_json::json!({
            "schema_version": 1,
            "files": {"a.go": "k::a"},
            "search_files": ["reference/subject.go", "mutants/M01/subject.go"],
            "mutant_files": ["mutants/M01/subject.go"],
            "sources": {
                "reference/subject.go": "comum\nretirada\ncomum de novo\n",
                "mutants/M01/subject.go": "comum\nnova\ncomum de novo\n"
            },
            "mutant_comparisons": [{
                "label": "M01",
                "files": [{
                    "reference": "reference/subject.go", "mutant": "mutants/M01/subject.go",
                    "changes": [[1, 1, 1, 1]]
                }]
            }],
            "links": []
        });
        let estrutura = estrutura_amostra();
        let dados = referencias_go_para_html(&entrada.to_string(), &estrutura).unwrap();
        assert_eq!(dados["mutant_comparisons"], entrada["mutant_comparisons"]);

        entrada["mutant_comparisons"][0]["files"][0]["changes"] = serde_json::json!([]);
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        entrada["mutant_comparisons"][0]["files"][0]["changes"] =
            serde_json::json!([[1, 999, 1, 1]]);
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        entrada["mutant_comparisons"][0]["files"][0]["reference"] =
            serde_json::json!("mutants/M01/subject.go");
        assert!(referencias_go_para_html(&entrada.to_string(), &estrutura).is_err());
        assert!(blocos_go_validos(
            "a\nb\n",
            "a\n",
            &serde_json::json!([[1, 1, 1, 0]])
        ));
        assert!(blocos_go_validos(
            "",
            "a\n",
            &serde_json::json!([[0, 0, 0, 1]])
        ));
    }

    #[test]
    fn resultados_de_subteste_exigem_posicoes_presentes_no_snapshot() {
        let fontes =
            serde_json::json!({"suite.go": "t.Run(\"falha\", func() {\n    t.Fatal()\n})"});
        let fontes = fontes.as_object().unwrap();
        let mut testes = serde_json::json!({
            "suite": "TestSuite", "run_label": "Execução local",
            "reference_passed": 1, "mutant_passed": 0, "mutant_failed": 1,
            "failures": [{"name": "falha", "file": "suite.go", "line": 1, "column": 8, "failure_line": 2}]
        });
        assert!(validar_testes_comparacao_go(&testes, fontes).is_ok());
        testes["failures"][0]["column"] = serde_json::json!(7);
        assert!(validar_testes_comparacao_go(&testes, fontes).is_err());
        testes["failures"][0]["column"] = serde_json::json!(8);
        testes["failures"][0]["failure_line"] = serde_json::json!(9);
        assert!(validar_testes_comparacao_go(&testes, fontes).is_err());
    }

    #[test]
    fn matriz_de_testes_go_confere_a_celula_com_a_falha_da_variante() {
        let fontes = serde_json::Map::new();
        let comparacoes = serde_json::json!([{
            "label": "M01", "tests": {
                "suite": "TestSuite", "run_label": "Execução local",
                "reference_passed": 2, "mutant_passed": 1, "mutant_failed": 1,
                "failures": [{"name": "primeiro"}]
            }
        }]);
        let mut matriz = serde_json::json!({
            "suite": "TestSuite", "run_label": "Execução local",
            "cases": ["primeiro", "segundo"],
            "reference": ["pass", "pass"],
            "variants": {"M01": ["fail", "pass"]}
        });
        assert!(validar_matriz_testes_go(&matriz, &comparacoes, &fontes).is_ok());
        matriz["variants"]["M01"] = serde_json::json!(["pass", "fail"]);
        assert!(validar_matriz_testes_go(&matriz, &comparacoes, &fontes).is_err());
        matriz["variants"]["M01"] = serde_json::json!(["fail"]);
        assert!(validar_matriz_testes_go(&matriz, &comparacoes, &fontes).is_err());
        matriz["variants"]["M01"] = serde_json::json!(["fail", "pass"]);
        matriz["run_label"] = serde_json::json!("Outra execução");
        assert!(validar_matriz_testes_go(&matriz, &comparacoes, &fontes).is_err());
    }

    #[test]
    fn locais_da_matriz_identificam_o_nome_literal_no_snapshot() {
        let fontes = serde_json::json!({
            "suite.go": "t.Run(\"primeiro-outro\", verificar)\n/* ç */ t.Run(`primeiro`, verificar)\n"
        });
        let fontes = fontes.as_object().unwrap();
        let casos = ["primeiro", "segundo"];
        let mut locais = serde_json::json!({
            "primeiro": {"file": "suite.go", "line": 2, "column": 17}
        });
        assert!(validar_locais_subtestes_go(&locais, &casos, fontes).is_ok());
        locais["primeiro"]["column"] = serde_json::json!(16);
        assert!(validar_locais_subtestes_go(&locais, &casos, fontes).is_err());
        locais["primeiro"] = serde_json::json!({"file": "suite.go", "line": 1, "column": 8});
        assert!(validar_locais_subtestes_go(&locais, &casos, fontes).is_err());
        locais["primeiro"] = serde_json::json!({"file": "ausente.go", "line": 2, "column": 17});
        assert!(validar_locais_subtestes_go(&locais, &casos, fontes).is_err());
        let fora_da_matriz = serde_json::json!({
            "primeiro": {"file": "suite.go", "line": 2, "column": 17}
        });
        assert!(validar_locais_subtestes_go(&fora_da_matriz, &["segundo"], fontes).is_err());
    }

    fn comparacao_amostra() -> Comparacao {
        Comparacao {
            nome_antes: "velho".to_string(),
            nome_depois: "novo".to_string(),
            pareados: vec![(Path::from("velho::a"), Path::from("novo::a"))],
            sem_par_antes: vec![Path::from("velho::sumiu")],
            sem_par_depois: vec![Path::from("novo::apareceu")],
            arestas_comuns: vec![ArestaComparada {
                de: Path::from("velho::a"),
                para: Path::from("velho::b"),
                peso_antes: 2,
                peso_depois: 7,
            }],
            arestas_so_antes: vec![DependenciaModulo {
                de: Path::from("velho::a"),
                para: Path::from("velho::c"),
                peso: 1,
            }],
            arestas_so_depois: vec![],
            ciclos_antes: lente_comparacao::ResumoCiclos {
                quantidade: 1,
                maior: 5,
            },
            ciclos_depois: lente_comparacao::ResumoCiclos {
                quantidade: 0,
                maior: 0,
            },
            chave: "path_completo".to_string(),
            proveniencia: lente_comparacao::Proveniencia {
                modo_antes: "workspace".to_string(),
                modo_depois: "crate".to_string(),
                crates_antes: 3,
                crates_depois: 1,
                fantasmas_antes: vec![Path::from("ext::Coisa")],
                fantasmas_depois: vec![],
                falhas_antes: vec!["crate_ruim — colisão irresolúvel".to_string()],
                falhas_depois: vec![],
                third_party_antes: 42,
                third_party_depois: 0,
            },
            itens: lente_comparacao::ComparacaoItens {
                pareados: vec![lente_comparacao::ItemPareado {
                    kind: "struct".to_string(),
                    trait_: String::new(),
                    nome_qualificado: "Counter".to_string(),
                    path_antes: "velho::a::Counter".to_string(),
                    path_depois: "novo::b::Counter".to_string(),
                }],
                ambiguos: vec![lente_comparacao::ItemAmbiguo {
                    kind: "fn".to_string(),
                    trait_: String::new(),
                    nome_qualificado: "new".to_string(),
                    candidatos_antes: vec![
                        "velho::a::X::new".to_string(),
                        "velho::a::Y::new".to_string(),
                    ],
                    candidatos_depois: vec!["novo::b::X::new".to_string()],
                }],
                sem_par_antes: vec![lente_comparacao::ItemSemPar {
                    kind: "fn".to_string(),
                    trait_: String::new(),
                    nome_qualificado: "sumiu".to_string(),
                    path: "velho::a::sumiu".to_string(),
                }],
                sem_par_depois: vec![],
                mapa: lente_comparacao::ResultadoMapa::default(),
            },
        }
    }

    #[test]
    fn comparacao_texto_traz_cabecalho_limite_e_secoes() {
        let s = formatar_comparacao(
            &comparacao_amostra(),
            Escopo::SeuCodigo,
            ModoUses::Todas,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("velho") && s.contains("novo"));
        assert!(s.contains("escopo=seu-codigo"));
        assert!(s.contains(cat::COMPARAR_LIMITE)); // limite do pareamento na saída
        assert!(s.contains("velho::sumiu"));
        assert!(s.contains("novo::apareceu"));
        assert!(s.contains("velho::c")); // aresta que sumiu
        assert!(s.contains("2 → 7")); // delta de peso
        assert!(s.contains("antes:  1 ciclo") && s.contains("depois: 0 ciclo"));
        // Prompt 0075: a proveniência declarada no texto.
        assert!(s.contains("antes=workspace") && s.contains("depois=crate"));
        assert!(s.contains("chave=path_completo"));
        assert!(s.contains("3 crate(s)") && s.contains("1 fantasma(s)"));
        // Prompt 0075: crates não extraídos declarados no texto.
        assert!(s.contains("Crates não extraídos"));
        assert!(s.contains("crate_ruim — colisão irresolúvel"));
        // Prompt 0076: third-party removido declarado no texto.
        assert!(s.contains("Third-party fora do censo") && s.contains("antes 42 nós"));
        // Prompt 0078: nível de item agregado no texto.
        assert!(s.contains("Itens (chave de identidade K4"));
        assert!(s.contains("pareados: 1") && s.contains("ambíguos: 1 chaves"));
        assert!(s.contains("sem-par por crate") && s.contains("velho: antes 1"));
    }

    #[test]
    fn comparacao_json_tem_o_contrato() {
        let s = formatar_comparacao(
            &comparacao_amostra(),
            Escopo::SeuCodigo,
            ModoUses::Todas,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"nome_antes\":\"velho\""));
        assert!(s.contains("\"pareados\":[{\"de\":\"velho::a\",\"para\":\"novo::a\"}]"));
        assert!(s.contains("\"sem_par_antes\":[\"velho::sumiu\"]"));
        assert!(s.contains("\"peso_antes\":2") && s.contains("\"peso_depois\":7"));
        assert!(s.contains(
            "\"arestas_so_antes\":[{\"de\":\"velho::a\",\"para\":\"velho::c\",\"peso\":1}]"
        ));
        // serde_json::Map ordena as chaves (BTreeMap): maior < quantidade.
        assert!(s.contains("\"ciclos_antes\":{\"maior\":5,\"quantidade\":1}"));
        assert!(s.contains("\"limite_pareamento\":"));
        assert!(s.contains("\"escopo\":\"seu-codigo\""));
        // Prompt 0075: proveniência aditiva no JSON.
        assert!(s.contains("\"chave\":\"path_completo\""));
        assert!(s.contains("\"modo_antes\":\"workspace\""));
        assert!(s.contains("\"crates_antes\":3"));
        assert!(s.contains("\"fantasmas_antes\":[\"ext::Coisa\"]"));
        // Prompt 0075: falhas por lado, aditivo.
        assert!(s.contains("\"falhas_antes\":[\"crate_ruim — colisão irresolúvel\"]"));
        assert!(s.contains("\"falhas_depois\":[]"));
        // Prompt 0076: third-party aditivo no JSON.
        assert!(s.contains("\"third_party_antes\":42"));
        assert!(s.contains("\"third_party_depois\":0"));
        // Prompt 0078: nível de item no JSON (pareado com os dois paths).
        assert!(s.contains("\"itens\":{"));
        assert!(s.contains("\"nome\":\"Counter\""));
        assert!(
            s.contains("\"de\":\"velho::a::Counter\"")
                && s.contains("\"para\":\"novo::b::Counter\"")
        );
        assert!(s.contains("\"candidatos_antes\":["));
    }

    #[test]
    fn html_seu_codigo_declara_dica_de_recorte() {
        // Prompt 0072: no escopo seu-codigo (default da vista), o cabeçalho
        // declara o recorte e como obter o completo.
        let s = formatar_estrutura_html(
            &estrutura_amostra(),
            Escopo::SeuCodigo,
            ModoUses::Todas,
            "k",
        );
        assert!(s.contains("\"escopo\":\"seu-codigo\""));
        assert!(s.contains(cat::DSM_ESCOPO_DICA));
        assert!(s.contains("--completo"));
    }

    #[test]
    fn texto_estrutura_destaca_ciclos_e_lista_dependencias() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::SeuCodigo,
            ModoUses::Todas,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Estrutura de módulos"));
        assert!(s.contains("escopo: seu-codigo"));
        assert!(s.contains("uses: todas"));
        assert!(s.contains("3 módulos"));
        assert!(s.contains("1 ciclos"));
        assert!(s.contains("Ciclos:"));
        assert!(s.contains("k::a, k::b"));
        assert!(s.contains("Dependências módulo → módulo:"));
        assert!(s.contains("k::a → k::b"));
        assert!(s.contains("k::b → k::a"));
    }

    #[test]
    fn texto_estrutura_sem_ciclos_diz_nenhum_ciclo() {
        let mut e = estrutura_amostra();
        e.ciclos.clear();
        let s = formatar_estrutura(
            &e,
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("nenhum ciclo"));
    }

    #[test]
    fn json_estrutura_sem_ciclos_lista_vazia() {
        let mut e = estrutura_amostra();
        e.ciclos.clear();
        let s = formatar_estrutura(
            &e,
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"ciclos\":[]"));
    }

    // Prompt 0034 — declaração do modo de uses na saída ---------------------

    #[test]
    fn json_estrutura_so_referencia_declara_modo_uses() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::SoReferencia,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        assert!(s.contains("\"modo_uses\":\"so-referencia\""));
    }

    #[test]
    fn texto_estrutura_so_referencia_aparece_no_cabecalho() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::SoReferencia,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("uses: so-referencia"));
    }

    // ---- Prompt 0035: ordem + blocos na saída -------------------------------

    #[test]
    fn json_estrutura_inclui_ordem_e_blocos() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: false,
                verbose: false,
            },
        );
        // Ordem: ["k", "k::a", "k::b"] — sequência exata da DSM.
        assert!(s.contains("\"ordem\":[\"k\",\"k::a\",\"k::b\"]"));
        // Blocos: um único, com {k::a, k::b}.
        assert!(s.contains("\"blocos\":[[\"k::a\",\"k::b\"]]"));
    }

    #[test]
    fn texto_estrutura_lista_ordem_com_marcador_de_bloco() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        // Título da seção.
        assert!(s.contains("Ordem da DSM"));
        // `k` (livre) e `k::a`/`k::b` (com marcador `◆`).
        assert!(s.contains("   k\n") || s.contains("    k\n"));
        assert!(s.contains("◆ k::a"));
        assert!(s.contains("◆ k::b"));
    }

    #[test]
    fn texto_estrutura_sem_blocos_lista_ordem_sem_marcadores() {
        let mut e = estrutura_amostra();
        e.blocos.clear();
        let s = formatar_estrutura(
            &e,
            Escopo::Completo,
            ModoUses::Todas,
            &Modo {
                text: true,
                verbose: false,
            },
        );
        assert!(s.contains("Ordem da DSM"));
        assert!(!s.contains("◆"));
    }

    // ---- Modo diff (prompt 0047) --------------------------------------------

    #[test]
    fn diff_json_tem_o_esquema_e_e_desserializavel() {
        use lente_core::domain::mapeamento::NoTocado;
        use lente_core::domain::resultado_diff::{RaioCombinado, ResultadoDiff, TocadoComRaio};
        use lente_core::domain::uniao::Fantasma;
        use lente_core::entities::grafo::Path;
        use std::collections::HashMap;

        let mut montante = HashMap::new();
        montante.insert(Path::from("t::B"), 1usize);
        let raio = Raio {
            alvo: Path::from("t::A"),
            classificacao: Classificacao::Base,
            uses_entrada: 1,
            uses_saida: 0,
            montante,
            jusante: HashMap::new(),
            owns_pai: None,
            owns_filhos: Vec::new(),
        };
        let resultado = ResultadoDiff {
            falhas: Vec::new(),
            tocados: vec![TocadoComRaio {
                tocado: NoTocado {
                    id: 1,
                    path: Path::from("t::A"),
                },
                raio,
            }],
            combinado: RaioCombinado {
                montante: vec![(Path::from("t::B"), 1)],
                jusante: Vec::new(),
            },
            ligados: vec![std::path::PathBuf::from("/r/a/src/lig.rs")],
            soltos: vec![std::path::PathBuf::from("/r/a/src/solto.rs")],
            nao_fonte: vec![std::path::PathBuf::from("/r/README.md")],
            fantasmas: vec![Fantasma {
                path: Path::from("t::Some"),
                referenciado_por: vec!["x".to_string()],
            }],
        };

        let json = formatar_diff(&resultado);
        let v: serde_json::Value =
            serde_json::from_str(&json).expect("JSON do diff deve desserializar");

        // tocados com raio
        assert_eq!(v["tocados"][0]["path"], "t::A");
        assert_eq!(v["tocados"][0]["id"], 1);
        assert!(v["tocados"][0]["classificacao"].is_string());
        assert_eq!(v["tocados"][0]["montante"], 1);
        assert_eq!(v["tocados"][0]["jusante"], 0);
        // combinado (path + profundidade)
        assert_eq!(v["combinado"]["montante"][0]["path"], "t::B");
        assert_eq!(v["combinado"]["montante"][0]["profundidade"], 1);
        assert!(v["combinado"]["jusante"].as_array().unwrap().is_empty());
        // censo do untracked
        assert_eq!(v["ligados"][0], "/r/a/src/lig.rs");
        assert_eq!(v["soltos"][0], "/r/a/src/solto.rs");
        assert_eq!(v["nao_fonte"][0], "/r/README.md");
        // fantasmas
        assert_eq!(v["fantasmas"][0]["path"], "t::Some");
        assert_eq!(v["fantasmas"][0]["referenciado_por"][0], "x");
    }

    // ---- Vistas de texto do --diff (prompt 0048) ----------------------------

    fn no_tocado(id: usize, path: &str) -> lente_core::domain::mapeamento::NoTocado {
        lente_core::domain::mapeamento::NoTocado {
            id,
            path: PathGrafo::from(path),
        }
    }

    fn raio_v(c: Classificacao, mont: &[(&str, usize)], jus: &[(&str, usize)]) -> Raio {
        Raio {
            alvo: PathGrafo::from("x"),
            classificacao: c,
            uses_entrada: mont.len(),
            uses_saida: jus.len(),
            montante: mont
                .iter()
                .map(|(p, d)| (PathGrafo::from(*p), *d))
                .collect(),
            jusante: jus.iter().map(|(p, d)| (PathGrafo::from(*p), *d)).collect(),
            owns_pai: None,
            owns_filhos: Vec::new(),
        }
    }

    /// `ResultadoDiff` forjado: 2 tocados em `lente_core` (No [Base], jusante 0;
    /// entities::grafo [Intermediario], jusante 1), combinado cruzando
    /// lente_infra/lente_wiring, 1 solto, 0 fantasmas.
    fn resultado_amostra() -> ResultadoDiff {
        use lente_core::domain::resultado_diff::{RaioCombinado, TocadoComRaio};
        ResultadoDiff {
            falhas: Vec::new(),
            tocados: vec![
                TocadoComRaio {
                    tocado: no_tocado(1, "lente_core::No"),
                    raio: raio_v(
                        Classificacao::Base,
                        &[("lente_infra::a", 1), ("lente_infra::b", 1)],
                        &[],
                    ),
                },
                TocadoComRaio {
                    tocado: no_tocado(2, "lente_core::entities::grafo"),
                    raio: raio_v(
                        Classificacao::Intermediario,
                        &[("lente_infra::a", 1), ("lente_wiring::c", 1)],
                        &[("lente_core::d", 1)],
                    ),
                },
            ],
            combinado: RaioCombinado {
                montante: vec![
                    (PathGrafo::from("lente_infra::a"), 1),
                    (PathGrafo::from("lente_infra::b"), 1),
                    (PathGrafo::from("lente_wiring::c"), 1),
                ],
                jusante: vec![(PathGrafo::from("lente_core::d"), 1)],
            },
            ligados: vec![std::path::PathBuf::from("/r/01_core/src/lig.rs")],
            soltos: vec![std::path::PathBuf::from("/r/02_shell/cli/src/cli_novo.rs")],
            nao_fonte: vec![
                std::path::PathBuf::from("/r/README.md"),
                std::path::PathBuf::from("/r/X.md"),
            ],
            fantasmas: Vec::new(),
        }
    }

    #[test]
    fn diff_sem_impacto_distingue_completo_de_parcial_em_todas_as_saidas() {
        let mut r = resultado_amostra();
        r.tocados.clear();
        r.combinado.montante.clear();
        r.combinado.jusante.clear();
        let completo: serde_json::Value = serde_json::from_str(&formatar_diff(&r)).unwrap();
        assert_eq!(completo["completude"], "completa");
        assert_eq!(completo["falhas"], serde_json::json!([]));
        for vista in [Vista::Resumo, Vista::Item, Vista::Camadas] {
            assert!(formatar_diff_vista(&r, vista).starts_with("Análise completa:"));
        }
        r.falhas
            .push(lente_core::domain::resultado_diff::FalhaCrate {
                crate_name: "ausente".into(),
                motivo: "extração falhou: \"erro\"\ncausa".into(),
            });
        let parcial: serde_json::Value = serde_json::from_str(&formatar_diff(&r)).unwrap();
        assert_eq!(parcial["completude"], "parcial");
        assert_eq!(parcial["falhas"][0]["crate_name"], "ausente");
        assert_eq!(parcial["falhas"][0]["motivo"], r.falhas[0].motivo);
        assert_eq!(parcial["tocados"], completo["tocados"]);
        assert_eq!(parcial["ligados"], completo["ligados"]);
        for vista in [Vista::Resumo, Vista::Item, Vista::Camadas] {
            let texto = formatar_diff_vista(&r, vista);
            assert!(texto.starts_with("Análise parcial:"));
            assert!(texto.contains("não garante ausência de impacto"));
            assert!(texto.contains("ausente"));
            assert!(texto.contains(&r.falhas[0].motivo));
        }
    }

    #[test]
    fn vista_resumo_traz_contagens_por_crate_censo_e_solto() {
        let s = formatar_diff_resumo(&resultado_amostra());
        assert!(s.contains("diff: 2 tocados em 1 crate"));
        assert!(s.contains("pode quebrar (montante): 3"));
        assert!(s.contains("lente_infra 2"));
        assert!(s.contains("lente_wiring 1"));
        assert!(s.contains("depende de (jusante): 1"));
        assert!(s.contains("untracked: 1 compilados"));
        assert!(s.contains("sem mod"));
        assert!(s.contains("cli_novo.rs"));
        assert!(!s.contains("fantasmas"), "0 fantasmas → linha ausente");
    }

    #[test]
    fn vista_item_um_bloco_por_tocado_com_classificacao_e_contagens() {
        let s = formatar_diff_item(&resultado_amostra());
        assert!(s.contains("2 tocados:"));
        assert!(s.contains("lente_core::No"));
        assert!(s.contains(classificacao_texto(Classificacao::Base)));
        assert!(s.contains("lente_core::entities::grafo"));
        // No: montante 2, jusante 0 → "—".
        assert!(s.contains("pode quebrar: 2"));
        assert!(s.contains("depende de: —"));
        assert!(s.contains("cli_novo.rs"));
    }

    #[test]
    fn vista_camadas_agrupa_por_crate_e_mostra_cross_crate() {
        let s = formatar_diff_camadas(&resultado_amostra());
        assert!(s.contains("tocados por crate:"));
        assert!(s.contains("lente_core"));
        // nomes curtos (sem o prefixo do crate).
        assert!(s.contains("No ["));
        assert!(s.contains("entities::grafo ["));
        assert!(s.contains("pode quebrar, por crate:"));
        assert!(s.contains("lente_infra 2"));
    }

    #[test]
    fn vista_resumo_enfase_adaptativa_arquivo_novo_lidera_jusante() {
        use lente_core::domain::resultado_diff::RaioCombinado;
        let mut r = resultado_amostra();
        // Diff só-arquivo-novo: montante combinado vazio, ligados presentes.
        r.combinado = RaioCombinado {
            montante: Vec::new(),
            jusante: vec![(PathGrafo::from("lente_core::d"), 1)],
        };
        let s = formatar_diff_resumo(&r);
        let pos_jus = s.find("depende de (jusante)").unwrap();
        let pos_mont = s.find("pode quebrar (montante)").unwrap();
        assert!(
            pos_jus < pos_mont,
            "arquivo novo deve liderar com o jusante"
        );
    }

    #[test]
    fn fantasma_aparece_so_se_maior_que_zero() {
        use lente_core::domain::uniao::Fantasma;
        let mut r = resultado_amostra();
        assert!(!formatar_diff_resumo(&r).contains("fantasmas"));
        r.fantasmas = vec![Fantasma {
            path: PathGrafo::from("t::Some"),
            referenciado_por: vec!["a".to_string()],
        }];
        assert!(formatar_diff_resumo(&r).contains("fantasmas: 1"));
        assert!(formatar_diff_item(&r).contains("fantasmas: 1"));
        assert!(formatar_diff_camadas(&r).contains("fantasmas: 1"));
    }

    #[test]
    fn solto_listado_em_todas_as_vistas() {
        let r = resultado_amostra();
        for s in [
            formatar_diff_resumo(&r),
            formatar_diff_item(&r),
            formatar_diff_camadas(&r),
        ] {
            assert!(s.contains("cli_novo.rs"), "solto deve aparecer em: {}", s);
        }
    }

    #[test]
    fn vistas_sao_deterministicas() {
        let r = resultado_amostra();
        assert_eq!(formatar_diff_resumo(&r), formatar_diff_resumo(&r));
        assert_eq!(formatar_diff_item(&r), formatar_diff_item(&r));
        assert_eq!(formatar_diff_camadas(&r), formatar_diff_camadas(&r));
    }

    #[test]
    fn roteamento_vista_chama_o_renderizador_certo() {
        let r = resultado_amostra();
        assert_eq!(
            formatar_diff_vista(&r, Vista::Resumo),
            format!("{}{}", cat::DIFF_ANALISE_COMPLETA, formatar_diff_resumo(&r))
        );
        assert_eq!(
            formatar_diff_vista(&r, Vista::Item),
            format!("{}{}", cat::DIFF_ANALISE_COMPLETA, formatar_diff_item(&r))
        );
        assert_eq!(
            formatar_diff_vista(&r, Vista::Camadas),
            format!(
                "{}{}",
                cat::DIFF_ANALISE_COMPLETA,
                formatar_diff_camadas(&r)
            )
        );
    }
}
