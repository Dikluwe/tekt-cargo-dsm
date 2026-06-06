# Prompt: `ler_diff` (L3) + `mapear_diff` (L1) — a entrada e o núcleo do modo `--diff`

**Camada**: L3 — Infra (`lente_infra`, ler o diff) + L1 — Núcleo (`lente_core`, o
mapeamento)
**Criado em**: 2026-06-05
**Estado**: `PROPOSTO`
**Decisões de origem**: laudo 0038 (mapear linhas→nós por `position` funciona para
rastreados; untracked é cego em `git diff HEAD`); laudo 0043 (validação do
untracked: corte **ligado/solto/não-fonte** via as `position.file` do grafo; hunk
sintético "tudo adicionado" para untracked; os 4 itens de apresentação para o
modo `--diff`).
**Pré-requisito de compilação**: laudo 0037 (`No.position` — o mapeamento casa por
posição). O grafo de workspace (0045) é usado só na orquestração (0047), não aqui.
**Arquivos afetados**: `03_infra/src/` (módulo de diff), `01_core/src/` (módulo de
mapeamento), testes de ambos.

---

## Contexto

O motor (grafo de workspace) está de pé. O modo `--diff` lê um diff e o mapeia ao
grafo, para mostrar o que a mudança toca. Este prompt entrega a **entrada** (ler o
diff: git + untracked, L3) e o **núcleo** (mapear linhas→nós + o censo do
untracked, L1). A **orquestração** (L4) e a **CLI + formatação** (L2) vêm no 0047.
O fluxo inteiro já foi validado na Arena (0038/0043).

---

## Restrições estruturais

- **L3 — admite I/O.** `ler_diff` roda `git` por subprocesso. O invariante "dois
  subprocessos do **cargo**" é sobre o cargo; o `git` é outra ferramenta — os
  subprocessos de git são distintos e não o violam. Por higiene, **uma primitiva
  única de git** (como o fork tem uma única para o cargo, laudo 0018):
  `invocar_git(args, dir)`.
- **L1 — `mapear_diff` é pura.** Só stdlib, **sem deps novas**, sem I/O.
  `cargo tree -p lente_core` continua só o crate.
- **Aditivo**: funções novas; nada existente muda.

---

## Parte 1 — L3: `ler_diff` (`lente_infra`)

### Tipos

```rust
pub struct DiffEstruturado { pub arquivos: Vec<ArquivoDiff> }
pub struct ArquivoDiff {
    pub caminho: PathBuf,            // normalizado p/ casar com position.file (absoluto)
    pub origem: OrigemArquivo,
    pub linhas_alteradas: Vec<FaixaLinhas>,  // linhas do lado NOVO (adicionadas/modificadas)
}
pub enum OrigemArquivo { Rastreado, NaoRastreado }
pub struct FaixaLinhas { pub inicio: u32, pub fim: u32 }  // 1-based, inclusiva

pub fn ler_diff(raiz: &Path) -> Result<DiffEstruturado, ErroDiff>
```

### Comportamento

1. **Rastreados**: `git diff HEAD` (pega staged + unstaged vs o último commit).
   **Parsear o diff unificado** em faixas de linha do **lado novo** (os cabeçalhos
   `@@ -a,b +c,d @@` dão a faixa nova; as `+linhas` são as alteradas). Extrair
   `parse_diff(&str) -> Vec<ArquivoDiff>` como função **pura e testável** (sem
   rodar git).
2. **Untracked**: `git ls-files --others --exclude-standard` (respeita
   `.gitignore`). Para cada arquivo, **sintetizar** um hunk "tudo adicionado"
   (`FaixaLinhas { inicio: 1, fim: n_linhas }`), lendo o arquivo do disco —
   `origem = NaoRastreado`.
3. **Normalizar os caminhos** para casar com `No.position.file` (absoluto). A
   reconciliação relativo↔absoluto é a **mesma** que o laudo 0038 fez (confirmou o
   casamento de rastreados) — reusar a abordagem; é o ponto que faz o mapeamento
   funcionar.
4. **Limitação — remoções não mapeiam**: um diff só de `-linhas` (deleção) não tem
   linha no lado novo, e o nó deletado já não está no grafo atual. Documentar; **não
   tratar** aqui (mapear contra o grafo pós-mudança vê adições/modificações, não
   deleções).

