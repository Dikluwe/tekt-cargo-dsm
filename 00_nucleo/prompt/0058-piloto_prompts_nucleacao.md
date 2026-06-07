# Prompt: migração de convenção — PILOTO (travar o padrão num crate pequeno)

**Camada**: transversal (cria `prompts/` + migra os cabeçalhos de UM crate) — no
`tekt-cargo-dsm`.
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0058-piloto_prompts_nucleacao.md`)
**Pré-requisito**: 0057 (refactor V3+V12 completo).
**Decisão fechada**: conjunto **novo** de prompts de **nucleação** (que descreveriam
o código), **por unidade de código**, numa pasta **nova** `prompts/`. **NÃO**
renomear nem mover o `prompt/` atual (pode ler, não mover).
**Objetivo deste prompt**: **piloto** — fazer **um crate L1 pequeno** para **travar
o padrão** (forma do prompt de nucleação, formato do Interface Snapshot que o V6
espera, granularidade da unidade, cabeçalho). Com o padrão aprovado, os próximos
prompts escalam para o resto.

---

## Por que piloto

Escrever um prompt de nucleação (com Interface Snapshot) por unidade de código, na
base inteira, é autoria grande. Se o padrão sair errado, é melhor descobrir num
crate do que em dezenas de arquivos. O piloto fixa o molde; o resto replica.

Efeito colateral bom da decisão: a `prompts/` nova só terá prompts que nucleiam
código existente → **sem órfão (V7 fecha sozinho)**; e o `prompt/` atual (trabalho +
verificação) fica intocado e fora do alcance do linter (que só olha `prompts/`).

---

## O que fazer (no crate piloto)

**Crate piloto**: o **menor L1 auto-contido** — sugestão `lente_ranking` ou
`lente_filtro` (poucos arquivos, interface pequena). Escolher um e registrar qual.

1. **Criar `00_nucleo/prompts/`** (pasta nova). **NÃO** renomear/mover o `prompt/`.
2. Para cada **unidade de código** do crate piloto (arquivo/módulo coerente;
   ignorar `mod.rs` trivial e fixtures), **escrever em `prompts/` um prompt de
   nucleação** que:
   - **descreva o que a unidade faz** (propósito, comportamento, invariantes) a
     ponto de o código ser uma **materialização fiel** — não é cópia do prompt de
     trabalho nem stub;
   - **inclua o Interface Snapshot** (a interface pública real da unidade) no
     formato que o **V6** compara — derivar do código (não inventar);
   - leia o código + o prompt de trabalho correspondente em `prompt/` **como
     referência**, sem mover.
3. **Migrar os cabeçalhos** dos arquivos do crate piloto para:
   ```
   //! Crystalline Lineage
   //! @prompt 00_nucleo/prompts/<unidade>.md
   //! @prompt-hash <placeholder>
   //! @layer L1
   //! @updated AAAA-MM-DD
   ```
   (`@prompt` aponta para o prompt **novo** em `prompts/`; `//!` no topo.)
4. **`crystalline-lint --fix-hashes .`** — preenche os `@prompt-hash` (V5).
5. **Rodar o linter completo** e conferir, **para o crate piloto**:
   - **V1 = 0** nos arquivos dele (têm cabeçalho).
   - **V5 = 0** (hashes).
   - **V6 = 0** (Interface Snapshot presente e casando com a interface real).
   - **Sem órfão V7** dos prompts novos (cada um tem arquivo apontando).
   - O **resto do projeto** segue com V1 alto (ainda não migrado) — **esperado**.
6. **Verificar**: suíte **273 + 28** (só cabeçalhos `//!` + prompts novos, não-código
   — comportamento idêntico); `cargo build` passa.

---

## O padrão que o piloto trava (para escalar depois)

- A **forma do prompt de nucleação** (seções, nível de detalhe).
- O **formato do Interface Snapshot** que o V6 aceita (confirmado contra o linter).
- A **granularidade da unidade** (um prompt por arquivo? por módulo? — o que casa
  com o V6 e o `@prompt` por-arquivo do linter).
