# Prompt L0: Smoke Test contra Typst Real

**Camada**: L₄ (Fiação) — teste de integração
**Ficheiro alvo**: `04_wiring/tests/typst_smoke_test.rs`
**Passo do roadmap**: validação adicional, entre 1.5 e Fase 2
**Status**: IMPLEMENTADO


---

## Decisões de design prévias

- Roadmap original (Passo 3.1) já previa execução em
  `lab/typst-original/` como critério de sucesso do MVP. Este
  prompt antecipa essa execução como teste **incremental** para
  detectar problemas antes de avançar para a Fase 2.

---

## Decisões locais (assumidas neste prompt)

1. **Marcado como `#[ignore]`**: o teste é rodado manualmente
   pelo agente/desenvolvedor (`cargo test --ignored
   typst_smoke_test`). Não roda em CI nem em `cargo test`
   normal, porque exige um caminho de Typst real no disco.

2. **Caminho via variável de ambiente**: o teste lê
   `TYPST_PATH` do ambiente. Se a variável estiver ausente, o
   teste falha com mensagem clara ("defina TYPST_PATH apontando
   para o lab/typst-original/"). Não há fallback hardcoded; o
   teste é explicitamente operado.

3. **Sem assertions de números mágicos**: o teste não fixa
   "deve ter X nós" ou "deve ter Y arestas". Esses números são
   desconhecidos antes da primeira execução. O teste verifica
   apenas sanidade ("`> 0`", "termina em tempo razoável") e
   imprime as métricas reais via `println!`.

4. **Saída via `println!`**: as métricas são impressas em stdout.
   Para ver, o operador roda
   `cargo test --ignored typst_smoke_test -- --nocapture`.
   Sem geração de arquivo separado.

5. **Não bloqueia merge se falhar**: como `#[ignore]`, mesmo se
   o teste falhar a suíte principal continua verde. O propósito
   é diagnóstico, não gate.

---

## Contexto

Após concluir os Passos 1.1 a 1.5, temos o pipeline completo
montado:

```
Workspace      ─┐
ModuleTrees    ─┼─> build_graph ─> DependencyGraph ─> detect_cycles
ImportEdges    ─┘
```

Este teste exercita o pipeline inteiro contra um codebase real
(Typst, ~50k linhas em Rust, workspace multi-crate) para:

- Validar que não há panics em código real (testes sintéticos
  cobrem casos típicos mas não capturam casos limítrofes).
- Medir tempo de execução total e por fase (input para
  optimização futura, se necessário).
- Descobrir quantos imports `Unresolved` aparecem em código real
  (proporção alta indicaria bug no resolvedor).
- Descobrir se há ciclos no Typst, e se sim, quantos e onde
  (informação interessante por si só).
- Detectar quaisquer `panic`, `unwrap`, ou erros não tratados
  que escaparam aos testes sintéticos.

Este teste é deliberadamente cedo — antes da Fase 2
(renderização DSM) — porque descobrir bugs no pipeline antes de
construir UI em cima é mais barato.

---

## Estrutura do teste

```rust
use crystalline_dsm_cli::graph_builder::build_graph;
use crystalline_dsm_core::entities::dependency_graph::NodeKind;
use crystalline_dsm_core::rules::cycle_detector::detect_cycles;
use crystalline_dsm_infra::cargo_metadata_reader::read_workspace;
use crystalline_dsm_infra::import_extractor::extract_imports;
use crystalline_dsm_infra::module_traverser::traverse_crate;

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

#[test]
#[ignore = "requer TYPST_PATH apontando para lab/typst-original/"]
fn typst_smoke_test() {
    // Fase 0: ler TYPST_PATH do ambiente.
    let typst_path = std::env::var("TYPST_PATH").expect(
        "Variável TYPST_PATH não definida. \
         Aponte para o lab/typst-original/ do typst-crystalline.",
    );
    let typst_path = PathBuf::from(typst_path);
    assert!(
        typst_path.exists(),
        "Path TYPST_PATH não existe: {:?}",
        typst_path,
    );

    println!("\n=== SMOKE TEST: Pipeline contra Typst Real ===");
    println!("Path: {:?}", typst_path);

    // Fase 1: ler workspace.
    let t_workspace_start = Instant::now();
    let workspace = read_workspace(&typst_path)
        .expect("read_workspace falhou contra Typst real");
    let t_workspace = t_workspace_start.elapsed();

    println!("\n--- Fase 1: Resolução de Workspace ---");
    println!("Membros: {}", workspace.member_count());
    println!("Tempo: {:?}", t_workspace);
    for member in &workspace.members {
        println!("  - {} ({:?})", member.name, member.entry_kind);
    }
    assert!(
        workspace.member_count() > 0,
        "Workspace deveria ter pelo menos um membro",
    );

    // Fase 2: traversar módulos de cada crate.
    let t_traverse_start = Instant::now();
    let mut trees = HashMap::new();
    let mut traverse_failures = Vec::new();

    for member in &workspace.members {
        match traverse_crate(member) {
            Ok(tree) => {
                trees.insert(member.name.clone(), tree);
            }
            Err(e) => {
                traverse_failures.push((member.name.clone(), e));
            }
        }
    }
    let t_traverse = t_traverse_start.elapsed();

    let total_modules: usize = trees.values()
        .map(|t| t.node_count())
        .sum();

    println!("\n--- Fase 2: Travessia de Módulos ---");
    println!("Crates processados com sucesso: {}", trees.len());
    println!("Crates com falha: {}", traverse_failures.len());
    println!("Total de módulos mapeados: {}", total_modules);
    println!("Tempo: {:?}", t_traverse);

    if !traverse_failures.is_empty() {
        println!("Falhas:");
        for (name, err) in &traverse_failures {
            println!("  - {}: {:?}", name, err);
        }
    }

    assert!(total_modules > 0, "Devia mapear pelo menos algum módulo");

    // Fase 3: extrair imports de cada crate.
    let workspace_crate_names: Vec<String> = workspace.members
        .iter()
        .map(|m| m.name.clone())
        .collect();

    let t_imports_start = Instant::now();
    let mut edges_per_crate = HashMap::new();
    let mut extract_failures = Vec::new();
    let mut total_unresolved = 0;

    for (crate_name, tree) in &trees {
        let member = workspace.find_member(crate_name)
            .expect("crate name deve existir no workspace");
        match extract_imports(member, tree, &workspace_crate_names) {
            Ok(edges) => {
                total_unresolved += edges.iter()
                    .filter(|e| matches!(
                        e.kind,
                        crystalline_dsm_core::entities::import_edge::ImportKind::Unresolved,
                    ))
                    .count();
                edges_per_crate.insert(crate_name.clone(), edges);
            }
            Err(e) => {
                extract_failures.push((crate_name.clone(), e));
            }
        }
    }
    let t_imports = t_imports_start.elapsed();

    let total_imports: usize = edges_per_crate.values()
        .map(|v| v.len())
        .sum();

    println!("\n--- Fase 3: Extracção de Imports ---");
    println!("Crates processados: {}", edges_per_crate.len());
    println!("Crates com falha: {}", extract_failures.len());
    println!("Total de imports: {}", total_imports);
    println!("Imports Unresolved: {} ({}%)",
             total_unresolved,
             if total_imports > 0 {
                 (total_unresolved * 100) / total_imports
             } else {
                 0
             });
    println!("Tempo: {:?}", t_imports);

    if !extract_failures.is_empty() {
        println!("Falhas:");
        for (name, err) in &extract_failures {
            println!("  - {}: {:?}", name, err);
        }
    }

    // Fase 4: construir grafo.
    let t_graph_start = Instant::now();
    let graph = build_graph(&workspace, &trees, &edges_per_crate);
    let t_graph = t_graph_start.elapsed();

    let internal_count = graph.internal_node_count();
    let external_count = graph.external_node_count();
    let total_nodes = graph.node_count();
    let edge_count = graph.edge_count();

    println!("\n--- Fase 4: Construção do Grafo ---");
    println!("Nós totais: {}", total_nodes);
    println!("  Internos: {}", internal_count);
    println!("  Externos: {}", external_count);
    println!("Arestas: {}", edge_count);
    println!("Tempo: {:?}", t_graph);

    assert!(total_nodes > 0, "Grafo deveria ter pelo menos um nó");
    assert!(edge_count > 0, "Grafo deveria ter pelo menos uma aresta");

    // Top 10 externos mais usados.
    let mut external_usage: Vec<(String, usize)> = graph
        .external_nodes()
        .map(|(id, node)| {
            (node.canonical_path.clone(), graph.in_degree(id))
        })
        .collect();
    external_usage.sort_by(|a, b| b.1.cmp(&a.1));

    println!("\nTop 10 externos mais usados:");
    for (path, count) in external_usage.iter().take(10) {
        println!("  {} ({} usos)", path, count);
    }

    // Fase 5: detectar ciclos.
    let t_cycles_start = Instant::now();
    let report = detect_cycles(&graph);
    let t_cycles = t_cycles_start.elapsed();

    println!("\n--- Fase 5: Detecção de Ciclos ---");
    println!("Ciclos detectados: {}", report.cycle_count());
    println!("  Multi-nó: {}", report.multi_node_cycle_count());
    println!("  Self-loops: {}", report.self_loop_count());
    println!("Nós afectados: {}", report.affected_node_count());
    println!("Tempo: {:?}", t_cycles);

    if report.has_cycles() {
        println!("\nCiclos encontrados:");
        for (i, cycle) in report.cycles.iter().take(5).enumerate() {
            let kind_str = match cycle.kind {
                crystalline_dsm_core::rules::cycle_detector::CycleKind::MultiNode => "multi",
                crystalline_dsm_core::rules::cycle_detector::CycleKind::SelfLoop => "self",
            };
            let paths: Vec<&str> = cycle.nodes.iter()
                .map(|id| graph.node(*id).canonical_path.as_str())
                .collect();
            println!("  Ciclo #{} ({}): {}",
                     i + 1, kind_str, paths.join(" → "));
        }
        if report.cycles.len() > 5 {
            println!("  ... e mais {} ciclos.",
                     report.cycles.len() - 5);
        }
    }

    // Resumo total.
    let t_total = t_workspace + t_traverse + t_imports + t_graph + t_cycles;
    println!("\n=== RESUMO ===");
    println!("Tempo total do pipeline: {:?}", t_total);
    println!(
        "Distribuição: workspace={:?}, traverse={:?}, \
         imports={:?}, graph={:?}, cycles={:?}",
        t_workspace, t_traverse, t_imports, t_graph, t_cycles,
    );

    // Sanidade: deve terminar em tempo razoável.
    // Target da ADR-0001: < 30s para primeira execução do Typst.
    // Margem generosa aqui (60s) para acomodar máquinas lentas.
    assert!(
        t_total < std::time::Duration::from_secs(60),
        "Pipeline tomou tempo excessivo: {:?}",
        t_total,
    );
}
```

---

## Critérios de sucesso

O teste é considerado bem-sucedido se, na primeira execução
contra o Typst real:

1. Termina sem panic.
2. Termina em < 60 segundos (target da ADR-0001 é 30s; margem
   dobrada para máquinas lentas).
3. Mapeia pelo menos 1 crate, 1 módulo, 1 import.
4. Constrói grafo com `node_count > 0` e `edge_count > 0`.
5. Detector de ciclos termina sem panic (resultado em si — com ou
   sem ciclos — é informação a inspecionar, não critério de
   sucesso/falha).

### Sinais de bug a investigar

Mesmo se o teste passar, certos números merecem inspecção manual:

- **`Unresolved` > 5% do total de imports**: pode indicar bug
  no resolvedor de `super::`/`crate::` ou caso não previsto.
- **Crates com falha em `traverse` ou `extract`**: cada falha é
  um caso real que escapou aos testes sintéticos. Investigar.
- **Tempo total > 30s**: dentro do critério de sucesso ampliado
  (< 60s) mas acima do target da ADR-0001. Vale investigar para
  Passo 3.1.
- **Quantidade de nós externos absurdamente alta** (ex: > 500):
  pode indicar bug na deduplicação ou na classificação.
- **Ciclos multi-nó** no Typst: provavelmente são reais se
  Typst tiver, mas Typst é projecto maduro; alguns ciclos podem
  ser falsos positivos do nosso parser. Vale investigar
  caso-a-caso.

---

## Acções decorrentes (após execução)

Independentemente do resultado do teste, registar manualmente:

1. **Métricas obtidas**: número de crates, módulos, imports,
   tempo de cada fase, número de ciclos. Anotar em comentário
   de commit ou nota interna.

2. **Bugs descobertos**: cada falha vira issue ou TODO no
   roadmap.

3. **Considerações para a ADR-0001 (critério de sucesso do MVP)**:
   se o teste passar com folga, marcar critério 1 ("Roda em
   `lab/typst-original/` sem panic e em tempo razoável") como
   parcialmente cumprido (parcialmente porque ainda falta o DSM
   HTML, critério 2 da ADR-0001).

---

## Dependências externas

`04_wiring/Cargo.toml`:
- Já tem todas as crates necessárias (`crystalline-dsm-core`,
  `crystalline-dsm-infra`).
- Nenhuma nova.

Em particular, NÃO adicionar:
- `criterion` ou outras crates de benchmark. Este não é um
  benchmark formal; é diagnóstico.
- `serde` ou serialização. Output é stdout.

---

## Critério de aceitação do prompt

- O ficheiro `04_wiring/tests/typst_smoke_test.rs` existe.
- Marcado com `#[ignore]`.
- Compila e passa `cargo test --ignored typst_smoke_test`
  contra um Typst real apontado por `TYPST_PATH`.
- Imprime as métricas listadas via `println!`.
- Não fixa números mágicos como assertions.
- `cargo clippy --all-targets` continua sem warnings.

---

## Como executar

```bash
export TYPST_PATH=/caminho/para/typst-crystalline/lab/typst-original
cargo test --ignored typst_smoke_test -- --nocapture
```

A flag `--nocapture` faz o cargo mostrar o output dos `println!`
durante a execução (por defeito, o cargo captura stdout em
testes que passam).

---

## Histórico de Revisões

- **2026-05-20**: Implementado e executado com sucesso contra o Typst real. O pipeline concluiu a análise em 4.08 segundos, identificando 1928 nós e 17 ciclos complexos.

