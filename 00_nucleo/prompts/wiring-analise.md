# Contrato de nucleação — preparação e análise independentes da extração

Unidade: `04_wiring/src/analise.rs`. Camada L4. ADR-0007.
Seleção explícita de Rust/JSON/memória por solicitação, sem registro dinâmico.
`preparar_analise_com` recebe um ExtratorGrafo injetado, extrai uma vez,
resolve colisões e aplica o escopo. `preparar_analise` seleciona o adaptador.
`AnaliseGrafo` conserva o envelope com grafo preparado, provedor e diagnósticos;
expõe consultas de raio, ranking, estrutura/ciclos e impacto de um diff já lido.
Consultas só compõem algoritmos existentes, sem processos nem leitura de arquivo.
JSON/Pacote legados usam a mesma preparação e preservam resultados analíticos.
Falhas do provedor continuam tipadas no ErroLente; resolução mantém seu erro.
Testes com extrator em memória verificam extração única, preservação de avisos,
igualdade com JSON e funções L1, incluindo colisões, ciclos, ranking e impacto.

## Interface Snapshot
<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->