`ErroDiff` novo: `Git(...)` (git falhou), `Parse(String)` (diff malformado),
`Io(std::io::Error)` (ler untracked). `Display` + `Error`.

---

## Parte 2 — L1: `mapear_diff` (`lente_core`)

### Tipos

```rust
pub struct NoTocado { pub id: usize, pub path: Path }
pub struct MapeamentoDiff {
    pub tocados: Vec<NoTocado>,   // nós cuja position cruza uma faixa alterada (rastreado + untracked ligado)
    pub ligados: Vec<PathBuf>,    // untracked QUE ESTÁ no grafo (compilado)
    pub soltos: Vec<PathBuf>,     // untracked .rs em dir de membro mas NÃO no grafo (não compilado)
    pub nao_fonte: Vec<PathBuf>,  // untracked fora de qualquer membro (docs, lab, não-.rs)
}

pub fn mapear_diff(diff: &DiffEstruturado, grafo: &Grafo, membros_dirs: &[PathBuf])
    -> MapeamentoDiff
```

### Comportamento

- **Tocados**: para cada `ArquivoDiff`, achar os nós cujo `position.file` é o
  arquivo **e** cuja faixa `[start_line, end_line]` **cruza** alguma
  `FaixaLinhas`. Isso pega o item específico **e** o módulo-arquivo que o contém
  (a `position` do módulo-arquivo abrange o arquivo) — coerente com o laudo 0043
  (o arquivo novo lá tocou o módulo + as 2 `fn`). Devolver todos os que cruzam.
- **Censo do untracked** (os 3 baldes do 0043):
  - `ligados`: o caminho do untracked está no conjunto de `position.file` do grafo
    → compilado (os nós dele aparecem em `tocados` via o hunk sintético).
  - `soltos`: untracked `.rs` **dentro de um `membros_dirs`** mas **não** no grafo
    → presente, não compilado.
  - `nao_fonte`: untracked fora de qualquer `membros_dirs` (ou não-`.rs`) → externo
    legítimo.
- **Puro**: só `diff` + `grafo` + `membros_dirs` (dados). Determinístico (ordenar
  as saídas).

---

## O que NÃO muda

- O grafo de workspace (0045), o cache (0044), a extração — usados como estão (a
  orquestração que os liga é o 0047).
- As funções existentes do `lente_core`/`lente_infra` — `mapear_diff`/`ler_diff`
  são aditivas.

---

## Critérios de Verificação

```
# parse_diff (L3, puro — sem git)
Dado um diff unificado de exemplo (um arquivo, um hunk com +linhas)
Quando parse_diff
Então o ArquivoDiff tem as faixas do lado NOVO corretas
E um hunk só de -linhas (deleção) não gera faixa nova (limitação documentada)

# ler_diff (L3, E2E — requer git) — #[ignore]
Dado um repo temporário com uma mudança em arquivo rastreado e um arquivo novo
não rastreado
Quando ler_diff(raiz)
Então DiffEstruturado tem a faixa do rastreado E o hunk "tudo adicionado" do
untracked; caminhos normalizados (absolutos)

# mapear_diff (L1, puro — sem git, sem fork)
Dado um grafo com um nó "A::foo" em position {file: F, linhas 10..20} e um diff
com F alterado nas linhas 12..14
Quando mapear_diff
Então "A::foo" está em tocados (a faixa cruza a position)
E o módulo-arquivo de F (se sua position abrange) também está em tocados

Dado um untracked cujo caminho ESTÁ nas position.file do grafo
Então está em ligados; seus nós estão em tocados

Dado um untracked .rs dentro de um membros_dirs mas NÃO no grafo
Então está em soltos (presente, não compilado)

Dado um untracked fora de qualquer membros_dirs
Então está em nao_fonte

Dado um caminho relativo do diff e a position absoluta do grafo
Então casam (reconciliação do laudo 0038)

Dado o mesmo diff/grafo
Quando mapear_diff duas vezes
Então MapeamentoDiff igual (determinístico)

Dado o código todo
Então cargo tree -p lente_core só o crate (mapear_diff é pura)
```

