# Prompt de Nucleação: `lente_app::main` — o ponto de entrada (L4)
Hash do Código: 5486eec0

**Camada**: L4 — Fiação (app/ponto de composição). Importa L1/L2/L4.
**Unidade**: `04_wiring/app/src/main.rs` (crate `lente_app`, o binário `lente`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (o `main`
veio da CLI); relocado ao L4 no Estágio 3 (prompt 0057).

> Prompt de **nucleação** (lógica ativa de dispatch — fica no walk; `main`/`run`
> não são `pub`, então o snapshot é mínimo).

---

## Propósito

O **ponto de composição**: o binário `lente`. Parseia os argumentos (`lente_cli::
args`, L2), chama a orquestração (`lente_wiring`, L4), trata o `ErroLente`, e chama
os formatadores (`lente_cli::saida`, L2) — devolvendo a saída no canal certo
(stdout/stderr) e o código de saída. É o que **tira a apresentação (L2) de depender
do L4** (a CLI virou lib pura no 0057).

## Comportamento e invariantes

- O fluxo exclusivo `--analise-artefatos` chama o componente L3 `analises`, imprime
  JSON em stdout e preserva códigos de domínio 1/2 das regras. Erros de transporte
  e configuração usam stderr; entrada inválida desse fluxo retorna 3.

- ADR-0007: leitura da entrada JSON delegada ao L3, mantendo flags e tratamento de erro de arquivo existentes.

- **`main()`** — `Cli::parse()` → `run(cli)` → imprime o `Ok(String)` (stdout,
  exit 0) ou a mensagem de erro (stderr, exit code).
- **`run(cli)`** — roteia entre os modos (diff / estrutura / ranking / per-nó),
  monta `FonteGrafo`/`AlvoBusca`/`Escopo`/`ModoUses` (do L1) das flags, chama a
  orquestração e formata; erro do wiring → `SaidaErro` via `erro::traduzir`.
- `--estrutura --html` solicita a estrutura com evidências ao wiring e delega
  a geração do HTML ao formatador L2. Texto e JSON usam a estrutura usual.
  Quando informado, lê `--referencias-go` localmente, valida o índice no
  formatador antes de gravar o HTML e propaga erro com o caminho da entrada.
- Quando informado `--indice-scip`, entrega os índices e `--raiz-scip` ao
  adaptador L3. Projeta as referências admitidas com L1, usa o cálculo
  existente de estrutura e grava a vista HTML somente após a admissão.
  Quando presente, `--artefato-ffi` pede ao adaptador L3 as ligações
  nativas apoiadas pelo artefato antes do cálculo da estrutura.
- **`SaidaErro`** — código (1=pipeline, 2=args) + mensagem já traduzida.
- **L4 importa L1/L2/L4** (composição — para baixo, permitido). A apresentação
  (args/saida) e o catálogo são L2; a orquestração é L4; o vocabulário é L1.

## Restrições (L4)

- É o topo: compõe as camadas abaixo. Não recria lógica de domínio nem formata —
  delega. O `main` não é `pub` (binário).

## Critérios de Verificação

```
Dado --grafo <json> --alvo <path> Quando run Então o JSON/texto do raio
Dado --diff --vista resumo Quando run Então a vista resumo
Dado um erro do pipeline Quando run Então SaidaErro com mensagem do catálogo
```

Revisão 2026-09-27: saneamento mecânico de Clippy/rustfmt, preservando
semântica e API; qualificação Option::None não é um braço curinga.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"SaidaErro","kind":"struct","members":["codigo","mensagem"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-08-26 | P0079: `run_comparar` lê o mapa opcional via L3 e o injeta no wiring; erro de arquivo/parse sai como erro de uso, sem executar comparação parcial. | `04_wiring/app/src/main.rs` |
| 2026-06-07 | Nucleação (migração de convenção, prompt 0065) do ponto de entrada. Código inalterado. | `04_wiring/app/src/main.rs` |

## Integração aditiva de provedores (2026-09-26)

Seleção externa roteia antes dos modos legados. Rejeita diff/comparação/filtros Rust/HTML/texto/fontes legadas antes de ler configuração ou extrair. Consulta raio/ranking/estrutura usa envelope comum do wiring. --provedor rust mantém fluxo anterior.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.

O modo HTML de fontes admite um IndexStore Swift/Clang isolado ou com SCIP/ELF e compõe os adaptadores antes da projeção estrutural.

A origem Go do HTML compõe a projeção independente de chamadas com os índices de referências legados ou SCIP.

A composição da exploração fornece ao adaptador as referências com origem dos índices SCIP ou o caminho do índice legado.

A geração SCIP usa `--base-compilacao` quando fornecida para adicionar, via
adaptador L3, chamadas C/C++ e cobertura antes da formatação. Lê o arquivo de
`--fluxos` quando fornecido e entrega o texto ao formatador antes da gravação.

Na DSM por arquivo, após reunir as fontes de todos os adaptadores, o ponto de
composição entrega ao adaptador L3 a mesma raiz para carregar os prompts
referenciados pelo código. O índice enriquecido segue ao formatador tanto na
saída HTML usual quanto na saída com fluxos publicados.
Quando `--textfold` está presente, a composição solicita os níveis ao L3
antes de formatar o HTML e propaga falha do adaptador.
