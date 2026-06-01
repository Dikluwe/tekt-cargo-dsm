//! Lineage: prompt 00_nucleo/prompt/0020-l2-cli.md
//! Camada:  L2 — Casca (apresentação). Nasce sob o Tekt ADR-0002:
//!          conteúdo de mensagens ao usuário é dado de L2; código nunca
//!          carrega literais de apresentação.
//!
//! Centraliza todas as mensagens que a CLI (e quaisquer outras bocas L2
//! futuras) emite. Sem dependências externas — só stdlib. Os templates são
//! constantes Rust; quando/se o carregamento de catálogo externo
//! (arquivo TOML/FTL, escolha de idioma) for adicionado, vira trabalho de
//! L3, mas o conteúdo continua sendo L2.

#![forbid(unsafe_code)]

/// Template com placeholders `{nome}` substituíveis via [`Template::render`].
///
/// Implementação simples (sem regex, sem crate de templates): percorre a
/// string e substitui ocorrências de `{nome}` pelo valor correspondente.
/// Placeholder ausente do mapa permanece como `{nome}` literal — degradação
/// visível, não panic.
#[derive(Debug, Clone, Copy)]
pub struct Template(pub &'static str);

impl Template {
    /// Renderiza o template substituindo cada `{chave}` pelo valor.
    pub fn render(&self, vars: &[(&str, &str)]) -> String {
        let mut saida = String::with_capacity(self.0.len());
        let bytes = self.0.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' {
                if let Some(fim) = self.0[i + 1..].find('}') {
                    let nome = &self.0[i + 1..i + 1 + fim];
                    let valor = vars.iter().find(|(k, _)| *k == nome).map(|(_, v)| *v);
                    match valor {
                        Some(v) => saida.push_str(v),
                        None => {
                            // Degradação visível: mantém o literal.
                            saida.push('{');
                            saida.push_str(nome);
                            saida.push('}');
                        }
                    }
                    i += 1 + fim + 1;
                    continue;
                }
            }
            // char comum (cuidado com UTF-8 multibyte)
            let ch = self.0[i..].chars().next().unwrap();
            saida.push(ch);
            i += ch.len_utf8();
        }
        saida
    }
}

// =============================================================================
// ERROS — templates apresentados ao usuário
// =============================================================================

pub const ERRO_FORK_AUSENTE: Template = Template(
    "Não foi possível invocar o cargo-modules: {detalhe}",
);
pub const ERRO_JSON_INVALIDO: Template = Template(
    "Falha ao processar dados do grafo: {detalhe}",
);
pub const ERRO_ALVO_INEXISTENTE: Template = Template(
    "Alvo '{alvo}' não existe no grafo",
);
pub const ERRO_ID_INEXISTENTE: Template = Template(
    "Nó com id {id} não existe no grafo",
);
pub const ERRO_RESOLUCAO: Template = Template(
    "Falha na resolução de colisão: {detalhe}",
);
pub const ERRO_GENERICO_PIPELINE: Template = Template(
    "Falha no pipeline: {detalhe}",
);

// Validações da CLI (não vêm do `ErroLente`).
pub const ERRO_FONTE_NAO_INFORMADA: Template = Template(
    "Informe --grafo <arquivo.json> ou --pacote <nome>",
);
pub const ERRO_ALVO_NAO_INFORMADO: Template = Template(
    "Informe --alvo <path> ou --alvo-id <N>",
);
pub const ERRO_LER_ARQUIVO: Template = Template(
    "Não foi possível ler {arquivo}: {detalhe}",
);

// =============================================================================
// ROTULOS — usados no texto humano e nas chaves do JSON
// =============================================================================

pub const ROTULO_ALVO: &str = "Alvo";
pub const ROTULO_ALVO_PEDIDO: &str = "Alvo pedido";
pub const ROTULO_ALVO_RESOLVIDO: &str = "Alvo resolvido";
pub const ROTULO_CLASSIFICACAO: &str = "Classificação";
pub const ROTULO_DIRETOS: &str = "Impacto direto";
pub const ROTULO_TRANSITIVOS: &str = "Transitivo";
pub const ROTULO_IMPACTADOS: &str = "Impactados";

