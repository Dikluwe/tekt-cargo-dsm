# Laudo de Execução — Prompt 0023 (Detecção por `cargo metadata`)

**Camada**: L5 (laudo)
**Data**: 2026-06-01
**Prompt executado**: `00_nucleo/prompt/0023-l3-deteccao-alvo-metadata.md`
**Decisões de origem**: laudo 0022 D1 (heurística por restrição "um só
`Command::new`") + D4 (fragilidade da heurística em crates exóticos) +
pendência "porta `--pacote`" (bin+lib via `fork::invocar_fork` não coberta).
**Estado**: `EXECUTADO` — detecção migrada para `cargo metadata` (fonte
autoritativa); heurística do 0022 removida; porta `--pacote` fechada para
bin+lib via descoberta por nome; **dois** subprocessos do cargo no crate
(um `export-json`, um `metadata`), cada um único; pureza do L1 intacta;
114 verdes + 11 ignored (todos passam).

---

## Fase 1 — Leitura e verificação contra o `cargo metadata` real

### O que o 0022 deixou (rememorado)

| Peça | Estado pré-0023 |
|------|-----------------|
| `invocacao::detectar_alvo` (heurística) | Parser TOML linha-a-linha + `listar_bins_dir` (layout `src/`) |
| `invocacao::descobrir_pacote` | Lia `name` de `[package]` |
| `invocacao::parse_alvos_do_toml` | Coletava `[lib]` + nomes de `[[bin]]` |
| `fork::AlvoFork` (`Lib`/`Bin(String)`) | Enum interno (`pub(crate)`) |
| `ErroAdaptador::{CargoTomlAusente, CargoTomlSemPackage, AlvosAmbiguos}` | Variantes da heurística |
| `fork::invocar_fork(pacote)` | Sem detecção; bin+lib via porta `--pacote` ainda falhava |

### Campos reais do `cargo metadata --no-deps --format-version 1`

Verificado contra o workspace deste projeto E contra fixtures temporários:

```jsonc
{
  "packages": [
    {
      "name": "lente_core",
      "manifest_path": "/.../01_core/Cargo.toml",
      "targets": [
        { "name": "lente_core", "kind": ["lib"], "crate_types": ["lib"] }
      ]
    }
    /* … */
  ],
  "workspace_root": "/...",
  "workspace_members": [/* package-ids */]
}
```

- `targets[].kind` é **lista** (raramente >1, mas a regra precisa iterar).
- `name` do `target` ≠ `name` do `package` em geral (binários custom
  via `[[bin]] name = "..."`); a flag `--bin <X>` espera o nome do target.

### `kind`s relevantes (observados na Fase 1)

| `kind` | Conta como… |
|--------|-------------|
| `lib`, `rlib`, `dylib`, `cdylib`, `staticlib` | biblioteca → `--lib` |
| `proc-macro` | biblioteca (verificado abaixo) |
| `bin` | binário → `--bin <nome>` |
| `example`, `test`, `bench`, `custom-build` | ignorados (não-analisáveis pela lente) |

### Comportamento de `cargo modules --lib` em proc-macro (a parte que não dava para assumir)

Fixture montado em `/tmp` (`[lib] proc-macro = true`), `cargo modules
export-json --sysroot --compact --lib --package pm_meta`: **exit 0, JSON
válido** (`{"crate":"pm_meta","nodes":[...]`). Por isso `proc-macro` está
em `KINDS_LIB` da seleção.

### Descoberta por nome a partir da raiz de um workspace

Fixture com `[workspace] members = ["crateA","crateB"]` em `/tmp`; `cargo
metadata --no-deps` na raiz listou ambos com seus `targets[]` — basta
filtrar `packages` por `name`. **Esse é o mecanismo que fecha a porta
`--pacote`**: `fork::invocar_fork(pacote)` precisa só do nome e do cwd
(via `current_dir = None`, herdando), não do diretório específico.

### Re-confirmação do 0022 (rápido)

`--lib` num pacote só-bin continua falhando ("No library target found");
bin+lib sem flag continua falhando ("Multiple targets present…"). A regra
de escolha segue a mesma do 0022.

### Internas removíveis (grep ANTES da Fase 2)

`grep -rn "AlvosAmbiguos\|CargoTomlAusente\|CargoTomlSemPackage\|descobrir_pacote"`
apontou só para arquivos do próprio `lente_infra`. Nenhum chamador externo
— supressão livre.

---

## Fase 2 — Conserto

### Estrutura final

```
03_infra/src/metadata.rs   (NOVO, ~270 linhas + testes)
  + pub enum ErroMetadata { Binario|Falha|Status|Stdout|Json|
                            PacoteNaoEncontrado|PacoteNoDiretorioNaoEncontrado|
                            AlvosAmbiguos }
  + pub(crate) const KINDS_LIB = [lib, rlib, dylib, cdylib, staticlib, proc-macro]
  + invocar_metadata(current_dir)                  ← único Command::new do metadata
  + selecionar_alvo(&MetadataPackage) -> AlvoFork  ← puro, testável com JSON literal
  + detectar_alvo_por_nome(pacote, current_dir)    ← porta --pacote
  + detectar_pacote_e_alvo_por_diretorio(dir)      ← porta extrair_grafo

03_infra/src/fork.rs
  ~ ErroFork::DeteccaoAlvo(ErroMetadata)           ← variante nova; From<ErroMetadata>
  ~ invocar_fork(pacote) {
       let alvo = metadata::detectar_alvo_por_nome(pacote, None)?;
       invocar_em(pacote, None, Some(&alvo))
    }
  = invocar_em(pacote, current_dir, alvo)          ← único Command::new do FORK

03_infra/src/invocacao.rs   (radicalmente menor — só `invocar` + mapeamento de erro)
  - parse_alvos_do_toml / listar_bins_dir / detectar_alvo / descobrir_pacote (RM)
  ~ invocar(diretorio) {
       let (pacote, alvo) = metadata::detectar_pacote_e_alvo_por_diretorio(dir)?;
       fork::invocar_em(&pacote, Some(dir), Some(&alvo)).map_err(mapear_erro_fork)
    }

03_infra/src/lib.rs
  + pub use metadata::ErroMetadata
  - ErroAdaptador::{CargoTomlAusente, CargoTomlSemPackage, AlvosAmbiguos} (RM)
  + ErroAdaptador::DeteccaoAlvo(ErroMetadata) (+ From<ErroMetadata>)
```

### Verificação grep dos dois subprocessos

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/metadata.rs:157:    let mut cmd = Command::new("cargo");   # cargo metadata
03_infra/src/fork.rs:107:        /// (doc-comment)
03_infra/src/fork.rs:115:    let mut cmd = Command::new("cargo");        # cargo modules
```

Dois Command::new reais, cada um único — invariante reformulado do prompt
0023 ("um só invocador **do fork**" + um só de metadata) cumprido.

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` (sem ignored) | **114 verdes** (mesma contagem do 0022 — supersessão dos testes da heurística por testes da seleção pura, sem inflar nem regredir) |
| `cargo test -p lente_infra -- --ignored` | **9/9** verdes (era 5; +4 novos E2Es do 0023) |
| `cargo build -p lente_core` | só o crate — pureza do L1 preservada |
| Chamadores externos tocados | **zero** (`invocar_fork`, `extrair_grafo`, `desserializar_grafo` mantêm assinaturas) |
| Testes externos alterados | **zero** (todos pré-existentes pelo nome continuam batendo) |

### Mapeamento dos testes (supersessão do 0022)

| Teste do 0022 | Destino no 0023 |
|---------------|-----------------|
| `descobre_pacote_de_cargo_toml_simples` | **removido** — `descobrir_pacote` saiu (resposta via metadata `manifest_path`) |
| `workspace_puro_sem_package_devolve_erro_claro` | **removido** — sem `descobrir_pacote`. Cobertura equivalente: `metadata::e2e_pacote_inexistente_da_erro_proprio` (`PacoteNaoEncontrado`) |
| `detecta_lib_em_pacote_bin_mais_lib` | **substituído** por `metadata::bin_mais_lib_seleciona_lib` (seleção pura sobre JSON literal) |
| `detecta_lib_so_pelo_layout_src_lib_rs` | **substituído** por `metadata::so_lib_seleciona_lib` |
| `detecta_bin_unico_pelo_src_main_rs` | **substituído** por `metadata::so_bin_unico_seleciona_bin_com_nome_do_target` |
| `detecta_bin_unico_explicito_em_cargo_toml` | **substituído** (mesma cobertura: target.name no metadata) |
| `varios_bins_sem_lib_devolve_erro_listando_nomes` | **substituído** por `metadata::multi_bin_sem_lib_devolve_ambiguo_com_nomes` |
| `detecta_bins_em_src_bin_subdir` | **removido** sem substituto direto — o caso "bins implícitos por arquivo" deixa de importar porque metadata já enxerga eles autoritativamente; cobertura é o caso geral de `multi_bin` |
| `e2e_bin_mais_lib_passa_a_funcionar` | **migrado** para `e2e_bin_mais_lib_via_extrair_grafo` (mesmo cenário, novo caminho de detecção) |
| `diretorio_inexistente_da_cargo_toml_ausente` | **substituído** por `diretorio_inexistente_da_erro_de_deteccao_alvo` (variante de erro mudou junto com a remoção de `CargoTomlAusente`) |

### Testes novos do 0023

| Teste | O que prova |
|-------|-------------|
| `bin_mais_lib_seleciona_lib` | seleção sobre JSON literal: target lista `[lib, bin]` → `Lib` |
| `so_lib_seleciona_lib` | `[lib]` → `Lib` |
| `so_bin_unico_seleciona_bin_com_nome_do_target` | nome vem do `target.name`, não do `package.name` (correção sutil sobre a heurística) |
| `proc_macro_conta_como_lib` | `proc-macro` está em `KINDS_LIB` (justificado pelo binário real) |
| `multi_bin_sem_lib_devolve_ambiguo_com_nomes` | regra preservada do 0022, agora confiável |
| `sem_nenhum_alvo_devolve_ambiguo_vazio` | caso degenerado coberto |
| `descoberta_por_nome_acha_pacote_certo_num_workspace` | descoberta por nome num workspace com múltiplos packages |
| `descoberta_por_manifest_path_em_workspace` | descoberta por diretório via `manifest_path` |
| `display_cobre_todas_as_variantes_de_erro_metadata` | sanity de Display |
| `e2e_descoberta_por_nome_no_workspace_real` `#[ignore]` | descoberta de `lente_core` por nome no workspace real |
| `e2e_pacote_inexistente_da_erro_proprio` `#[ignore]` | `PacoteNaoEncontrado` no workspace real |
| `e2e_porta_pacote_descobre_bin_mais_lib_por_nome` `#[ignore]` | **a porta `--pacote` fechada**: fixture bin+lib, descoberta por nome devolve `Lib` |

---

## Decisões tácitas

### D1 — `ErroMetadata` próprio, embrulhado por ambos os erros existentes

Em vez de duplicar variantes em `ErroFork` e `ErroAdaptador`, criei
`ErroMetadata` em `metadata.rs` e ambos os tipos ganharam uma variante
única `DeteccaoAlvo(ErroMetadata)` com `From<ErroMetadata>`. Vantagens:
- A lógica de "modos de falha da descoberta" mora em um lugar só.
- Os dois chamadores (`invocacao::invocar`, `fork::invocar_fork`)
  usam o `?` natural.
- Pattern matching detalhado fica disponível em ambos os lados quando
  necessário (ex.: o teste `pacote_inexistente_retorna_erro_de_deteccao_de_alvo`
  casa `ErroFork::DeteccaoAlvo(ErroMetadata::PacoteNaoEncontrado(_))`).

Custo aceito: dois níveis de aninhamento no match. Para a quantidade de
chamadores hoje (2 internos), é honesto.

### D2 — `descobrir_pacote` removido

A heurística do 0022 mantinha `descobrir_pacote` (parser de `[package]
name`) porque a porta `invocar(diretorio)` precisava do nome para passar a
`--package`. Com metadata, o nome vem junto com os `targets[]`
(`packages[].name` no JSON), e a função
`detectar_pacote_e_alvo_por_diretorio` devolve o par `(nome, alvo)`.
Resultado: a função e suas variantes de erro (`CargoTomlAusente`,
`CargoTomlSemPackage`) saem inteiras. Testes pré-existentes que batiam
nelas foram **substituídos** pelos equivalentes via metadata, registrados
na tabela de mapeamento acima.

### D3 — `selecionar_alvo` é função pura sobre `&MetadataPackage`

A seleção é separada do subprocesso para permitir testes de unidade sem
cargo no PATH. Padrão idêntico ao `traducao` (parsing puro do JSON do
fork). Testes alimentam JSON literal de cada caso (`JSON_BIN_MAIS_LIB`,
`JSON_PROC_MACRO`, etc.); o que precisa de cargo vira `#[ignore]`.

### D4 — `name` do `target` (não do `package`) para `--bin <nome>`

Confirmado em `[[bin]] name = "tool"`: o metadata expõe
`target.name == "tool"` (diferente de `package.name`). A heurística do
0022, para o caso `src/main.rs` implícito, usava `package.name` — só
convergente quando os dois batem. O caminho via metadata é correto por
construção. Teste `so_bin_unico_seleciona_bin_com_nome_do_target` ancora.

### D5 — `NotFound` de `current_dir` colapsa com `cargo` ausente

`Command::output()` falha com `io::ErrorKind::NotFound` em dois casos
indistinguíveis sem inspeção extra: (a) `cargo` ausente do PATH, (b) o
diretório passado em `current_dir(d)` não existe. Optei por manter o
mapeamento simples (`NotFound → ErroMetadata::BinarioNaoEncontrado`) e
registrar a colisão no doc-comment do teste
`diretorio_inexistente_da_erro_de_deteccao_alvo`. Refinar (checar
`d.exists()` antes de invocar) é otimização legítima mas custa código por
um caso que não bloqueia uso normal.

### D6 — Variantes de erro removidas eram só internas — não há período de transição

`grep` da Fase 1 confirmou: `AlvosAmbiguos`, `CargoTomlAusente`,
`CargoTomlSemPackage` só apareciam em `lente_infra`. Sem chamadores
externos — quebra zero. Se algum dia houver, a variante embrulhada
`DeteccaoAlvo(ErroMetadata::AlvosAmbiguos { bins })` cobre o mesmo caso
com pattern matching só uma camada mais profundo.

### D7 — Sem cache de metadata

Cada análise dispara uma chamada de `cargo metadata`. Para a CLI da lente
(uma invocação por uso humano), o custo é desprezível. O prompt 0023
declara "não estruturar antes do uso pedir" — se o ranking em massa
demandar, vira otimização própria.

---

## Não-regressão registrada

| Crate | Antes do 0023 | Depois |
|-------|---------------|--------|
| lente_core | 30 / 0 ignored | 30 / 0 |
| lente_infra (não-ignored) | 27 | **27** (substituição teste-por-teste) |
| lente_infra (ignored) | 5/5 | **9/9** (+4 do 0023: 3 E2Es metadata + 1 E2E porta-pacote) |
| lente_investiga | 17 / 0 | 17 / 0 |
| lente_resolve | 11 / 0 | 11 / 0 |
| lente_wiring | 6 / 1 ignored | 6 / 1 |
| lente_catalogo | 7 / 0 | 7 / 0 |
| lente_cli | 16 / 1 ignored | 16 / 1 |

**Total**: 114 verdes (estabilidade), 11 ignored (era 7; todos passam).

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0023 |
|-----------|-----------------|
| Heurística pode superestimar bins (laudo 0022 D4) | **Coberta** — fonte autoritativa elimina o falso-positivo. |
| Bin+lib via `fork::invocar_fork(pacote)` (porta `--pacote`) | **Coberta** — descoberta por nome via metadata. Sem mudar assinatura. |
| Bin+lib via `extrair_grafo(diretorio)` (porta do laudo 0022) | **Coberta com método novo** — não-regressão preservada via `e2e_bin_mais_lib_via_extrair_grafo`. |
| Diagnóstico de "pacote inexistente" no modo `--pacote` | **Melhorou** — antes vinha do fork como `StatusErro` ("package X not found in workspace"); agora vem do metadata como `PacoteNaoEncontrado(nome)` (variante semântica, antes do fork ser chamado). |
| Filtro de stdlib | **Aberta** — pendência 2 do laudo 0021; prompt próprio. |
| Cache de metadata | **Aberta** — D7 explica por que não entrou. |

---

## Observação metodológica — supersessão granular em cascata

Padrão observado neste prompt (candidato a LESSON do Tekt, registrado no
prompt 0023 §Observação):

```
prompt 0022 — restrição minha larga demais ("um só Command::new")
                ↓ forçou
            D1 (heurística)
                ↓ cujo custo
            D4 (fragilidade em crates exóticos)
prompt 0023 — corrigiu A MONTANTE (a restrição), liberando a solução certa
              (cargo metadata), e D1/D4 saíram inteiros como consequência.
```

A leitura "um só `Command::new`" do 0022 era genérica demais. A intenção
era "não reintroduzir a duplicação **do fork** do laudo 0018" — não
proibir qualquer outro subprocesso. O `cargo metadata` é outro propósito
(descoberta), não duplicação do invocador do fork. O prompt 0023 fez
explicitamente essa supersessão.

Princípio reforçado: **fonte autoritativa > heurística**, sempre que o
custo for aceitável. Pago: 1 subprocesso a mais (irrelevante para uso
humano). Ganho: correção por construção + porta `--pacote` fechada com a
mesma peça.

---

## O que NÃO mudou (declarado)

- `lente_core` (L1): zero toques — pureza preservada.
- `lente_investiga` / `fontes.rs` / E2: em quarentena, intocados.
- `cargo-modules` (fork): nenhuma mudança — usamos `--lib`/`--bin`
  e o subcomando `metadata` que o próprio cargo já oferece.
- Assinaturas públicas (`invocar_fork(pacote)`, `extrair_grafo(diretorio)`,
  `desserializar_grafo(json)`): inalteradas. Caminho aditivo total.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-01 | Detecção de alvo migrada da heurística (laudo 0022 D1/D4) para `cargo metadata` (fonte autoritativa). Porta `--pacote` fechada para bin+lib via descoberta por nome (pendência do 0022). Invariante reformulado: um único invocador do **fork** (`cargo modules`) e um único de **metadata**. | `03_infra/src/metadata.rs` (novo), `03_infra/src/fork.rs`, `03_infra/src/invocacao.rs`, `03_infra/src/lib.rs`, `00_nucleo/lessons/0023-l3-deteccao-alvo-metadata.md` |
