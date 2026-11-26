# Prompt de Nucleação: `lente_cli::args` — os argumentos da CLI (L2)
Hash do Código: 0fb1d48a

**Camada**: L2 — Casca (apresentação pura). Os textos de ajuda vêm do
`lente_catalogo` (ADR-0002).
**Unidade**: `02_shell/cli/src/args.rs` (crate `lente_cli`).
**Origem de trabalho** (referência): `00_nucleo/prompt/0020-l2-cli.md` (+ os modos
0027/0030/0031/0034/0047/0048).

> Prompt de **nucleação** (descreve o código existente).

---

## Propósito

A **definição declarativa** (clap `derive`) dos argumentos do binário `lente`:
qual modo o usuário pede e com quais opções. Só **estrutura de argumentos** — a
composição (chamar a orquestração) vive no `lente_app` (L4), e a apresentação dos
resultados no `saida`.

## Comportamento e invariantes

- `--analise-artefatos CONFIG.json` é um fluxo exclusivo de snapshots/regras/DSM.
  Não combina com flags dos fluxos legados ou de extração de provedores.

- **`Cli`** (`#[derive(Parser)]`) — as flags: fonte (`--grafo`/`--pacote`,
  mutuamente exclusivas), alvo (`--alvo`/`--alvo-id`), modos (`--ranking`/`--top`,
  `--estrutura`/`--so-referencia`, `--diff`/`--repo`/`--vista`), escopo
  (`--filtrar-stdlib`), saída (`--text`/`--verbose`). Os modos são mutuamente
  exclusivos via `conflicts_with_all`; `--vista` `requires --diff`.
- **P0079:** `--mapa-correspondencia <arquivo.toml>` requer `--comparar` e injeta
  correspondências documentadas sem alterar o comportamento quando ausente.
- **`Vista`** (`#[derive(ValueEnum)]`) — `Resumo`/`Item`/`Camadas` (do `--diff`,
  prompt 0048).
- **Todos os `help`/`about`** vêm do `lente_catalogo` — **nenhum literal de
  apresentação** aqui (ADR-0002).

- `--referencias-go` recebe um índice de referências de arquivos e exige
  `--estrutura --html --grafo`. A opção apenas complementa o HTML.
- `--indice-scip` é repetível e requer `--estrutura --html --raiz-scip`.
  Usa os índices binários como fonte da vista por arquivo e é exclusivo
  das fontes `--grafo`, `--pacote`, `--provedor` e `--referencias-go`.
  O recorte corresponde aos arquivos e referências presentes nesses índices.
- `--indice-swift` recebe o IndexStore de Swift/Clang. Aceita a mesma raiz,
  o modo HTML e índices SCIP opcionais. A raiz exige ao menos uma fonte.
- `--artefato-ffi` requer `--indice-scip` e recebe um ELF final com debug
  do mesmo build para ligar chamadas FFI indexadas a C/C++.
  Python e Java usam a biblioteca compartilhada carregada pelas fontes.

## Restrições (L2 — apresentação)

- Importa só `clap` + `lente_catalogo`. Não importa L1/L3/L4 (a estrutura de args é
  independente da orquestração).

## Critérios de Verificação

```
Dado --grafo e --pacote juntos Então clap rejeita (conflicts_with)
Dado --vista sem --diff Então clap rejeita (requires)
Dado --vista camadas Então Vista::Camadas
```

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[],"types":[{"name":"Vista","kind":"enum","members":["Resumo","Item","Camadas"]},{"name":"Cli","kind":"struct","members":["analise_artefatos","provedor","config_provedor","grafo","pacote","alvo","alvo_id","ranking","top","estrutura","diff","repo","vista","so_referencia","filtrar_stdlib","text","html","saida","referencias_go","indices_scip","indice_swift","raiz_scip","artefato_ffi","base_compilacao","fluxos","completo","comparar","antes","depois","mapa_correspondencia","verbose"]}],"reexports":[]} -->

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Nucleação (migração de convenção, prompt 0063) dos argumentos. Código inalterado. | `02_shell/cli/src/args.rs` |

## Integração aditiva de provedores (2026-09-26)

Adiciona --provedor (rust/python/typescript/zig/c/cpp/go/pacote-analise) e --config-provedor. Ausência de seleção mantém fluxo anterior; validação semântica e execução pertencem ao L4/L3.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.

`--base-compilacao` requer `--indice-scip` e recebe o JSON do mesmo build para
Explorar chamadas em C/C++. `--fluxos` requer a raiz SCIP e recebe um JSON de fluxos
publicados junto ao HTML, no formato posicional versionado da vista.
