# Prompt de Nucleação: `lente_catalogo` — o catálogo de apresentação (L2)
Hash do Código: 5f9c82dc

**Camada**: L2 — Casca (apresentação). Pureza de apresentação: zero literais de
texto ao usuário fora daqui (ADR-0002).
**Unidade**: `02_shell/catalogo/src/lib.rs` (crate `lente_catalogo`, arquivo único).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` e os prompts
que ampliaram o catálogo (0027/0030/0031/0034/0035/0047/0048).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **fonte única** do texto apresentado ao usuário (ADR-0002 do Tekt): toda mensagem
de ajuda, erro, rótulo e chave de JSON mora aqui, como **constantes**, para que
nenhum outro crate carregue literal de apresentação. A `CLI`/`app` referenciam estas
constantes; a tradução técnica (o `Display` dos erros) entra como `{detalhe}` dentro
das molduras.

## Comportamento e invariantes

- `HELP_REFERENCIAS_GO` e `ERRO_REFERENCIAS_GO` descrevem a entrada
  complementar da vista HTML e enquadram seu erro de validação.
- Ajuda `HELP_ANALISE_ARTEFATOS` descreve fluxo exclusivo e códigos de regras.

- Completude do diff: chaves `completude`, `falhas`, `crate_name`, `motivo`; valores `completa`/`parcial`; cabeçalhos explícitos e aviso de que ausência de impacto observado em análise parcial não garante ausência de impacto; template por crate com motivo.

- **`Template`** — `struct Template(&'static str)` com `render(&[(&str, &str)]) ->
  String`: substitui `{placeholder}` por valores; placeholder ausente fica literal.
- **~100 `pub const`** organizados por grupo (cada grupo = um contexto de saída):
  - `HELP_*` / `ABOUT_CLI` — textos de ajuda dos argumentos `clap`.
  - `ERRO_*` — mensagens de erro (`Template` com `{detalhe}`/`{alvo}`/…):
    fork/JSON/alvo/id/resolução/fork-sem-uses-kind/workspace/diff.
  - `JSON_*` — chaves dos vários modos (raio, ranking, estrutura/DSM, diff).
  - `ROTULO_*` / `SUFIXO_*` / `*_CABECALHO` — rótulos e cabeçalhos do texto humano.
  - `DIFF_*` — rótulos/templates das três vistas do `--diff` (0048).
  - Valores estáveis (`ESCOPO_*`, `MODO_USES_*`, `CLASSIFICACAO_*`).
- **Estável**: os valores são amigáveis a parsing/UI; mudanças são deliberadas.

## Restrições (L2 — apresentação pura)

- **Zero dep externa** (só stdlib); não importa outras camadas. É um vocabulário de
  texto, não lógica.

## Critérios de Verificação

```
Dado Template("Alvo '{alvo}' não existe") Quando render([("alvo","x")]) Então "Alvo 'x' não existe"
Dado um placeholder ausente Então permanece literal (sem panic)
Dado UTF-8 multibyte no texto Então render preserva
```

Revisão 2026-09-27: saneamento mecânico de Clippy/rustfmt, preservando
semântica e API; qualificação Option::None não é um braço curinga.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"Template","kind":"struct","members":[]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-08-26 | P0079: vocabulário estável da flag, proveniência, relações, alegações e diagnósticos do mapa de correspondência. | `02_shell/catalogo/src/lib.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) do catálogo. Código inalterado. | `02_shell/catalogo/src/lib.rs` |

| 2026-09-26 | Correção da perda de falhas no diff: completude explícita, resultados parciais consultáveis e erro se nenhum crate for analisado. | Unidade deste contrato |

## Integração aditiva de provedores (2026-09-26)

Vocabulário da seleção explícita de provedor, configuração, incompatibilidades e descrição da ferramenta MCP aditiva. Helpers opcionais experimentais são declarados na interface.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.