// Chaves do JSON (default snake_case)
pub const JSON_ALVO: &str = "alvo";
pub const JSON_ALVO_PEDIDO: &str = "alvo_pedido";
pub const JSON_ALVO_RESOLVIDO: &str = "alvo_resolvido";
pub const JSON_CLASSIFICACAO: &str = "classificacao";
pub const JSON_DIRETOS: &str = "diretos";
pub const JSON_TRANSITIVOS: &str = "transitivos";
pub const JSON_IMPACTADOS: &str = "impactados";

// Sufixo "N itens"
pub const SUFIXO_ITENS: &str = "itens";

// =============================================================================
// CLASSIFICAÇÕES — string legível para `Classificacao` do lente_core
// =============================================================================

pub const CLASSIFICACAO_ISOLADO: &str = "Isolado";
pub const CLASSIFICACAO_FOLHA: &str = "Folha";
pub const CLASSIFICACAO_BASE: &str = "Base";
pub const CLASSIFICACAO_INTERMEDIARIO: &str = "Intermediário";

// =============================================================================
// HELP / ABOUT — para clap
// =============================================================================

pub const ABOUT_CLI: &str = "O que quebra se eu mexer aqui?";
pub const HELP_GRAFO: &str = "JSON pronto (gerado pelo fork cargo-modules 0.27.0)";
pub const HELP_PACOTE: &str = "Nome do pacote — a lente invoca o fork internamente";
pub const HELP_ALVO: &str = "Path do alvo no grafo (ex.: 'ErroRaio::<Display>::fmt')";
pub const HELP_ALVO_ID: &str = "Id do alvo no grafo (alternativa ao --alvo)";
pub const HELP_TEXT: &str = "Saída em texto humano-legível (default é JSON)";
pub const HELP_VERBOSE: &str = "Inclui lista completa de itens impactados";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_substitui_um_placeholder() {
        let t = Template("ola {nome}!");
        assert_eq!(t.render(&[("nome", "mundo")]), "ola mundo!");
    }

    #[test]
    fn render_substitui_multiplos_placeholders() {
        let t = Template("{a} + {b} = {c}");
        let r = t.render(&[("a", "1"), ("b", "2"), ("c", "3")]);
        assert_eq!(r, "1 + 2 = 3");
    }

    #[test]
    fn render_placeholder_ausente_permanece_literal() {
        let t = Template("ola {nome}, idade {idade}");
        let r = t.render(&[("nome", "x")]);
        assert_eq!(r, "ola x, idade {idade}");
    }

    #[test]
    fn render_lida_com_utf8_multibyte() {
        let t = Template("açúcar com {ingrediente}");
        let r = t.render(&[("ingrediente", "canela")]);
        assert_eq!(r, "açúcar com canela");
    }

    #[test]
    fn render_string_sem_placeholders_inalterada() {
        let t = Template("sem chaves aqui");
        assert_eq!(t.render(&[]), "sem chaves aqui");
    }

    #[test]
    fn constantes_de_apresentacao_nao_sao_vazias() {
        // Sanidade — protege contra esvaziamento acidental.
        assert!(!ABOUT_CLI.is_empty());
        assert!(!ROTULO_ALVO.is_empty());
        assert!(!CLASSIFICACAO_FOLHA.is_empty());
        assert!(!ERRO_ALVO_INEXISTENTE.0.is_empty());
        assert!(!HELP_GRAFO.is_empty());
    }

    #[test]
    fn templates_de_erro_concretos_renderizam() {
        let m = ERRO_ID_INEXISTENTE.render(&[("id", "42")]);
        assert_eq!(m, "Nó com id 42 não existe no grafo");

        let m = ERRO_ALVO_INEXISTENTE.render(&[("alvo", "foo::bar")]);
        assert_eq!(m, "Alvo 'foo::bar' não existe no grafo");
    }
}