- O **cabeçalho** (`@prompt`/`@prompt-hash`/`@layer`/`@updated`).

O laudo deve deixar esse molde explícito.

---

## O que NÃO fazer

- Renomear/mover o `prompt/` — fica intocado; só leitura.
- Migrar o projeto todo — é **piloto** (um crate).
- Cópias do `prompt/` ou stubs — prompts de **nucleação reais**.
- Inventar o Interface Snapshot — derivar da interface pública real (V6).
- Mudar código.

---

## Critérios de Verificação

```
Dado 00_nucleo/prompts/ (nova) e prompt/ (intocado)
Então o prompt/ não foi renomeado nem movido; só lido

Dado o crate piloto
Então cada unidade tem um prompt de nucleação em prompts/ com Interface Snapshot,
e cada arquivo tem o cabeçalho //! Crystalline Lineage apontando para ele

Dado --fix-hashes
Então os @prompt-hash do piloto preenchidos

Dado o linter completo
Então o crate piloto: V1=0, V5=0, V6=0, sem órfão V7; o resto do projeto inalterado

Dado a suíte
Então 273 + 28 — comportamento idêntico (só comentários + prompts novos)
```

---

## Resultado esperado

- A pasta `prompts/` + os prompts de nucleação do crate piloto (com Interface
  Snapshot).
- Os cabeçalhos do piloto migrados; `--fix-hashes` aplicado.
- O linter mostrando o **piloto limpo** (V1/V5/V6 = 0, sem órfão); o **padrão**
  (forma do prompt, formato do snapshot, granularidade, cabeçalho) **explícito**
  para escalar.
- A suíte **273 + 28**.
- **Laudo** em `00_nucleo/lessons/0058-…`: o crate escolhido, os prompts de
  nucleação criados, o molde travado, e o que falta escalar.

---

## Cuidados

- **`prompt/` intocado** — só leitura; não renomear, não mover.
- **O Interface Snapshot tem que casar com a interface real** (V6) — derivar do
  código, conferir contra o que o linter espera (a fonte do linter, já lida, diz o
  formato).
- **Prompts de nucleação reais** (descreveriam o código) — não cópia, não stub.
- **Comportamento idêntico** — só `//!` + prompts; a suíte 273 + 28 é a prova.
- **Crate piloto pequeno e auto-contido** — o objetivo é travar o molde rápido, não
  migrar tudo.
- **Sem órfão por construção** — cada prompt novo nucleia uma unidade que tem
  arquivos; se algum prompt novo ficar sem arquivo apontando, é sinal de granularidade
  errada — reportar.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Piloto da migração de convenção (decisão: conjunto novo de prompts de nucleação por unidade, em pasta nova `prompts/`; **não** renomear/mover o `prompt/`). (Renumerado para 0058 — o 0058 anterior, migração por renomeação, foi descartado antes de executar.) Num crate L1 pequeno (sugestão `lente_ranking`/`lente_filtro`): criada `00_nucleo/prompts/`; escritos prompts de nucleação reais (descreveriam o código, com Interface Snapshot derivado da interface pública real para o V6), lendo código + prompt de trabalho como referência sem mover; migrados os cabeçalhos do crate para `//! Crystalline Lineage / @prompt 00_nucleo/prompts/<unidade> / @prompt-hash (via --fix-hashes) / @layer L1 / @updated`. Linter completo: o crate piloto com **V1=0/V5=0/V6=0, sem órfão V7** (prompts novos só nucleiam unidades com arquivos; `prompt/` fora do alcance do linter); resto do projeto inalterado (V1 alto, ainda não migrado). **Preserva comportamento**: só `//!` + prompts novos; suíte 273 + 28. O laudo trava o **molde** (forma do prompt, formato do snapshot, granularidade, cabeçalho) para escalar aos demais crates nos próximos prompts. | `00_nucleo/prompts/` (nova, prompts do piloto), `<crate piloto>/src/*.rs` (cabeçalhos), `crystalline.toml` (se preciso ajuste para o piloto), `00_nucleo/lessons/0058-...` |
