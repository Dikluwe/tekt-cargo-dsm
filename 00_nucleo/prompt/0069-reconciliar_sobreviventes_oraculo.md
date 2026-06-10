# Prompt: reconciliar os 38 sobreviventes e separar inerte de fora-do-oráculo

**Camada**: verificação e documentação (laudo) — alvo: o repo do **linter**
(clone canônico, o que tem o conserto do 0052). Só vira código se a
reconciliação achar sobrevivente que muda veredito (aí, fixture).
**Criado em**: 2026-06-09
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0069-reconciliar_sobreviventes_oraculo.md`)
**Pré-requisito**: laudo 0055 (fechamento dos ~18 de extração; estado: 29+8
fixtures, self-lint = 0, suíte verde fora do `blanket_impl` pré-existente).
**Objetivo**: fechar honestamente o laudo 0055 — (1) reconciliar a itemização
dos sobreviventes com o total da ferramenta (2 não-individualizados), e
(2) separar, na classificação, **equivalente-inerte** de **fora-do-oráculo**,
reescrevendo a alegação de completude no termo exato do que foi provado.

> Nota de numeração: 0069 segue a sequência do projeto (último: 0068). Se o
> repo do linter mantém sequência própria de `00_nucleo/`, registrar no laudo
> o número local usado e a correspondência com este.

---

## Contexto — duas coisas a fechar no 0055

O 0055 reportou **38 sobreviventes** no `rs_parser.rs` (total vindo da
ferramenta, confiável: 178 = 127 mortos + 13 inviáveis + 38 sobreviventes) e
marcou "0 não-documentados". Dois problemas impedem tratar isso como fechado:

1. **A itemização não soma 38.** A tabela final lista
   17 + 5 + 5 + 4 + 2 + 1 + 2 = **36**. Faltam **2 sobreviventes**
   individualizados. O critério "0 não-documentados" não está demonstrado
   pela tabela — ou foram 17 mortos e não 15, ou há 2 sem classificar.

2. **"Equivalente" está cobrindo duas naturezas diferentes.** Dos 38, só
   **8** são equivalentes no sentido forte — mutá-los não muda comportamento
   observável nenhum:
   - `parse_layer_tag` (5): produz `PromptHeader.layer`, que nenhuma regra lê.
   - `collect_imports:253` (1): só altera `ImportKind`, que nenhuma regra lê.
   - `collect_type_param_names:786/788` (2): arms de grammar antiga, código
     morto sob `tree-sitter-rust 0.23`.

   Os outros **28** — `find_first_error_pos` (17) e a aritmética de
   linha:coluna (11) — **mudam comportamento observável**: a posição
   reportada de uma violação e a posição de um erro de sintaxe (V0/`PARSE`).
   O harness não os pega porque afirma **IDs + contagem, não posição**. Isso
   não os torna inertes; torna-os **fora do oráculo** — comportamento real
   que o corpo, por desenho, escolheu não testar. Chamá-los de "equivalentes"
   diz mais do que se provou.

   É a mesma lição da anamnese recorrendo dentro do próprio corpo:
   "0 sobreviventes" é verdadeiro contra o oráculo de **veredito**, que é
   cego à posição. O ganho real, a registrar nesses termos: o corpo é
   **completo para vereditos**; não é completo para a saída inteira.

---

## Pré-condição

Confirmar antes de mexer:
- Clone canônico com o conserto do 0052 (`crate_registry.rs`).
- Estado do 0055 presente e verde: fixtures + harness `tests/fixtures.rs`,
  self-lint = 0, suíte verde fora do `blanket_impl` pré-existente.

Se a pré-condição falhar, **parar e relatar** — não trabalhar sobre outra
linha do código.

---

## Tarefa

### 1. Lista crua e reconciliação um-a-um

Re-rodar a mutação no escopo e ler a lista autoritativa de sobreviventes do
diretório de saída — **não confiar na tabela do laudo 0055**:

```
cargo mutants -j 4 --file '03_infra/rs_parser.rs'
cat mutants.out/missed.txt      # os sobreviventes, um por linha (= "MISSED")
wc -l mutants.out/caught.txt mutants.out/missed.txt mutants.out/unviable.txt
```

Bater `missed.txt` contra a tabela do 0055. Para **cada** sobrevivente (os
38, ou o número que a ferramenta der agora), uma de três decisões, explícita:

- **Muda veredito** (qual regra dispara / contagem de IDs muda) → não é
  equivalente nem fora-do-oráculo: **matar com fixture** (bite-proof;
  afirmar IDs + contagem), re-rodar, confirmar morto.
- **Fora do oráculo**: muda só a posição (linha:coluna de violação) ou a
  posição de erro de sintaxe V0/`PARSE`. Registrar com linha, função e o que
  exatamente muda na saída.
- **Inerte**: saída que nenhuma regra lê, ou código inalcançável sob a
  grammar pinada. Registrar com a **prova** (o `grep` que mostra que ninguém
  lê o valor, ou a nota da versão da grammar).

Os **2 que faltam na itemização** têm de cair numa dessas três, nomeados. Ao
fim, a soma das três categorias = o total da ferramenta, **exatamente** —
sem resíduo, sem "adiado".

### 2. Reenquadrar a alegação de completude

Atualizar o laudo 0055 (ou registrar no laudo deste prompt, com ponteiro de
volta) com a separação:

- **Inertes (equivalência real)**: a lista dos 8 (ou o número confirmado),
  cada um com linha e prova.
- **Fora-do-oráculo**: a lista dos 28 (ou o número confirmado), cada um com
  linha e o comportamento que muda. Registrar a decisão pendente, sem
  resolvê-la aqui: **posição e V0/`PARSE` são um oráculo à parte** — decidir
  depois se vale construí-lo (fixtures que afirmam posição) ou declará-lo
  formalmente fora do contrato do corpo. As duas opções são legítimas; o que
  não é legítimo é o estado atual, em que a fronteira não está declarada.
- **A alegação reescrita**: onde o 0055 diz "0 sobreviventes
  não-documentados" / completo, passar a dizer: **completo para vereditos**
  (regras V1–V14, classificação de import e a extração que alimenta os
  vereditos), com N inertes provados e M fora-do-oráculo declarados.

### 3. Registrar os erros de premissa do prompt 0055

Para o registro causal (são informação, não culpa):

- Os números de linha do prompt vinham da versão **pré-0052** (lidos do
  master público, não do clone canônico).
- A suposição de que `collect_type_param_names` alimentava as type-sigs de
  V6/V12 estava errada — ele alimenta o **V11** (blanket impls). Por isso as
  fixtures genéricas do V12 não mataram nada (os mutantes-alvo eram código
  morto). As fixtures permanecem válidas como categorias do corpo, mas não
  foram carga-útil.
- O que absorveu o erro foi a disciplina do método ("matar ou provar
  equivalente, da fonte, um a um") — registrar isso como lição: prompts que
  raciocinam de fonte não-canônica precisam da verificação contra o clone
  canônico embutida na execução.

### 4. (Opcional, barato) Conferência de ramos

`cargo llvm-cov` sobre regras + extração: confirmar que não há ramo sem
execução que a mutação não tenha tocado ("a mutação subsome cobertura de
ramos" só vale para os ramos que o `cargo-mutants` mutou; um ramo sem
mutante gerado pode ficar morto sem aparecer como sobrevivente). Se rodar,
anexar o resumo ao laudo; se não rodar, registrar que ficou de fora.

---

## O que NÃO fazer

- **Não construir o oráculo de posição** — só declarar a fronteira e a
  decisão pendente. Construí-lo (se for o caso) é prompt próprio.
- **Não mexer nas fixtures existentes** além do estritamente necessário se
  um sobrevivente mudar-veredito exigir fixture nova.
- **Não recolocar whitelists** nem mascarar achados.
- **Não tocar nos três detectores** (contador de `Layer::Unknown`, oráculo
  diferencial, corpus de projetos reais) — prompts seguintes.
- **Não decidir o merge com o master público** — trilha à parte.

---

## Critérios de Verificação

```
Dado missed.txt re-gerado pela ferramenta
Então a itemização do laudo soma exatamente o total da ferramenta — os 2
faltantes nomeados e classificados; 0 sobreviventes sem decisão explícita

