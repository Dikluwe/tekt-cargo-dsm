# Prompt: `lente_infra` — extração cacheada (chave completa) + enumeração de membros (L3)

**Camada**: L3 — Infra (`lente_infra`)
**Criado em**: 2026-06-05
**Estado**: `EXECUTADO` (laudo `00_nucleo/lessons/0044-infra_cache_e_membros.md`)
**Decisões de origem**: laudo 0039 (o grafo de workspace precisa extrair todos os
crates); laudo 0040 (cache do JSON cru por crate viabiliza o uso reativo;
**limitação registrada**: a chave não pega `Cargo.toml`); laudo 0043 (a chave por
glob de filesystem aceita a re-extração espúria do arquivo solto — decisão
mantida).
**Pré-requisito**: o desenho do cache foi validado na Arena (0040); este prompt o
nucleia em produção **com a chave completa** (o que faltava no 0040).
**Arquivos afetados**: `03_infra/src/` (módulo novo de cache + enumeração),
`03_infra/Cargo.toml` (talvez +1 dep de hash estável), testes do `lente_infra`,
`.gitignore` (diretório do cache).

---

## Contexto e escopo

O motor da trilha local precisa de um grafo **de workspace** — todos os crates,
resolvido e rápido. Esse grafo se espalha por três camadas: a **união por path**
(L1, `lente_core`), a **extração cacheada + enumeração** (L3, este prompt), e a
**orquestração** (L4, `lente_wiring`, próximo prompt).

**Este prompt é só a fundação de I/O (L3)**: (1) extração por crate com cache de
**chave completa** — fechando a limitação do 0040 — e (2) enumeração dos membros
do workspace. A união (L1) e a orquestração (L4) que montam o grafo vêm no 0045 e
**não** entram aqui.

A **resolução** de colisões não é tocada — a fiação já a faz (correta após 0042);
o 0045 a reusa.

---

## Restrições estruturais

- **L3 — admite I/O** (filesystem, subprocess) e deps externas (já tem
  `serde`/`serde_json`).
- **Invariante dos subprocessos do cargo** (briefing): não proliferar subprocessos
  do `cargo`. A enumeração **lê os `Cargo.toml` direto** (sem `cargo metadata`); a
  versão do toolchain vem do **`rustc`** (não do `cargo`). Confirmar no laudo que
  nenhum subprocesso novo do `cargo` foi adicionado — só o do fork, que já existe.
- **Retrocompatível**: `extrair_grafo`, `desserializar_grafo` e o `fork`
  **não mudam**. A extração cacheada é função **nova**, ao lado.
- **`lente_core` intocado** — isto é L3; a pureza do L1 não é assunto aqui
  (`cargo tree -p lente_core` segue só o crate).

---

## O que adicionar

### 1. Enumeração de membros (I/O, sem subprocesso)

```rust
pub struct MembroWorkspace { pub nome: String, pub dir: PathBuf }

pub fn enumerar_membros(raiz: &Path) -> Result<Vec<MembroWorkspace>, ErroWorkspace>
```

- Lê `raiz/Cargo.toml`, seção `[workspace].members` (lista de **caminhos**, podem
  ter glob tipo `crates/*`).
- Expande globs pelo **filesystem**.
- Para cada diretório-membro, lê seu `Cargo.toml` e extrai `[package].name` (o
  nome do pacote, que é o que o fork recebe — `≠` nome do diretório).
- Membro sem `[package]` (sub-workspace virtual) → pular, registrando.
- **Nenhum subprocesso do cargo** (parse de TOML linha-a-linha ou via dep de
  parse já presente — decisão do gerador, registrar).
- O `lab/` tem `[workspace]` próprio → **não** é membro do workspace principal;
  `enumerar_membros(raiz_principal)` não o lista (confirmar).

### 2. Versão do toolchain (um subprocesso do `rustc`, não `cargo`)

```rust
pub fn versao_toolchain() -> Result<String, ErroWorkspace>
```

- Roda `rustc --version` (ou lê `rust-toolchain.toml` se houver pin — decisão do
  gerador, registrar) e devolve a string. **Uma** chamada por rodada do grafo (o
  L4 consulta uma vez e passa adiante); por isso é parâmetro da extração, não
  consulta interna por membro.

