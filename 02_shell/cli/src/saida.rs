//! Formatação do `Raio` para stdout. Quatro modos (matriz `--text` ×
//! `--verbose`); todos os literais visíveis ao usuário vêm do
//! `lente_catalogo` (ADR-0002).
//!
//! Inclui também a formatação do **modo ranking** (prompt 0027):
//! `formatar_ranking(&[ItemRanking], Escopo, &Modo)`.
//!
//! Pós-prompt 0030: a saída (JSON e texto) **declara o escopo** em ambos
//! os modos — campo `escopo` no JSON e linha/cabeçalho no texto.

use lente_catalogo as cat;
use lente_core::domain::raio::{Classificacao, Raio};
use lente_core::entities::grafo::Path as PathGrafo;
use lente_wiring::{Escopo, EstruturaModulos, ItemRanking, ModoUses, ResultadoDiff};

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
        s.push_str(&format!(
            "{}:\t{}\n",
            cat::ROTULO_ALVO_RESOLVIDO,
            raio.alvo
        ));
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
    root.insert(
        cat::JSON_RANKING.to_string(),
        serde_json::Value::Array(arr),
    );
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
    serde_json::Value::Object(root).to_string()
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
    root.insert(cat::JSON_TOCADOS.to_string(), serde_json::Value::Array(tocados));

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
    root.insert(cat::JSON_COMBINADO.to_string(), serde_json::Value::Object(comb));

    // censo do untracked.
    root.insert(cat::JSON_LIGADOS.to_string(), lista_de_paths(&resultado.ligados));
    root.insert(cat::JSON_SOLTOS.to_string(), lista_de_paths(&resultado.soltos));
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: false, verbose: true },
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
            &Modo { text: true, verbose: false },
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
            &Modo { text: true, verbose: false },
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
            &Modo { text: true, verbose: true },
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: true, verbose: false },
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
        let _ = formatar_ranking(&v, Escopo::Completo, &Modo { text: true, verbose: false });
        let _ = formatar_ranking(&v, Escopo::Completo, &Modo { text: false, verbose: false });
    }

    // ---- prompt 0030: saída do raio declara o escopo ------------------------

    #[test]
    fn json_do_raio_inclui_campo_escopo_completo() {
        let r = raio_simples("foo::bar");
        let s = formatar(
            &r,
            &AlvoPedido::Path("foo::bar".to_string()),
            Escopo::Completo,
            &Modo { text: false, verbose: false },
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: true, verbose: false },
        );
        assert!(s.contains("Escopo:\tseu-codigo"));
    }

    // ---- Modo estrutura (prompt 0031) ----------------------------------------

    use lente_wiring::{Ciclo, DependenciaModulo};

    fn estrutura_amostra() -> EstruturaModulos {
        EstruturaModulos {
            modulos: vec![
                Path::from("k"),
                Path::from("k::a"),
                Path::from("k::b"),
            ],
            dependencias: vec![
                DependenciaModulo {
                    de: Path::from("k::a"),
                    para: Path::from("k::b"),
                },
                DependenciaModulo {
                    de: Path::from("k::b"),
                    para: Path::from("k::a"),
                },
            ],
            ciclos: vec![Ciclo {
                modulos: vec![Path::from("k::a"), Path::from("k::b")],
            }],
            // Prompt 0035: amostra de ordem + bloco para os testes de saída.
            // Ordem da DSM: k (crate, sem deps) → {k::a, k::b} (bloco).
            ordem: vec![
                Path::from("k"),
                Path::from("k::a"),
                Path::from("k::b"),
            ],
            blocos: vec![vec![Path::from("k::a"), Path::from("k::b")]],
        }
    }

    #[test]
    fn json_estrutura_tem_escopo_modulos_dependencias_ciclos() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::Todas,
            &Modo { text: false, verbose: false },
        );
        assert!(s.contains("\"escopo\":\"completo\""));
        assert!(s.contains("\"modo_uses\":\"todas\""));
        assert!(s.contains("\"modulos\":[\"k\",\"k::a\",\"k::b\"]"));
        assert!(s.contains("\"de\":\"k::a\""));
        assert!(s.contains("\"para\":\"k::b\""));
        assert!(s.contains("\"ciclos\":[[\"k::a\",\"k::b\"]]"));
    }

    #[test]
    fn texto_estrutura_destaca_ciclos_e_lista_dependencias() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::SeuCodigo,
            ModoUses::Todas,
            &Modo { text: true, verbose: false },
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
            &Modo { text: true, verbose: false },
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: false, verbose: false },
        );
        assert!(s.contains("\"modo_uses\":\"so-referencia\""));
    }

    #[test]
    fn texto_estrutura_so_referencia_aparece_no_cabecalho() {
        let s = formatar_estrutura(
            &estrutura_amostra(),
            Escopo::Completo,
            ModoUses::SoReferencia,
            &Modo { text: true, verbose: false },
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
            &Modo { text: false, verbose: false },
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
            &Modo { text: true, verbose: false },
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
            &Modo { text: true, verbose: false },
        );
        assert!(s.contains("Ordem da DSM"));
        assert!(!s.contains("◆"));
    }

    // ---- Modo diff (prompt 0047) --------------------------------------------

    #[test]
    fn diff_json_tem_o_esquema_e_e_desserializavel() {
        use lente_core::domain::mapeamento::NoTocado;
        use lente_core::entities::grafo::Path;
        use lente_wiring::{Fantasma, RaioCombinado, ResultadoDiff, TocadoComRaio};
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
}
