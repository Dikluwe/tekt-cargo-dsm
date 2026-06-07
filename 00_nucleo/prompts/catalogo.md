# Prompt de Nucleação: `lente_catalogo` — o catálogo de apresentação (L2)
Hash do Código: 2b3d1675

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

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"Template","kind":"struct","members":[]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) do catálogo. Código inalterado. | `02_shell/catalogo/src/lib.rs` |
