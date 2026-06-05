# Laudo de Execução — Prompt 0024 (Diretório inexistente: diagnóstico próprio)

**Camada**: L5 (laudo)
**Data**: 2026-06-02
**Prompt executado**: `00_nucleo/prompt/0024-l3-diretorio-inexistente-diagnostico.md`
**Decisões de origem**: laudo 0023 D5 — `Command::current_dir(d)` num diretório
inexistente falha com `io::ErrorKind::NotFound`, indistinguível de `cargo`
ausente do PATH; a D5 documentava a colisão como aceita. Em relação ao 0022
isso era **piora** (lá, a falha era "Cargo.toml ausente" — clara). Este
prompt restaura a clareza sobre o caminho via metadata.
**Estado**: `EXECUTADO` — `ErroMetadata::DiretorioInexistente(PathBuf)`
adicionada; checagem `is_dir()` curto-circuita antes do spawn em
`detectar_pacote_e_alvo_por_diretorio`; `BinarioNaoEncontrado` no caminho com
diretório passa a ser inequivocamente "cargo ausente". Pureza do L1 intacta;
117 verdes (era 114; +3 do 0024) + 8 ignored — todos passam.

---

## Fase 1 — Leitura

| Item | Achado |
|------|--------|
| `metadata.rs:invocar_metadata` | Aplica `cmd.current_dir(d)` quando `Some`; mapeia `io::Error` no `.output()` para `BinarioNaoEncontrado` se `NotFound`, senão `FalhaSubprocess(String)`. Esse mapeamento é o único ponto da colisão D5. |
| Caminho `--pacote` (`fork::invocar_fork` → `current_dir = None`) | Cwd do processo sempre existe → NotFound aqui é mesmo "cargo ausente"; nada a mudar. |
| Caminho com diretório (`detectar_pacote_e_alvo_por_diretorio`) | Único ponto com `current_dir = Some(dir)`; é onde a checagem precisa entrar. |
| Teste antigo `diretorio_inexistente_da_erro_de_deteccao_alvo` | `#[ignore]` (precisava de cargo só para gerar o erro). Afirmava só `ErroAdaptador::DeteccaoAlvo(_)` — variante embrulhada genérica, documentando a colisão. |
| Display de `ErroMetadata` | Lista variantes existentes; precisa do novo ramo para `DiretorioInexistente`. |

Decisão da Fase 1 sobre **onde a checagem mora**: em
`detectar_pacote_e_alvo_por_diretorio`, **não** dentro de `invocar_metadata`.
Razão: mantém `invocar_metadata` com responsabilidade pura ("só roda o
subprocesso") e concentra a guarda exatamente onde o `Path` tipado vem do
chamador — sem espalhar pré-condição opaca pelos demais consumidores
hipotéticos.

---

## Fase 2 — Conserto

### Variante de erro nova

```rust
// metadata.rs — ErroMetadata
DiretorioInexistente(PathBuf),
```

Display: `"diretório não existe: {path}"`. **Não** se reusa
`PacoteNoDiretorioNaoEncontrado`: esta significa "o diretório existe, mas
nenhum pacote do metadata casa" — causa diferente (workspace puro, sem
`[package]`). Reusar repetiria o tipo de colisão que está sendo consertada.

### Checagem `is_dir()` antes do spawn

```rust
pub(crate) fn detectar_pacote_e_alvo_por_diretorio(
    diretorio: &Path,
) -> Result<(String, AlvoFork), ErroMetadata> {
    if !diretorio.is_dir() {
        return Err(ErroMetadata::DiretorioInexistente(diretorio.to_path_buf()));
    }
    let md = invocar_metadata(Some(diretorio))?;
    // …
}
```

Custo: um `stat(2)`. Ganho: diagnóstico preciso + colisão D5 removida por
construção.

`invocar_metadata` segue inalterado (mantém a forma do laudo 0023; o
mapeamento `NotFound → BinarioNaoEncontrado` faz sentido nesse contexto
porque a porta `--pacote` chama com `None`).

### Teste antigo agora afirma a variante correta — e perde o `#[ignore]`

```rust
// invocacao.rs
#[test]
fn diretorio_inexistente_da_diretorio_inexistente() {
    let inexistente = Path::new("/tmp/__lente_diretorio_inexistente_0024__xyz");
    match invocar(inexistente).unwrap_err() {
        ErroAdaptador::DeteccaoAlvo(crate::metadata::ErroMetadata::DiretorioInexistente(p))
            => assert_eq!(p, inexistente),
        outro => panic!("erro inesperado: {:?}", outro),
    }
}
```

Sem `#[ignore]`: a checagem curto-circuita antes do spawn — não há
subprocess, então não há dependência de `cargo` no PATH. Vira teste de
unidade determinístico.

### Testes novos em `metadata.rs`

- `por_diretorio_curto_circuita_em_diretorio_inexistente` — afirma a
  variante e o caminho exato passado.
- `por_diretorio_arquivo_e_nao_diretorio_tambem_curto_circuita` — o caso
  "passou um arquivo, não um diretório" também dispara (uso de `is_dir`,
  não `exists`).
- `display_cobre_todas_as_variantes_de_erro_metadata` — ganhou a variante
  nova.

### Verificação grep dos dois subprocessos

```
$ grep -rn 'Command::new("cargo")' --include "*.rs"
03_infra/src/metadata.rs:170:    let mut cmd = Command::new("cargo");   # cargo metadata
03_infra/src/fork.rs:117:    let mut cmd = Command::new("cargo");        # cargo modules
```

Dois subprocessos, cada um único — invariante do laudo 0023 preservado
(este prompt não cria subprocess novo).

---

## Verificação

