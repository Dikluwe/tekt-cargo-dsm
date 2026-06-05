# Prompt: Diagnóstico claro para diretório inexistente na detecção (`lente_infra`)

**Camada**: L3 — Infraestrutura
**Criado em**: 2026-06-01
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0023, D5 — `invocar_metadata` com
`current_dir = Some(dir)` num diretório **inexistente** falha no spawn do
`Command` com `io::ErrorKind::NotFound`, indistinguível de `cargo` ausente do
PATH, e é mapeado para `ErroMetadata::BinarioNaoEncontrado` ("cargo ausente").
A mensagem aponta a causa errada. Em relação ao laudo 0022 isto é **piora**:
lá, um diretório inexistente dava "Cargo.toml não encontrado" (claro), porque
a leitura era por `fs`. A D5 registrou a colisão como aceita; este prompt a
fecha.
**Pré-requisito**: laudo 0023 (`metadata.rs`, `ErroMetadata`, as duas portas
de detecção; teste `diretorio_inexistente_da_erro_de_deteccao_alvo` que
documenta a colisão).
**Posição**: último débito da leva bin+lib / metadata. Antes do filtro de
stdlib. Pequeno e isolado.
**Arquivos afetados (a confirmar na Fase 1)**: `03_infra/src/metadata.rs`,
testes; possivelmente `03_infra/src/lib.rs` (Display da variante embrulhada).

---

## Contexto

Apontar a lente para um diretório que não existe (erro de digitação no
caminho, p.ex.) é um erro comum e de causa óbvia. Hoje a resposta é "cargo
ausente do PATH", que manda o usuário investigar a coisa errada. A causa real
("esse diretório não existe") é barata de detectar **antes** de invocar o
subprocesso.

O caminho `--pacote` (`invocar_fork` → `current_dir = None`, herda o cwd) não
tem esse problema: o cwd do processo sempre existe. A colisão é só no caminho
que recebe um diretório explícito (`detectar_pacote_e_alvo_por_diretorio`).

---

## Restrições estruturais

- **L3, mudança mínima.** Toca só o caminho de detecção por diretório.
- **Sem mudar assinaturas públicas** (`extrair_grafo`, `invocar_fork`,
  `desserializar_grafo`). Aditivo.
- **Não toca o caminho `--pacote`** (cwd sempre existe).
- **Não toca o fork, o `lente_core` (L1) nem a E2** (quarentena).
- **Um só invocador do fork e um só de metadata** — invariante do laudo 0023
  preservado; este prompt não cria subprocesso novo.

---

## Fase 1 — Leitura primeiro

1. `metadata.rs`: `invocar_metadata(current_dir)` — onde o `current_dir` é
   aplicado ao `Command`, e o mapeamento atual `NotFound → BinarioNaoEncontrado`.
2. O teste `diretorio_inexistente_da_erro_de_deteccao_alvo` (o que ele afirma
   hoje) e o `Display` de `ErroMetadata`.
3. Confirmar que nada mais depende do mapeamento `NotFound → BinarioNaoEncontrado`
   **no caminho com diretório** (o caminho cwd/`--pacote` deve manter esse
   mapeamento — ali `NotFound` é mesmo "cargo ausente").

---

## Fase 2 — Conserto

- **Antes** de invocar com um diretório explícito, checar que ele existe (e que
  é diretório). Se não, devolver erro próprio.
- **Variante nova** (sugestão: `ErroMetadata::DiretorioInexistente(PathBuf)`),
  **não** reusar `PacoteNoDiretorioNaoEncontrado` — esta significa "o diretório
  existe, mas nenhum pacote casou", causa diferente. Reusá-la repetiria o tipo
  de colisão que estamos consertando. `Display` claro, com o caminho.
- Com a checagem, um `NotFound` que reste no caminho com diretório passa a
  significar inequivocamente "cargo ausente" — a colisão da D5 some por
  construção.
- O caminho `--pacote` (`current_dir = None`) fica inalterado.

Detalhe de implementação a decidir na Fase 1: onde a checagem mora — dentro de
`invocar_metadata` (quando `current_dir.is_some()`), ou em
`detectar_pacote_e_alvo_por_diretorio` antes de chamar `invocar_metadata`. A
segunda mantém `invocar_metadata` como "só roda o subprocesso"; a primeira
concentra a guarda perto do uso do `current_dir`. Decisão do gerador conforme
a leitura.

---

## Critérios de Verificação

```
Dado um diretório inexistente passado à porta de detecção por diretório
Quando a detecção é chamada
Então erro DiretorioInexistente com o caminho (NÃO BinarioNaoEncontrado)

Dado um diretório que existe mas é workspace puro (sem pacote casando)
Quando a detecção é chamada
Então PacoteNoDiretorioNaoEncontrado (não-regressão — causa diferente)

Dado um diretório válido com pacote
Quando a detecção é chamada
Então detecção normal do alvo (não-regressão)

Dado cargo ausente do PATH
Quando qualquer porta é chamada
Então BinarioNaoEncontrado (mapeamento inalterado)
```

- O teste `diretorio_inexistente_da_erro_de_deteccao_alvo` passa a **afirmar a
  variante corrigida** (`DiretorioInexistente`), em vez de documentar a
  colisão. Como a checagem curto-circuita antes do spawn, esse teste deixa de
  depender de `cargo` no PATH — vira unidade determinística (não `#[ignore]`).
- `display_cobre_todas_as_variantes_de_erro_metadata` ganha a variante nova.
- Todos os demais testes verdes; contagem estável (substituição de afirmação,
  não teste novo de função).

---

## Resultado esperado

- Diretório inexistente diagnostica a causa real; a clareza que o laudo 0022
  tinha está restaurada, agora sobre o caminho via metadata.
- Colisão `NotFound` da D5 resolvida: no caminho com diretório, `NotFound`
  passa a ser só "cargo ausente".
- Sem mudança de assinatura, sem subprocesso novo.
- **Laudo** registrando o fechamento da D5 do laudo 0023 e a checagem escolhida.

---

## O que NÃO entra

- **Filtro de stdlib e ranking**: prompts próprios.
- **Remoção da E2**: quarentena.
- **Mudança no fork, no `lente_core`, ou de assinatura pública**: nenhuma.
- **Refatorar outras variantes de `ErroMetadata`**: só a adição da nova.

---

## Observação metodológica

A D5 era **dívida de diagnóstico, não otimização**: o laudo 0023 a enquadrou
como "refinar custa código por um caso que não bloqueia uso normal", mas o
caso já tinha mensagem certa no 0022 e passou a ter mensagem enganosa. O
princípio é que o erro aponte a causa real; colisões de `NotFound` (cargo
ausente vs. diretório inexistente) resolvem-se distinguindo a causa **barata e
cedo** — checar o diretório antes do spawn, em vez de tentar inferir depois.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Diretório inexistente passa a diagnosticar a causa real (`ErroMetadata::DiretorioInexistente`) em vez de colapsar em "cargo ausente"; fecha a D5 do laudo 0023; sem subprocesso novo nem mudança de assinatura. | `03_infra/src/metadata.rs`, `03_infra/src/lib.rs`, `00_nucleo/lessons/0024-l3-diretorio-inexistente-diagnostico.md` |
