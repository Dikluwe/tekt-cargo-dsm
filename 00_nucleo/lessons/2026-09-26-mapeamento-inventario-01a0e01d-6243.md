# Mapeamento de diff por inventário explícito

Data: 2026-09-26. Tarefa exclusiva: mapeamento-inventario-01a0e01d-6243.

## Escopo e decisão

Alterados somente `01_core/core/src/domain/mapeamento.rs`, seu contrato
`00_nucleo/prompts/mapeamento.md` e este laudo. Os dois arquivos estavam sem
alterações locais no início da tarefa; as alterações dos demais trabalhos foram
preservadas. Leitura: AGENTS.md, contrato de mapeamento, spec do grafo e ADR-0002.

Adicionada `mapear_diff_com_inventario`, com tipos auxiliares locais. Inventário
explicita arquivos analisados e fontes conhecidas não analisadas; ausência é
cobertura desconhecida. Nenhuma inferência por extensão. Associações permitem
muitos arquivos por nó e muitos nós por arquivo, por linhas inclusivas ou pelo
arquivo inteiro. Evidências preservam essa granularidade por arquivo/ID/escopo.

Caminhos normalizados pelo chamador são chaves PathBuf sem reconciliação por
sufixo ou relativo/absoluto. Relativos pressupõem a mesma base lógica. Sem I/O,
novas dependências ou mudança de tipos compartilhados. Validação integral de
IDs referenciados, intervalos e caminhos duplicados antes de produzir resultado;
erros e resultados ordenados e deduplicados. Unicidade dos IDs do grafo segue a
spec existente. A API Rust anterior conserva sua lógica e assinatura.

## Verificação reproduzível

Comandos executados da raiz, com ferramentas já disponíveis:

```sh
export RUSTUP_AUTO_INSTALL=0
export CARGO_TARGET_DIR=/tmp/lente-mapeamento-inventario-target
cargo test --offline --locked -p lente_core mapeamento
cargo test --offline --locked -p lente_core
cargo test --offline --locked --workspace
cargo test --offline --locked -p lente_wiring tests::analisar_diff_propaga_falhas_do_cache_e_rejeita_falha_total -- --exact
```

- Mapeamento: 17 testes passaram (7 novos e 10 regressões Rust).
- Core: 79 testes passaram; doctests sem casos, sem falhas.
- Workspace: 375 passaram, 1 falhou e 34 foram ignorados até a interrupção
  pelo Cargo. Não se declara aprovação integral do workspace.
- A falha foi reproduzida isoladamente em
  `lente_wiring::tests::analisar_diff_propaga_falhas_do_cache_e_rejeita_falha_total`:
  esperava `Parcial`, recebeu `Completa`, na asserção de completude após
  invalidar o cache. Esse teste usa a API anterior, cuja lógica não mudou.
  Wiring/cache contêm trabalho concorrente e estão fora da área de escrita;
  não foram alterados nem se atribui aqui uma causa definitiva para a falha.
- `rustfmt` aplicado somente ao componente; `git diff --check` sem erros.
- Snapshot da interface e hashes recíprocos regenerados com `crystalline-lint`
  em cópia isolada dos dois arquivos; V5/V6/V7 sem violações. A cópia de volta
  verificou previamente que os arquivos originais não haviam mudado.

Cobertura nova: C, Python, TypeScript, Zig e arquivo sem extensão; pacote com
vários arquivos; arquivo com vários nós; posições ausentes; limites de intervalos;
arquivo inteiro; caminhos homônimos e absolutos/relativos; analisado sem nós;
fonte não analisada e fora do inventário; IDs inexistentes; início zero e faixa
invertida; duplicação de associações/diff; permutação de entradas; paths de nós
iguais com IDs distintos. Faixas vazias só fazem censo.

## Limites

Não integra leitores, adaptadores, wiring ou CLI. Não analisa remoções. Não
valida normalização nem acessa o filesystem. A precisão do inventário é
responsabilidade do adaptador/chamador; esses testes não demonstram completude
semântica de qualquer extrator.