| Item | Resultado |
|------|-----------|
| `cargo test --workspace` (sem ignored) | **117 verdes** (era 114 no 0023; +3 efetivos do 0024) |
| `cargo test -p lente_infra -- --ignored` | **8/8** verdes (era 9 no 0023; -1 porque o teste antigo perdeu o `#[ignore]` e migrou para a contagem regular) |
| Chamadores externos tocados | **zero** (assinaturas públicas inalteradas) |
| Testes pré-existentes alterados | **um**: o `diretorio_inexistente_da_erro_de_deteccao_alvo` renomeado e refinado para `diretorio_inexistente_da_diretorio_inexistente` — afirmação mais forte, sem `#[ignore]`. |

### Tabela de não-regressão (efeito da mudança em testes pré-existentes)

| Cenário | Antes (0023) | Depois (0024) |
|---------|--------------|---------------|
| Diretório inexistente | `DeteccaoAlvo(BinarioNaoEncontrado)` ("cargo ausente") — `#[ignore]` | `DeteccaoAlvo(DiretorioInexistente(p))` — sem ignore |
| Workspace puro (existe, sem `[package]`) | `DeteccaoAlvo(PacoteNoDiretorioNaoEncontrado(_))` | **inalterado** |
| Diretório válido + crate | sucesso | **inalterado** |
| Cargo ausente do PATH | `BinarioNaoEncontrado` | **inalterado** (mapeamento preservado; só vale agora para "cargo ausente" mesmo) |

---

## Decisões tácitas

### D1 — Checagem em `detectar_pacote_e_alvo_por_diretorio`, não em `invocar_metadata`

`invocar_metadata` continua sendo "só roda o subprocesso" — a pré-condição
"diretório existe quando passado" vive no chamador que tem o `Path` tipado
e a intenção. Alternativa rejeitada: enfiar `if let Some(d) = current_dir
&& !d.is_dir() { … }` dentro de `invocar_metadata`. Não fica errado, mas
mistura responsabilidades; e se um chamador futuro quiser passar um
diretório opcional sem essa checagem (improvável mas possível), a guarda
ficaria caminho-do-meio.

### D2 — `is_dir()`, não `exists()`

`is_dir` cobre dois casos com a mesma mensagem clara: caminho não existe,
e caminho é arquivo (uso comum: usuário apontou para o `Cargo.toml`
diretamente em vez do diretório). Dois testes ancoram. `exists()` deixaria
o caso-arquivo cair em `StatusErro` do cargo ("could not find Cargo.toml")
— não terrível, mas o `DiretorioInexistente` dá o caminho exato sem
parsear stderr.

### D3 — Mapeamento `NotFound → BinarioNaoEncontrado` mantido

Tentei: "deveria refinar a mensagem do NotFound também?" Não. Depois da
checagem, no caminho com diretório, qualquer NotFound restante é genuíno
"cargo ausente do PATH" (impossível ser cwd, já filtrado). No caminho
`--pacote` (`current_dir = None`), NotFound também é só "cargo ausente"
(cwd herdado sempre existe). O mapeamento é correto **por construção**
nos dois caminhos depois deste prompt — não há ambiguidade para refinar.

### D4 — Variante nova, não embrulhada noutra existente

Avaliado: poderia-se passar o caminho dentro de
`PacoteNoDiretorioNaoEncontrado(String)`. Rejeitado: as duas causas são
distintas ("o diretório não existe" vs "o diretório existe mas nenhum
pacote casa"). Compartilhar variante repetiria a colisão D5 num nível
acima. Variante nova é a forma honesta.

---

## Por que este laudo é mais curto que o 0022/0023

Convergente com o padrão da observação metodológica do laudo 0021: laudos
de **componente nucleado** novo (0022, 0023 — mudaram estrutura,
exibiram regras, supersessões) são densos; laudos de **correção
localizada** (0024 — uma variante de erro + uma checagem + um teste
migrado) são curtos. O conteúdo principal mora no código e nos testes;
este registro só ancora a decisão e a sua razão.

---

## Pendências cobertas / abertas

| Pendência | Estado pós-0024 |
|-----------|-----------------|
| Colisão D5 do laudo 0023 (`NotFound` ambíguo) | **Coberta**. Caminho com diretório distingue antes do spawn; caminho sem diretório não tem ambiguidade. |
| Restauração da clareza diagnóstica do laudo 0022 | **Coberta** (sobre o caminho via metadata). |
| Filtro de stdlib | **Aberta** — pendência 2 do laudo 0021; prompt próprio. |

---

## O que NÃO mudou (declarado)

- `lente_core` (L1): zero toques.
- `lente_investiga` / E2: quarentena intocada.
- `cargo-modules` (fork): nenhuma mudança.
- Assinaturas públicas (`extrair_grafo`, `invocar_fork`,
  `desserializar_grafo`): inalteradas.
- Subprocessos do cargo: continuam 2, cada um único.
- Pureza do L1: `cargo build -p lente_core` sem dependências novas.

---

## Histórico de Revisões

| Data | Motivo | Arquivos afetados |
|------|--------|-------------------|
| 2026-06-02 | Diretório inexistente passa a diagnosticar a causa real (`ErroMetadata::DiretorioInexistente(PathBuf)`) via checagem `is_dir` antes do spawn em `detectar_pacote_e_alvo_por_diretorio`. Fecha a D5 do laudo 0023; sem subprocess novo nem mudança de assinatura. Teste pré-existente sai do `#[ignore]` (vira unidade determinística). | `03_infra/src/metadata.rs`, `03_infra/src/invocacao.rs`, `00_nucleo/lessons/0024-l3-diretorio-inexistente-diagnostico.md` |
