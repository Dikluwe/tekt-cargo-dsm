//! Formatação do `Raio` para stdout. Quatro modos (matriz `--text` ×
//! `--verbose`); todos os literais visíveis ao usuário vêm do
//! `lente_catalogo` (ADR-0002).

use lente_catalogo as cat;
use lente_core::domain::raio::{Classificacao, Raio};

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

pub fn formatar(raio: &Raio, alvo_pedido: &AlvoPedido, modo: &Modo) -> String {
    if modo.text {
        formatar_texto(raio, alvo_pedido, modo.verbose)
    } else {
        formatar_json(raio, alvo_pedido, modo.verbose)
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

fn formatar_json(raio: &Raio, alvo_pedido: &AlvoPedido, verbose: bool) -> String {
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

fn formatar_texto(raio: &Raio, alvo_pedido: &AlvoPedido, verbose: bool) -> String {
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
            &Modo { text: true, verbose: true },
        );
        assert!(s.contains("Impactados:"));
        assert!(s.contains("  a\n"));
        assert!(s.contains("  b\n"));
    }
}