Dado cada sobrevivente classificado como inerte
Então há prova anexa (grep de não-leitura ou nota de grammar), com linha

Dado cada sobrevivente classificado como fora-do-oráculo
Então há registro de qual comportamento observável muda (posição de
violação ou de erro V0/PARSE), com linha e função

Dado algum sobrevivente que muda veredito (se existir)
Então fixture bite-proof o mata; mutação re-rodada confirma

Dado o laudo final
Então a alegação é "completo para vereditos", com a fronteira do oráculo de
posição declarada como decisão pendente; a ressalva do 0054 e o "0
não-documentados" do 0055 apontam para este fechamento

Dado o repo
Então self-lint = 0; suíte verde fora do blanket_impl pré-existente; nada
mascarado; nenhuma mudança de código além de fixtures (se houver)
```

---

## Resultado esperado

- Itemização reconciliada: soma exata, 2 faltantes nomeados.
- Classificação tripartida completa: inertes provados / fora-do-oráculo
  declarados / (eventuais) mudam-veredito mortos por fixture.
- Laudo 0055 reenquadrado ("completo para vereditos") com ponteiro para este.
- Erros de premissa do prompt 0055 registrados (linhas pré-0052; V11 vs
  V6/V12).
- **Laudo** em `00_nucleo/lessons/0069-…` (ou número local do repo do
  linter, com correspondência registrada): a lista crua, as três categorias
  com prova, a decisão pendente do oráculo de posição, e o estado do
  programa — fechado este, restam os três detectores contra a linguagem e,
  à parte, o merge com o master.

---

## Fora de escopo (prompts seguintes)

Em ordem: contador de `Layer::Unknown` em alvo real; oráculo diferencial
contra a computação de dependências da própria lente (`tekt-cargo-dsm`);
corpus de projetos reais estruturalmente variados. À parte: decisão de merge
com o `master` público (multi-linguagem + Hash Locking ⊕ conserto do 0052).
E a decisão sobre o oráculo de posição/V0-`PARSE`, que este prompt só
declara.

---

## Disciplina (do repo)

Prompt nucleado antes do código; linhagem nos arquivos novos (se houver
fixture); a mutação é a forma mecânica da prova-de-mordida; nada mascarado;
laudo ao fim.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-09 | Fechamento honesto do 0055: reconciliar a itemização (36 ≠ 38; 2 sobreviventes a nomear) contra a lista crua da ferramenta (`missed.txt`), e separar a classificação em três categorias com prova — inertes (8: `parse_layer_tag` ×5, `collect_imports:253`, arms 786/788 de grammar antiga) / fora-do-oráculo (28: `find_first_error_pos` ×17, aritmética linha:coluna ×11 — mudam posição reportada, que o harness de IDs+contagem não vê) / mudam-veredito (matar com fixture, se existir). Reescreve a alegação de completude para "completo para vereditos" e declara o oráculo de posição/V0-PARSE como decisão pendente, sem construí-lo. Registra os erros de premissa do prompt 0055 (linhas pré-0052; `collect_type_param_names` alimenta V11, não V6/V12). Sem mudança de código além de eventuais fixtures; nada mascarado. | laudo do 0055 (reenquadramento), `00_nucleo/lessons/0069-…` (novo), eventuais fixtures novas |