### 3. Extração cacheada com **chave completa**

```rust
pub fn extrair_grafo_cacheado(
    membro: &MembroWorkspace,
    raiz: &Path,
    versao_toolchain: &str,
) -> Result<Grafo, ErroWorkspace>
```

**A chave** (SHA-256, ou hash estável equivalente — a do 0040 era SHA-256;
gerador decide o crate, registrar) sobre, em ordem fixa:

1. os **fontes** do membro — glob `membro.dir/src/**.rs`, ordenado, conteúdo
   concatenado (a decisão do 0043: glob de filesystem, aceitando a re-extração
   espúria do arquivo solto — **não** tentar corrigir aqui);
2. o **`Cargo.toml`** do membro (o que faltava no 0040);
3. o **`Cargo.lock`** do workspace (`raiz/Cargo.lock`) — mudança de dependência
   invalida (conservador: invalida **todos** os membros, aceitável — `lock` muda
   pouco);
4. a **`versao_toolchain`** — troca de toolchain invalida (o fork lê via
   rust-analyzer; versão diferente pode emitir diferente).

**O fluxo**:

- Computa a chave. Diretório do cache: gitignorado (sugestão `raiz/target/lente-cache/`,
  já que `target/` é ignorado; gerador decide e registra).
- Arquivo `chave.json` existe → lê o JSON cru → `desserializar_grafo` → `Grafo`
  (**acerto**, sem fork).
- Não existe → `fork::invocar_fork(&membro.nome)` → grava o JSON cru no cache →
  `desserializar_grafo` → `Grafo` (**erro de cache**, com fork).
- Gravação **atômica** (escrever em temp + rename) para não deixar entrada
  corrompida se o processo morrer no meio.

### 4. Helper de chave isolável (para testar sem fork)

```rust
pub fn chave_cache(membro: &MembroWorkspace, raiz: &Path, versao_toolchain: &str)
    -> Result<String, ErroWorkspace>
```

Permite verificar a invalidação (cada componente muda a chave) sem rodar o fork.

### Tipo de erro

`ErroWorkspace` novo, cobrindo: `Io(std::io::Error)`, `Manifesto(String)` (TOML
malformado / sem `[package]`), `Fork(ErroFork)`, `Adaptador(ErroAdaptador)`,
`Toolchain(String)` (rustc falhou). `impl Display` + `std::error::Error`.

### `.gitignore`

Adicionar o diretório do cache (se não cair sob `target/` já ignorado).

---

## O que NÃO muda

- `extrair_grafo`, `desserializar_grafo`, `fork::invocar_fork`,
  `fork::invocar_em` — intocados (retrocompat).
- A tradução DTO→Grafo — intocada.
- A resolução de colisões — não é assunto deste prompt (fica na fiação, 0045).
- `lente_core` — intocado.

---

## Critérios de Verificação

```
Dado um workspace de fixture com 2+ membros (um com glob nos members)
Quando enumerar_membros(raiz)
Então lista todos os membros com nome (de [package].name) e dir corretos,
globs expandidos

Dado o workspace principal real
Quando enumerar_membros(raiz_principal)
Então NÃO inclui nenhum crate do lab/ (lab tem [workspace] próprio)
(pode ser #[ignore] se depender do layout real)

Dado os mesmos fontes + Cargo.toml + Cargo.lock + versao_toolchain
Quando chave_cache duas vezes
Então a mesma chave

Dado uma mudança em qualquer fonte do membro
Então chave_cache muda

Dado uma mudança no Cargo.toml do membro
Então chave_cache muda

Dado uma mudança no Cargo.lock do workspace
Então chave_cache muda

Dado uma versao_toolchain diferente
Então chave_cache muda

Dado um cache pré-populado com um JSON cru válido para a chave do membro
Quando extrair_grafo_cacheado
Então lê o cache, NÃO roda o fork, devolve o Grafo desserializado
(testável SEM fork — pré-gravar o arquivo de cache)

Dado um cache vazio e o fork disponível
Quando extrair_grafo_cacheado
Então roda o fork, grava o arquivo de cache, devolve o Grafo
(#[ignore] — depende do fork, padrão dos E2E existentes)

Dado o fork disponível
Quando extrair_grafo_cacheado (erro) e depois de novo (acerto)
Então os dois devolvem Grafos iguais (cache transparente) (#[ignore])

Dado o código todo
Então grep por Command::new mostra só o fork (cargo) já existente e o rustc
(versao_toolchain) — nenhum subprocesso novo do cargo
```

