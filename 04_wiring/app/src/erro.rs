//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/app-erro.md
//! @prompt-hash 8f884580
//! @layer L4
//! @updated 2026-06-07
//!
//! Tradução de `lente_wiring::ErroLente` em mensagens de catálogo.
//!
//! O `Display` técnico do `ErroLente` e dos erros internos é **embutido
//! como `{detalhe}`** dentro das molduras do catálogo: o catálogo dá a
//! apresentação ao usuário; o `Display` dá a informação técnica.

use lente_catalogo as cat;
use lente_wiring::ErroLente;

/// Contexto de quem chamou — usado para personalizar mensagens quando o
/// erro tem informação importante que o `ErroLente` por si só não carrega.
#[derive(Debug, Clone)]
pub struct ContextoErro {
    /// Texto que o usuário pediu (path direto ou `"id=N"`).
    pub alvo_informado: String,
}

/// Traduz `ErroLente` em mensagem pronta para stderr.
pub fn traduzir(erro: &ErroLente, ctx: &ContextoErro) -> String {
    match erro {
        ErroLente::Fork(e) => {
            cat::ERRO_FORK_AUSENTE.render(&[("detalhe", &format!("{}", e))])
        }
        ErroLente::Adaptador(e) => {
            cat::ERRO_JSON_INVALIDO.render(&[("detalhe", &format!("{}", e))])
        }
        ErroLente::Raio(_) => {
            // O `ErroRaio` interno é `AlvoInexistente(Path)`; usamos o alvo
            // que o usuário pediu (que é o que ele entende), não o path
            // interno (que pode ter sido renomeado no pipeline).
            cat::ERRO_ALVO_INEXISTENTE
                .render(&[("alvo", ctx.alvo_informado.as_str())])
        }
        ErroLente::IdInexistente(id) => {
            cat::ERRO_ID_INEXISTENTE.render(&[("id", &id.to_string())])
        }
        ErroLente::Resolucao(e) => {
            cat::ERRO_RESOLUCAO.render(&[("detalhe", &format!("{}", e))])
        }
        ErroLente::ForkSemUsesKind => {
            cat::ERRO_FORK_SEM_USES_KIND.render(&[("detalhe", &format!("{}", erro))])
        }
        ErroLente::Workspace(e) => {
            cat::ERRO_WORKSPACE.render(&[("detalhe", &format!("{}", e))])
        }
        ErroLente::Diff(e) => {
            cat::ERRO_DIFF.render(&[("detalhe", &format!("{}", e))])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lente_core::domain::raio::ErroRaio;
    use lente_core::entities::grafo::Path;

    fn ctx(alvo: &str) -> ContextoErro {
        ContextoErro {
            alvo_informado: alvo.to_string(),
        }
    }

    #[test]
    fn id_inexistente_traduz_com_o_id() {
        let m = traduzir(&ErroLente::IdInexistente(42), &ctx("id=42"));
        assert_eq!(m, "Nó com id 42 não existe no grafo");
    }

    #[test]
    fn raio_alvo_inexistente_usa_alvo_pedido_pelo_usuario() {
        // O Erro do raio interno traz o Path interno; a CLI usa o alvo
        // que o usuário entende (pode ser path ou "id=N").
        let erro = ErroLente::Raio(ErroRaio::AlvoInexistente(Path::from("interno")));
        let m = traduzir(&erro, &ctx("foo::bar"));
        assert_eq!(m, "Alvo 'foo::bar' não existe no grafo");
    }

    #[test]
    fn adaptador_traduz_com_detalhe_tecnico() {
        let erro = ErroLente::Adaptador(lente_infra::ErroAdaptador::JsonInvalido(
            "eof".to_string(),
        ));
        let m = traduzir(&erro, &ctx("x"));
        assert!(m.starts_with("Falha ao processar dados do grafo:"));
        assert!(m.contains("eof") || m.contains("JSON")); // detalhe técnico embutido
    }
}
