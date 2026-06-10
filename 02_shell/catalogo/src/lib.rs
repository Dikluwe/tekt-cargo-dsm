//! Crystalline Lineage
//! @prompt 00_nucleo/prompts/catalogo.md
//! @prompt-hash 28b667e5
//! @layer L2
//! @updated 2026-06-07
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
pub const ERRO_WORKSPACE: Template = Template(
    "Falha ao montar o grafo de workspace: {detalhe}",
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
// Prompt 0071 — vista DSM em HTML.
pub const HELP_HTML: &str =
    "Gera a DSM de estrutura como HTML autocontido e imprime o caminho (só com --estrutura)";
pub const HELP_SAIDA: &str = "Caminho do HTML (--html). Default: lente-estrutura.html no cwd";
/// Prompt 0072: a vista `--html` tem default `seu-codigo`; esta flag restaura o `completo`.
pub const HELP_COMPLETO: &str =
    "Inclui sysroot/stdlib na vista --html (restaura o escopo completo; default da vista é seu-codigo)";
/// Dica de escopo no cabeçalho da vista quando o recorte filtrado está ativo (0072).
pub const DSM_ESCOPO_DICA: &str =
    " (sysroot/stdlib ocultos — use --completo para o escopo completo)";
/// Nome de arquivo default do `--html` (cwd).
pub const DSM_SAIDA_PADRAO: &str = "lente-estrutura.html";
/// Mensagem impressa após gravar a vista (stdout do modo `--html`).
pub const DSM_ESCRITO: Template = Template("Vista DSM escrita em {caminho} — abra no navegador.");
/// Falha de I/O ao gravar o HTML.
pub const DSM_ERRO_ESCRITA: Template =
    Template("não consegui gravar a vista em {caminho}: {detalhe}");
pub const HELP_RANKING: &str =
    "Top-N por impacto no pacote. Conflita com --alvo/--alvo-id.";
pub const HELP_TOP: &str = "Quantos itens no top-N do ranking (default 10)";
pub const HELP_FILTRAR_STDLIB: &str =
    "Filtra stdlib (core/std/alloc/proc_macro/test). Default: escopo completo (com stdlib).";
pub const HELP_ESTRUTURA: &str =
    "Estrutura do pacote: módulos, dependências e ciclos (vista global tipo DSM). \
     Conflita com --ranking/--alvo/--alvo-id.";
pub const HELP_SO_REFERENCIA: &str =
    "Conta só arestas `Uses` de referência (uso de tipo direto). Descarta \
     declarações `use` no nível do módulo (Limite 4). Só efeito com \
     --estrutura. Default: todas as `Uses`.";

pub const ERRO_FORK_SEM_USES_KIND: Template = Template(
    "O fork `cargo-modules` instalado não emite `uses_kind` por aresta. \
     Atualize o fork (pós-commit `b44aa96`) para usar --so-referencia. \
     Detalhe técnico: {detalhe}",
);

// =============================================================================
// RANKING — prompt 0027 (cabeçalho ampliado pelo 0030 para incluir escopo)
// =============================================================================

/// Cabeçalho da saída-texto do ranking. Formato (pós-0030):
///   "Ranking de impacto (escopo: {escopo}) — top {n}:\n"
///   "  #  Impacto  Classificação    Path"
pub const RANKING_CABECALHO: Template =
    Template("Ranking de impacto (escopo: {escopo}) — top {n}:");
pub const RANKING_COLUNAS: &str = "  #  Impacto  Classificação    Path";

/// Chaves do JSON do ranking.
pub const JSON_RANKING: &str = "ranking";
pub const JSON_POSICAO: &str = "posicao";
pub const JSON_IMPACTO: &str = "impacto";
pub const JSON_PATH: &str = "path";

/// Erro de validação de CLI: --ranking não combina com --alvo/--alvo-id.
pub const ERRO_RANKING_COM_ALVO: Template = Template(
    "Use --ranking OU --alvo/--alvo-id, não os dois",
);

// =============================================================================
// ESCOPO — prompt 0030 (rótulos comuns às duas saídas)
// =============================================================================

/// Rótulo legível para a linha "Escopo:" no texto humano (modo per-nó).
pub const ROTULO_ESCOPO: &str = "Escopo";

/// Chave JSON do escopo, presente nos dois modos (raio e ranking).
pub const JSON_ESCOPO: &str = "escopo";

/// Valores do escopo na saída (JSON e texto). Mantidos curtos e estáveis
/// para serem amigáveis a parsing e mostrarem bem na CLI/UI.
pub const ESCOPO_COMPLETO: &str = "completo";
pub const ESCOPO_SEU_CODIGO: &str = "seu-codigo";

// =============================================================================
// ESTRUTURA — prompt 0031 (vista global: módulos, dependências, ciclos)
// =============================================================================

/// Cabeçalho do texto humano do modo estrutura. Placeholders:
/// `{escopo}`, `{modo_uses}`, `{n}` (módulos), `{c}` (ciclos).
/// (Pós-prompt 0034: declara o `modo_uses` ao lado do escopo.)
pub const ESTRUTURA_CABECALHO: Template = Template(
    "Estrutura de módulos (escopo: {escopo}, uses: {modo_uses}) — {n} módulos, {c} ciclos:",
);

/// Subseções do texto.
pub const ESTRUTURA_CICLOS_TITULO: &str = "Ciclos:";
pub const ESTRUTURA_DEPENDENCIAS_TITULO: &str = "Dependências módulo → módulo:";
pub const ESTRUTURA_SEM_CICLOS: &str = "(nenhum ciclo entre módulos)";

/// Chaves do JSON do modo estrutura — formato DSM-friendly.
/// `{ escopo, modulos: [path], dependencias: [{de, para}], ciclos: [[path]] }`
pub const JSON_MODULOS: &str = "modulos";
pub const JSON_DEPENDENCIAS: &str = "dependencias";
pub const JSON_CICLOS: &str = "ciclos";
pub const JSON_DE: &str = "de";
pub const JSON_PARA: &str = "para";
/// Peso de acoplamento da aresta módulo→módulo (prompt 0071): nº de arestas-de-item
/// `Uses` que colapsaram nela. Campo aditivo no `--estrutura --json`.
pub const JSON_PESO: &str = "peso";
pub const JSON_MODO_USES: &str = "modo_uses";
pub const MODO_USES_TODAS: &str = "todas";
pub const MODO_USES_SO_REFERENCIA: &str = "so-referencia";

// Prompt 0035 — ordenamento da DSM (matriz como dado).
pub const JSON_ORDEM: &str = "ordem";
pub const JSON_BLOCOS: &str = "blocos";

// Prompt 0071 — vista DSM em HTML (campos extras do dado embutido na tela).
pub const JSON_PACOTE: &str = "pacote";
pub const JSON_LIMITE: &str = "limite";
pub const JSON_ESCOPO_DICA: &str = "escopo_dica";
/// Raio por módulo embutido na vista HTML (prompt 0073): por posição na `ordem`,
/// `{m:[índices montante], j:[índices jusante]}`. Índices (compacto), não paths.
pub const JSON_RAIOS: &str = "raios";
pub const JSON_RAIO_SEMANTICA: &str = "raio_semantica";

// Prompt 0074 — modo paridade (`--comparar`).
pub const HELP_COMPARAR: &str =
    "Compara a estrutura de duas raízes (--antes/--depois): projeto vs refatoração";
pub const HELP_ANTES: &str = "Raiz do lado antes (--comparar): diretório de crate";
pub const HELP_DEPOIS: &str = "Raiz do lado depois (--comparar): diretório de crate";
/// Faltam --antes e/ou --depois no modo comparar.
pub const COMPARAR_FALTA_RAIZ: &str =
    "informe as duas raízes: --antes <dir> e --depois <dir>";
/// Cabeçalho do texto da comparação.
pub const COMPARAR_CABECALHO: Template = Template(
    "Paridade: {antes} (antes) × {depois} (depois) — escopo={escopo}, uses={uses}",
);
/// O limite honesto do pareamento (sai na saída, não escondido — prompt 0074).
pub const COMPARAR_LIMITE: &str = "Pareamento por path normalizado na raiz do crate \
    (crate renomeado pareia; módulo movido aparece sem-par dos dois lados — não há \
    detecção de movido por similaridade). Sem nota única: conjuntos e contagens; quem \
    julga é você.";
/// Erro identificando o lado que falhou.
pub const COMPARAR_ERRO_LADO: Template = Template("lado {lado}: {detalhe}");
pub const COMPARAR_LADO_ANTES: &str = "antes";
pub const COMPARAR_LADO_DEPOIS: &str = "depois";
// Títulos das seções do texto.
pub const COMPARAR_TIT_RESUMO: &str = "Resumo:";
pub const COMPARAR_TIT_SEM_PAR_ANTES: &str = "Só no antes (sem par):";
pub const COMPARAR_TIT_SEM_PAR_DEPOIS: &str = "Só no depois (sem par):";
pub const COMPARAR_TIT_ARESTAS_SO_ANTES: &str = "Dependências que sumiram:";
pub const COMPARAR_TIT_ARESTAS_SO_DEPOIS: &str = "Dependências que apareceram:";
pub const COMPARAR_TIT_DELTAS_PESO: &str = "Maiores mudanças de acoplamento (peso):";
pub const COMPARAR_TIT_CICLOS: &str = "Ciclos (lado a lado):";
// Chaves do JSON da comparação.
pub const JSON_NOME_ANTES: &str = "nome_antes";
pub const JSON_NOME_DEPOIS: &str = "nome_depois";
pub const JSON_PAREADOS: &str = "pareados";
pub const JSON_SEM_PAR_ANTES: &str = "sem_par_antes";
pub const JSON_SEM_PAR_DEPOIS: &str = "sem_par_depois";
pub const JSON_ARESTAS_COMUNS: &str = "arestas_comuns";
pub const JSON_ARESTAS_SO_ANTES: &str = "arestas_so_antes";
pub const JSON_ARESTAS_SO_DEPOIS: &str = "arestas_so_depois";
pub const JSON_PESO_ANTES: &str = "peso_antes";
pub const JSON_PESO_DEPOIS: &str = "peso_depois";
pub const JSON_CICLOS_ANTES: &str = "ciclos_antes";
pub const JSON_CICLOS_DEPOIS: &str = "ciclos_depois";
pub const JSON_QUANTIDADE: &str = "quantidade";
pub const JSON_MAIOR: &str = "maior";
pub const JSON_LIMITE_PAREAMENTO: &str = "limite_pareamento";
/// Rótulo do painel de raio na vista (semântica + limite §3 — honestidade na interface).
pub const DSM_RAIO_SEMANTICA: &str = "Raio estrutural (alcançabilidade por item, projetada a \
    módulos): montante = quem depende deste (sente a mudança); jusante = do que este depende. \
    Estar no raio é estar na forma — não significa que vai quebrar.";
/// Declaração de limite (proposta §3) embutida no cabeçalho da vista HTML — a
/// honestidade é parte da interface, como nas descrições do MCP (0070). HTML
/// curto (renderizado dentro de `<p class="limite">`).
pub const DSM_LIMITE_HTML: &str = "<strong>O que esta tela mostra:</strong> a forma \
    <strong>estática e estrutural</strong> — quem depende de quem (arestas <code>Uses</code>), \
    a ordem topológica e os ciclos. <strong>Não</strong> mostra impacto comportamental nem \
    afirma que algo vai quebrar: mostra a forma; quem decide é você.";
pub const ESTRUTURA_ORDEM_TITULO: &str = "Ordem da DSM (topológica + blocos):";
pub const ESTRUTURA_ORDEM_LINHA_LIVRE: &str = "  ";
pub const ESTRUTURA_ORDEM_LINHA_BLOCO: &str = "  ◆";

// =============================================================================
// DIFF — prompt 0047 (modo --diff: o que o diff toca, view-agnóstico → JSON)
// =============================================================================

pub const HELP_DIFF: &str =
    "Mapeia o diff do repositório (git) aos nós que ele toca e emite o impacto \
     em JSON. Opera na raiz do repo (ver --repo). Mutuamente exclusivo com os \
     outros modos.";
pub const HELP_REPO: &str =
    "Raiz do repositório a analisar no modo --diff (default: diretório atual)";

pub const ERRO_DIFF: Template =
    Template("Falha ao analisar o diff do repositório: {detalhe}");

/// Chaves do JSON do modo `--diff` (view-agnóstico). Esquema:
/// `{ tocados: [{path, id, classificacao, montante, jusante}],
///    combinado: { montante: [{path, profundidade}], jusante: [...] },
///    ligados: [path], soltos: [path], nao_fonte: [path],
///    fantasmas: [{path, referenciado_por: [crate]}] }`
pub const JSON_TOCADOS: &str = "tocados";
pub const JSON_ID: &str = "id";
pub const JSON_MONTANTE: &str = "montante";
pub const JSON_JUSANTE: &str = "jusante";
pub const JSON_COMBINADO: &str = "combinado";
pub const JSON_PROFUNDIDADE: &str = "profundidade";
pub const JSON_LIGADOS: &str = "ligados";
pub const JSON_SOLTOS: &str = "soltos";
pub const JSON_NAO_FONTE: &str = "nao_fonte";
pub const JSON_FANTASMAS: &str = "fantasmas";
pub const JSON_REFERENCIADO_POR: &str = "referenciado_por";

// =============================================================================
// DIFF — vistas de texto (prompt 0048): --vista resumo | item | camadas
// Ausente = JSON (0047, intocado). Renderizadores sobre o ResultadoDiff.
// =============================================================================

pub const HELP_VISTA: &str =
    "Vista de texto do --diff: resumo | item | camadas. Ausente: JSON (default).";

/// Rodapé comum às três vistas: o censo do untracked e o solto listado.
pub const DIFF_UNTRACKED: Template =
    Template("untracked: {lig} compilados {sep} {solto} sem mod {sep} {nf} não-fonte");
pub const DIFF_SEM_MOD: &str = "sem mod (não compilado):";
pub const DIFF_FANTASMAS: Template = Template("fantasmas: {n}");
pub const DIFF_SEM_TOCADOS: &str = "(nenhum nó tocado)";
pub const DIFF_VAZIO: &str = "—";
pub const DIFF_SEP: &str = "·";

/// Vista `resumo`.
pub const DIFF_RESUMO_CABECALHO: Template = Template("diff: {n} tocados em {c} crate(s)");
pub const DIFF_ROTULO_MONTANTE: &str = "pode quebrar (montante)";
pub const DIFF_ROTULO_JUSANTE: &str = "depende de (jusante)";

/// Vista `item`.
pub const DIFF_ITEM_CABECALHO: Template = Template("{n} tocados:");
pub const DIFF_ITEM_PODE_QUEBRAR: &str = "pode quebrar";
pub const DIFF_ITEM_DEPENDE_DE: &str = "depende de";

/// Vista `camadas`.
pub const DIFF_CAMADAS_TOCADOS_POR_CRATE: &str = "tocados por crate:";
pub const DIFF_CAMADAS_POR_CRATE: &str = "pode quebrar, por crate:";

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