Casos a cobrir: enumeração (fixture com glob; exclusão do lab); chave estável e
sua invalidação por cada um dos 4 componentes; acerto de cache **sem** fork
(pré-gravado); erro→fork→grava e transparência (`#[ignore]`); ausência de
subprocesso novo do cargo. Mais a não-regressão do `lente_infra`.

---

## Resultado esperado

- `lente_infra` com `enumerar_membros`, `versao_toolchain`, `chave_cache`,
  `extrair_grafo_cacheado`, `MembroWorkspace`, `ErroWorkspace`. Cache em diretório
  gitignorado, gravação atômica.
- Chave completa (fontes + `Cargo.toml` + `Cargo.lock` + toolchain) — fecha a
  limitação do 0040.
- `extrair_grafo` e o fork **inalterados**.
- Testes: enumeração, chave + invalidação (sem fork), acerto sem fork
  (pré-gravado), erro/transparência (`#[ignore]`), ausência de subprocesso novo do
  cargo.
- **Pureza L1**: `cargo tree -p lente_core` só o crate (não tocado).
- **Laudo** em `00_nucleo/lessons/0044-…`:
  - O diretório do cache escolhido e por quê (gitignorado).
  - A composição da chave e a ordem dos componentes.
  - De onde vem a versão do toolchain (`rustc --version` ou `rust-toolchain.toml`).
  - O método de enumeração (parse direto, **sem `cargo metadata`**) — e a
    confirmação de que nenhum subprocesso novo do cargo entrou.
  - A confirmação de que o `lab/` não é listado.
  - Se precisou de dep nova (hash) e qual.
  - A re-extração espúria do arquivo solto (0043) segue aceita — registrar que
    **não** foi "corrigida" aqui.
  - Não-regressão (contagem da suíte: era 220 verdes + 22 ignored no laudo 0042).

---

## Cuidados

- **Invariante do cargo**: a enumeração **não** roda `cargo` (lê TOML); a versão
  vem do `rustc`. Se o gerador achar que `cargo metadata` é inevitável, **parar e
  registrar** — é decisão que mexe no invariante, não tomar sozinho.
- **Chave completa, ordem fixa**: os 4 componentes em ordem determinística, senão
  a chave varia à toa entre rodadas.
- **`Cargo.lock` invalida todos**: conservador-correto; não tentar invalidação
  fina por dependência (complexidade sem evidência de necessidade).
- **Gravação atômica**: temp + rename, para o cache nunca ter entrada parcial.
- **Concorrência**: o uso é reativo (uma rodada por vez); se duas rodadas
  extraírem o mesmo crate ao mesmo tempo, last-write-wins é aceitável (o conteúdo
  é idêntico para a mesma chave). Registrar como aceito, não resolver.
- **Diretório gitignorado**: o cache **não** vai pro repositório.
- **Retrocompat estrita**: nada do caminho de crate-único (`extrair_grafo`,
  `calcular_raio_de_alvo`) muda — a extração cacheada é aditiva.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-05 | `lente_infra` (L3) ganha a fundação de I/O do grafo de workspace: `enumerar_membros` (parse direto dos `Cargo.toml`, sem subprocesso do cargo; exclui o `lab/`), `versao_toolchain` (via `rustc`), e `extrair_grafo_cacheado` com **chave completa** — SHA dos fontes (glob, decisão 0043) + `Cargo.toml` do membro + `Cargo.lock` do workspace + versão do toolchain — fechando a limitação registrada no laudo 0040 (a chave não pegava `Cargo.toml`). Cache do JSON cru por crate, diretório gitignorado, gravação atômica. `extrair_grafo` e o fork inalterados (aditivo). A união (L1) e a orquestração (L4) que montam o grafo ficam para o 0045. `lente_core` intocado. | `03_infra/src/{lib.rs,módulo de cache}`, `03_infra/Cargo.toml`, `.gitignore`, `00_nucleo/lessons/0044-...` |
