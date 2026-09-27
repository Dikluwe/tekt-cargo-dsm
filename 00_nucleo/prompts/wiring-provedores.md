# Contrato — análise de provedores externos
Hash do Código: ee66dbbc

Unidade `04_wiring/src/provedores.rs`, L4; ADR-0008.
`analisar_provedor` valida consulta e configuração, executa ExtratorFerramenta pela
fronteira ExtratorGrafo e compõe raio, ranking ou estrutura com APIs existentes.
Não resolve colisões Rust, filtra prefixos stdlib, faz diff, cache ou comparação.
Formatação analítica reutiliza lente_cli. Envelope único JSON para CLI/MCP contém
analise e extracao integral com metadados/verificação. Completudes distintas ficam
no summary do verificador; não converte unknown/partial em complete.
Erros preservam categorias da infraestrutura; consulta é enum serde estrito.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"analisar_provedor","params":["&str","&str","ConsultaProvedor"],"return_type":"Result<String, ErroFerramenta>"}],"types":[{"name":"ConsultaProvedor","kind":"enum","members":["Ranking","Raio","RaioId","Estrutura"]}],"reexports":[]} -->
