# Prompt: verificar a semântica de `[excluded_files]` (escopo das checagens)

**Camada**: verificação (fonte do `tekt-linter` + um arquivo do `tekt-cargo-dsm`).
**Criado em**: 2026-06-07
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0061-verificar_excluded_files.md`)
**Pré-requisito**: 0060 (L1 migrado; `vizinhanca.rs` posto em `[excluded_files]`).
**Objetivo**: determinar, com **leitura da fonte + prova empírica que concordem**,
**quais** checks (V1–V14) o `[excluded_files]` suspende — em especial **se suspende
a pureza (V4 I/O no núcleo, V13 estado mutável) além da linhagem (V1, V7)** — e
confirmar se o `vizinhanca.rs` é de fato puro. **Sem mudar código nem config** — só
medir, para decidir a regra de arquivos internos com dado.

Repos: linter em `/home/dikluwe/Documentos/Antigravity/tekt-linter`; projeto
`tekt-cargo-dsm`.

---

## Por que (o ponto exato)

O 0060 pôs em `[excluded_files]` três coisas de naturezas diferentes: um teste
(`e2e_lente_core.rs`), um arquivo de quarentena que sai (`fontes.rs`, E2), e um
arquivo de **lógica interna que fica** (`investiga/src/vizinhanca.rs`, `pub(crate)`).
Se `[excluded_files]` remove o arquivo de **todas** as checagens, o `vizinhanca.rs`
— lógica L1 que permanece — fica **sem guarda de pureza**. Se remove **só** as de
linhagem, a regra do 0060 está correta. Antes do L3 (`lente_infra`, muitos arquivos
internos), é preciso saber qual dos dois.

---

## O que fazer

1. **Fonte do linter** — localizar onde `[excluded_files]` é **lido** e **aplicado**.
   Determinar se a exclusão age no **walk de arquivos** (remove o arquivo de **todas**
   as checagens) ou **por-check** (só alguns checks consultam a lista). **Listar
   exatamente quais checks (V1–V14) honram a exclusão.**
2. **Prova empírica (controle)** — numa **cópia descartável** do projeto: pegar um
   arquivo que **está** em `[excluded_files]` e injetar nele (a) uma violação clara
   de **V4** (ex.: `std::fs::read`/`std::io`) e (b) uma de **V13** (ex.: `static mut`
   / estado mutável global). Rodar o linter. **Registrar se V4/V13 disparam ou ficam
   em silêncio.** A cópia é descartada ao fim — o repo real não muda.
3. **No projeto** — ler `01_core/investiga/src/vizinhanca.rs` e confirmar se é **de
   fato puro**: sem `std::fs`/`std::io`/`std::net`, sem `static mut`/estado mutável
   global. Registrar o veredito.
4. **Reportar** — o escopo do `[excluded_files]` (quais checks suspende, fonte +
   prova concordando) + o veredito sobre o `vizinhanca.rs`. **Sem mudar nada.**

---

## O que NÃO fazer

- Mudar código ou config — do linter **ou** do projeto. **Só medir.** (A injeção do
  passo 2 é numa **cópia descartável**, jogada fora ao fim.)
- **Decidir** a regra dos internos — isto é só a medição; a decisão vem depois, com
  o dado em mãos.
- Confundir **"não dispara porque o arquivo é puro"** com **"não dispara porque foi
  excluído"** — é exatamente por isso que o passo 2 **injeta** a violação: força o
  gatilho; se mesmo assim fica em silêncio, é a **exclusão** que silencia, não a
  pureza.

---

## Critérios de Verificação

```
Dado a fonte do linter
Então a lista exata dos checks (V1–V14) que [excluded_files] suspende está
identificada, com o ponto de aplicação (walk vs por-check)

Dado a prova empírica numa cópia descartável (violação V4 e V13 injetada num
arquivo excluído)
Então está registrado se V4/V13 disparam — e isso concorda com a leitura da fonte

Dado o vizinhanca.rs
Então há veredito: puro (sem I/O, sem estado mutável global) ou não

Dado o repo real
Então intocado (a injeção foi em cópia descartável)
```

---

## Resultado esperado

- **Laudo** dizendo, sem ambiguidade: `[excluded_files]` suspende **{só linhagem
  V1/V7 | linhagem + pureza V4/V13 | outro conjunto — qual}**; a prova empírica
  **concorda** com a leitura da fonte; o `vizinhanca.rs` é **{puro | impuro}**.
- Com isso a decisão fica clara:
  - se suspende **só linhagem** → a regra do 0060 está **correta** (internos sem
    contrato público não precisam de prompt e seguem com pureza checada) → segue o L2.
  - se suspende **pureza** → há uma decisão de tratamento dos internos (cabeçalho
    leve ligado ao prompt do crate / ajuste no linter / aceitar e documentar),
    **informada** pela pureza real do `vizinhanca.rs`.

---

## Cuidados

- **Prova em cópia descartável** — o repo real não muda; nada de config nem código.
- **Leitura + prova devem concordar** (bite-proof). Se divergirem, **reportar a
  divergência** — não escolher uma das duas.
- O **controle** (injeção deliberada) é o que dá sentido ao silêncio: sem ele, um
  arquivo puro e um arquivo excluído parecem iguais (ambos sem achado).
- Esta é a medição que destrava o tratamento dos **muitos** internos do L3 — fazê-la
  agora, no caso pequeno, evita repeti-la em escala.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-07 | Verificação (sem mudança) do escopo de `[excluded_files]` no `crystalline-lint`, motivada pelo 0060 ter excluído um arquivo de lógica interna que fica (`investiga/src/vizinhanca.rs`, `pub(crate)`) junto de agregadores triviais, um teste e um arquivo de quarentena. Passos: (1) ler a fonte do linter e identificar onde `[excluded_files]` é aplicado — walk (todas as checagens) vs por-check — listando quais V1–V14 são suspensos; (2) prova empírica em cópia descartável: injetar V4 (`std::fs`/`std::io`) e V13 (`static mut`) num arquivo excluído e ver se disparam (controle que distingue silêncio-por-pureza de silêncio-por-exclusão); (3) ler o `vizinhanca.rs` e confirmar pureza real. Reporta o escopo do `[excluded_files]` + o veredito do `vizinhanca.rs`. Sem alterar repo (injeção em cópia). Decisão da regra dos internos fica para depois, com o dado — antes do L3, que multiplica arquivos internos. | (verificação — nenhum arquivo do repo alterado; laudo em `00_nucleo/lessons/0061-verificar_excluded_files.md`) |
