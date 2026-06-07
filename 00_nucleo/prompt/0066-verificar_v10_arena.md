# Prompt: verificar o V10 (vazamento de quarentena) após a exclusão da Arena

**Camada**: verificação (fonte do `tekt-linter` + prova empírica no `tekt-cargo-dsm`).
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0066-verificar_v10_arena.md`)
**Pré-requisito**: 0065 (a Arena `lab/` saiu do `[layers]` e foi para `[excluded]`
para o projeto chegar a V1 = 0).
**Objetivo**: determinar, com **leitura da fonte + prova empírica que concordem**, se
o **V10 (QuarantineLeak)** ainda detecta um vazamento **treliça→lab** depois de a
Arena ter saído do `[layers]` e ido para `[excluded]`. O `V10 = 0` na tabela do 0065
pode ser **"sem vazamento"** ou **"V10 sem alvo"** — separar os dois. **Sem mudar o
repo** — só medir, para decidir o tratamento da Arena com dado.

Repos: linter em `/home/dikluwe/Documentos/Antigravity/tekt-linter`; projeto
`tekt-cargo-dsm`.

---

## Por que (o ponto exato)

O 0061 mediu que `[excluded_files]`/`[excluded]` é **exclusão total** (poda do walk).
O V10 dispara quando código da treliça importa do `lab` — para isso o linter precisa
**saber quais caminhos são `lab`**. Se essa identificação vinha do `[layers]` (agora
removido para a Arena), o V10 pode ter ficado **sem alvo**: ele ainda examina os
arquivos da treliça (que estão no walk), mas talvez não classifique mais um import
como "do lab". Antes de declarar a arquitetura limpa, é preciso saber se o V10 vê ou
não vê.

---

## O que fazer

1. **Fonte do linter** — achar a implementação do **V10**. Responder, com o código:
   - **Como o V10 identifica a quarentena/`lab`?** Via `[layers]`, `[module_layers]`,
     chave dedicada, ou heurística de path?
   - **Como o V10 detecta um leak?** Que forma de referência ele procura (um `use`
     resolvível ao lab, uma dependência de Cargo, um `mod` por path, …)?
   - **Como o `[excluded]` (poda por diretório) interage** com isso — a Arena fora do
     `[layers]` e podada do walk **deixa o V10 com alvo** ou não?
2. **Prova empírica (controle)** — em **worktrees descartáveis** (removidos ao fim):
   injetar a **mesma** violação **treliça→lab** (na forma que o V10 detecta, conforme
   o passo 1 — p.ex. um arquivo da treliça referenciando o lab) e rodar
   `crystalline-lint --checks v10 .` sob **duas** configs:
   - **(a)** a config **atual** — Arena em `[excluded]`, fora do `[layers]`.
   - **(b)** um **controle** — Arena **de volta** no `[layers]` (o setup de quarentena
     pré-0065).
   Registrar se o V10 **dispara** em cada uma:
   - dispara em **(b)** mas **não** em (a) → a exclusão da Arena **desligou** o V10.
   - dispara em **ambas** → a exclusão é **segura** para o V10.
   - **não** dispara em nenhuma → a injeção **não** é um leak válido (a prova não
     vale; corrigir a forma da injeção conforme o passo 1, **não** concluir).
3. **Reportar** — como o V10 acha a quarentena; se a config atual o deixa disparar;
   e, **se não**, a **config mínima** que mantém a Arena fora da linhagem (V1) **e**
   o V10 ativo. **Sem mudar o repo real.**

---

## O que NÃO fazer

- Mudar código ou config do **repo real** — do linter ou do projeto. **Só medir.**
  (Injeções e configs de teste vivem em **worktrees descartáveis**.)
- **Decidir** o tratamento da Arena — isto é só a medição; a decisão (se for preciso)
  vem depois, com o dado.
- Confundir **"não dispara porque não há vazamento"** com **"não dispara porque o V10
  perdeu o alvo"** — é exatamente para isso o controle **(a) vs (b)**.
- Concluir com uma injeção que **não** seja um leak válido — se não dispara nem em
  (b), a prova está errada, não o V10.

---

## Critérios de Verificação

```
Dado a fonte do V10
Então está identificado como ele acha a quarentena (layers/dedicada/path) e como
detecta um leak, e se a Arena fora do [layers]+[excluded] o deixa com alvo

Dado a prova: a mesma violação treliça→lab sob (a) atual e (b) Arena no [layers]
Então está registrado se o V10 dispara em cada — e isso concorda com a fonte; a
injeção é um leak válido (dispara ao menos em (b))

Dado o caso de o V10 não disparar na config atual
Então há a config mínima que recupera o V10 sem reintroduzir V1

Dado o repo real
Então intocado (tudo em worktrees descartáveis)
```

---

## Resultado esperado

- **Laudo** dizendo, sem ambiguidade: o V10 identifica a quarentena via **{layers |
  chave dedicada | path | …}**; com a config **atual** (Arena excluída) o V10
  **{ainda dispara | não dispara}** num leak treliça→lab; fonte e prova **concordam**;
  a injeção é um leak válido (disparou ao menos em (b)).
- Com isso a decisão fica clara:
  - V10 **ainda dispara** → a exclusão da Arena é **segura**; o `V10 = 0` do 0065 é
    "sem vazamento" de verdade → seguir ao teste do `consulta` (encerrar a migração).
  - V10 **não dispara** → a Arena precisa de outro tratamento (config mínima que a
    mantenha conhecida pela checagem de quarentena sem ganhar cabeçalhos/V1) →
    decisão informada, num prompt à parte.

---

## Cuidados

- **Prova em worktrees descartáveis** — o repo real não muda; nem config nem código.
- **O controle (a) vs (b) é o que dá sentido ao silêncio** — sem a config (b), um
  "sem vazamento" e um "V10 sem alvo" parecem iguais (ambos V10 = 0).
- **A injeção tem de ser um leak que o V10 detecta** (forma do passo 1) — senão a
  prova mede a injeção, não o V10; o disparo em (b) é o atestado de validade.
- **Leitura + prova devem concordar** (bite-proof). Se divergirem, **reportar a
  divergência** — não escolher uma.
- Isto fecha a última dúvida da migração: se o V10 está vivo, a arquitetura está de
  fato limpa (fora o V12 intencional e o V2 do `consulta`).

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Verificação (sem mudança) do **V10 (QuarantineLeak)** após o 0065 ter tirado a Arena (`lab/`) do `[layers]` e a posto em `[excluded]` para o projeto chegar a V1 = 0 — risco de o V10 ter ficado **sem alvo** (o `V10 = 0` seria "não roda", não "sem vazamento"), análogo ao achado do 0061 (exclusão é total). Passos: (1) ler a fonte do V10 — como identifica a quarentena (`[layers]`/chave/path) e como detecta um leak — e se a Arena fora do `[layers]`+`[excluded]` o deixa com alvo; (2) prova em worktrees descartáveis: a **mesma** violação treliça→lab (na forma que o V10 detecta) sob **(a)** config atual (Arena excluída) e **(b)** controle (Arena no `[layers]`), rodando `--checks v10`, para distinguir "sem vazamento" de "V10 sem alvo" (disparo em (b) atesta que a injeção é um leak válido); (3) reportar o escopo + a config mínima que recupera o V10 sem reintroduzir V1, se necessário. Repo real intocado. Decisão do tratamento da Arena (se preciso) fica para depois, com o dado. | (verificação — nenhum arquivo do repo alterado; laudo em `00_nucleo/lessons/0066-verificar_v10_arena.md`) |
