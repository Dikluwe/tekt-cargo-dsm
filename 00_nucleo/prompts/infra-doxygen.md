# Projeção documental do grafo recebido
Hash do Código: 85175a06

Unidade L3 03_infra/src/doxygen.rs, ADR-0009. Recebe bytes UTF-8 do grafo,
politica_nomes opaque (default) ou rust-path (declaração explícita do provedor),
coverage complete/partial/unknown (default unknown), alvos_metricas opcional.
SHA-256 dos bytes exatos; desserializador existente e validação id/path endpoints.
Catálogo v1: node_id original, qualified_name null para opaque ou path literal
para rust-path; signature null, location de position.file/start_line positiva.
Diagnósticos explícitos para ausência de nome/posição/assinatura. Contagens
montante calculadas exclusivamente por calcular_raio, com paths únicos. Ciclos
terminam, alvo excluído, Owns ignorado, todas Uses, consumidores distintos.
Alvos não selecionados têm métrica null com diagnóstico; IDs de seleção inválidos
ou repetidos são erro. Se path colide, todas as métricas ficam null, coverage
impact partial, sem conflar identidades. Sem I/O de fontes ou execução de extrator.
Retorna objeto com schema lente.doxygen.graph_projection v1, graph_digest,
política e cobertura declaradas, catalog e impact. Catálogo/impact partilham o hash.
Não presume equivalência semântica nem identifica assinaturas C++.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"projetar","params":["&[u8]","&str","&str","Option<&[u64]>"],"return_type":"Result<Value, String>"}],"types":[],"reexports":[]} -->
