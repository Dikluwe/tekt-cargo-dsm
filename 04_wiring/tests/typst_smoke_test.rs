/*
 * Crystalline Lineage
 * @prompt 00_nucleo/prompts/typst-smoke-test.md
 * @prompt 00_nucleo/prompts/smoke-test-diagnostico.md
 * @prompt 00_nucleo/prompts/dsm_partitioner.md
 * @layer L4
 * @updated 2026-05-20
 */

use crystalline_dsm_core::rules::cycle_detector::detect_cycles;
use crystalline_dsm_core::rules::dsm_partitioner::partition_for_dsm;
use crystalline_dsm_infra::cargo_metadata_reader::read_workspace;
use crystalline_dsm_infra::import_extractor::{ExtractError, extract_imports};
use crystalline_dsm_infra::module_traverser::{TraverseError, traverse_crate};

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

#[path = "../src/graph_builder.rs"]
mod graph_builder;

use graph_builder::build_graph;

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
    let workspace = read_workspace(&typst_path).expect("read_workspace falhou contra Typst real");
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

    let total_modules: usize = trees.values().map(|t| t.node_count()).sum();

    println!("\n--- Fase 2: Travessia de Módulos ---");
    println!("Crates processados com sucesso: {}", trees.len());
    println!("Crates com falha: {}", traverse_failures.len());
    println!("Total de módulos mapeados: {}", total_modules);
    println!("Tempo: {:?}", t_traverse);

    if !traverse_failures.is_empty() {
        println!("\nFalhas em traversal:");
        for (name, err) in &traverse_failures {
            println!("  Crate: {}", name);
            match err {
                TraverseError::ModuleFileNotFound {
                    module,
                    parent_file,
                    attempted_paths,
                } => {
                    println!("    Tipo: ModuleFileNotFound");
                    println!("    Módulo procurado: {}", module);
                    println!("    Declarado em: {}", parent_file.display());
                    println!("    Caminhos tentados:");
                    for p in attempted_paths {
                        println!("      - {}", p.display());
                    }
                }
                TraverseError::ParseFailed { file, source } => {
                    println!("    Tipo: ParseFailed");
                    println!("    Ficheiro: {}", file.display());
                    println!("    Erro de parser: {}", source);
                }
                TraverseError::FileReadFailed { path, source } => {
                    println!("    Tipo: FileReadFailed");
                    println!("    Caminho: {}", path.display());
                    println!("    Erro de I/O: {}", source);
                }
                TraverseError::TreeError(e) => {
                    println!("    Tipo: TreeError");
                    println!("    Detalhe: {:?}", e);
                }
            }
        }
    }

    assert!(total_modules > 0, "Devia mapear pelo menos algum módulo");

    // Fase 3: extrair imports de cada crate.
    let workspace_crate_names: Vec<String> =
        workspace.members.iter().map(|m| m.name.clone()).collect();

    let t_imports_start = Instant::now();
    let mut edges_per_crate = HashMap::new();
    let mut extract_failures = Vec::new();
    let mut total_unresolved = 0;

    for (crate_name, tree) in &trees {
        let member = workspace
            .find_member(crate_name)
            .expect("crate name deve existir no workspace");
        match extract_imports(member, tree, &workspace_crate_names) {
            Ok(edges) => {
                total_unresolved += edges
                    .iter()
                    .filter(|e| {
                        matches!(
                            e.kind,
                            crystalline_dsm_core::entities::import_edge::ImportKind::Unresolved,
                        )
                    })
                    .count();
                edges_per_crate.insert(crate_name.clone(), edges);
            }
            Err(e) => {
                extract_failures.push((crate_name.clone(), e));
            }
        }
    }
    let t_imports = t_imports_start.elapsed();

    let total_imports: usize = edges_per_crate.values().map(|v| v.len()).sum();

    println!("\n--- Fase 3: Extracção de Imports ---");
    println!("Crates processados: {}", edges_per_crate.len());
    println!("Crates com falha: {}", extract_failures.len());
    println!("Total de imports: {}", total_imports);
    println!(
        "Imports Unresolved: {} ({}%)",
        total_unresolved,
        if total_imports > 0 {
            (total_unresolved * 100) / total_imports
        } else {
            0
        }
    );
    println!("Tempo: {:?}", t_imports);

    if !extract_failures.is_empty() {
        println!("\nFalhas em extracção de imports:");
        for (name, err) in &extract_failures {
            println!("  Crate: {}", name);
            match err {
                ExtractError::FileReadFailed { path, source } => {
                    println!("    Tipo: FileReadFailed");
                    println!("    Caminho: {}", path.display());
                    println!("    Erro de I/O: {}", source);
                }
                ExtractError::ParseFailed { file, source } => {
                    println!("    Tipo: ParseFailed");
                    println!("    Ficheiro: {}", file.display());
                    println!("    Erro de parser: {}", source);
                }
            }
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
        .map(|(id, node)| (node.canonical_path.clone(), graph.in_degree(id)))
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
            let paths: Vec<&str> = cycle
                .nodes
                .iter()
                .map(|id| graph.node(*id).canonical_path.as_str())
                .collect();
            println!("  Ciclo #{} ({}): {}", i + 1, kind_str, paths.join(" → "));
        }
        if report.cycles.len() > 5 {
            println!("  ... e mais {} ciclos.", report.cycles.len() - 5);
        }
    }

    // Fase 6: particionamento DSM.
    let t_partition_start = Instant::now();
    let partition = partition_for_dsm(&graph);
    let t_partition = t_partition_start.elapsed();

    println!("\n--- Fase 6: Particionamento DSM ---");
    println!("order.len(): {}", partition.order.len());
    println!("internal_boundary: {}", partition.internal_boundary);
    println!("Total SCCs: {}", partition.sccs.len());
    println!("  Cíclicos: {}", partition.cyclic_scc_count());
    println!("  Triviais: {}", partition.trivial_scc_count());
    println!("Tempo: {:?}", t_partition);

    assert_eq!(partition.order.len(), graph.node_count());
    assert_eq!(partition.internal_boundary, internal_count);
    // O número de SCCs cíclicos deve bater com o número de ciclos
    // detectados pelo cycle_detector (mesma definição matemática).
    assert_eq!(
        partition.cyclic_scc_count(),
        report.cycle_count(),
        "cyclic_scc_count e cycle_count devem coincidir",
    );

    // Resumo total.
    let t_total = t_workspace + t_traverse + t_imports + t_graph + t_cycles + t_partition;
    println!("\n=== RESUMO ===");
    println!("Tempo total do pipeline: {:?}", t_total);
    println!(
        "Distribuição: workspace={:?}, traverse={:?}, \
         imports={:?}, graph={:?}, cycles={:?}, partition={:?}",
        t_workspace, t_traverse, t_imports, t_graph, t_cycles, t_partition,
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