Casos a cobrir: `parse_diff` (faixas do lado novo; deleção não gera faixa);
`ler_diff` (`#[ignore]`, repo temporário: rastreado + untracked, normalização);
`mapear_diff` (tocado por cruzamento; módulo-arquivo junto; os 3 baldes do censo;
reconciliação de caminho; determinismo). Mais a não-regressão da suíte.

---

## Resultado esperado

- `ler_diff` (L3) — git diff HEAD + untracked + `parse_diff` + síntese + normalização;
  `invocar_git` primitiva única; `DiffEstruturado`/`ArquivoDiff`/`OrigemArquivo`/
  `FaixaLinhas`; `ErroDiff`.
- `mapear_diff` (L1, pura) — `tocados` por cruzamento de posição + o censo
  `ligados`/`soltos`/`nao_fonte`; `MapeamentoDiff`/`NoTocado`.
- **Pureza L1**: `cargo tree -p lente_core` só o crate.
- Testes: `parse_diff` (unit), `ler_diff` (`#[ignore]` com git), `mapear_diff`
  (unit, sem fork).
- **Laudo** em `00_nucleo/lessons/0046-…`:
  - A abordagem de reconciliação de caminho (relativo do diff ↔ absoluto da
    position) — a do 0038.
  - A limitação de deleção (registrada, não tratada).
  - O censo (3 baldes) confirmado com casos.
  - A primitiva única de git (e que os subprocessos de git são distintos do
    invariante do cargo).
  - Contagem da suíte (era 242 verdes + 26 ignored no laudo 0045).

---

## Cuidados

- **Reconciliação de caminho é o pulo do gato.** Os caminhos do diff são relativos
  ao repo; as `position.file` são absolutas (0037). Têm de casar — reusar a
  abordagem do 0038 (que confirmou rastreados). Um teste de `mapear_diff` com
  caminho relativo↔absoluto é a guarda.
- **`git` ≠ `cargo`.** Os subprocessos de git são distintos do invariante "dois do
  cargo". Uma primitiva única de git por higiene.
- **Deleção não mapeia.** Diff só de `-linhas`: o nó já não está no grafo
  pós-mudança. Documentar; não inventar.
- **O censo precisa dos dirs de membro.** `mapear_diff` recebe `membros_dirs`
  (vem do `enumerar_membros` do 0044, fornecido pela orquestração do 0047) — sem
  isso não dá para separar `solto` (membro não compilado) de `nao_fonte` (fora de
  membro).
- **0037 é o pré-requisito de compilação** (o mapeamento lê `No.position`) — na
  ordem de pouso, 0037 precede este.
- **Determinismo**: ordenar `tocados`/`ligados`/`soltos`/`nao_fonte`. **Pureza
  L1**: `mapear_diff` só stdlib.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | Entrada + núcleo do modo `--diff`. **L3**: `ler_diff` (`lente_infra`) roda `git diff HEAD` (rastreados) + `git ls-files --others --exclude-standard` (untracked, hunk sintético "tudo adicionado") via primitiva única `invocar_git`; `parse_diff` puro extrai as faixas do lado novo do diff unificado; caminhos normalizados para casar com `No.position.file` (abordagem do 0038); deleção não mapeia (documentado). `DiffEstruturado`/`ArquivoDiff`/`OrigemArquivo`/`FaixaLinhas`/`ErroDiff`. **L1**: `mapear_diff` (`lente_core`, pura) devolve `tocados` (nós cuja `position` cruza uma faixa alterada — item + módulo-arquivo) e o censo do untracked em 3 baldes (`ligados` = no grafo / `soltos` = `.rs` em membro fora do grafo / `nao_fonte` = fora de membro), do laudo 0043; `MapeamentoDiff`/`NoTocado`. Pureza L1 (só stdlib, sem dep). Aditivo. Orquestração (L4) + CLI/formatação (L2) ficam para o 0047. Pré-req de compilação: 0037 (`No.position`). Suíte era 242+26. | `03_infra/src/{diff (novo),lib.rs}`, `01_core/src/{mapeamento (novo),lib.rs}`, `00_nucleo/lessons/0046-...` |
