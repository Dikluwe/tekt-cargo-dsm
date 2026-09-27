# Contrato de nucleação — envelope de extração

Unidade: `01_core/core/src/domain/extracao.rs`. Camada L1. ADR-0007.
`IdentidadeProvedor`: RustCargo, Json, Memoria (seleção fechada nesta fase).
`ExtracaoGrafo`: grafo cru válido conforme a spec, identidade do provedor e
`Vec<String>` de diagnósticos não fatais. Apenas dados; sem serde, I/O, caminhos
de executável, comandos ou conhecimento de adaptadores.
Teste: envelope preserva grafo, identidade e diagnósticos.

## Interface Snapshot
<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->
