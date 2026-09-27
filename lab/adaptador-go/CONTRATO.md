# Contrato experimental v1

Linhagem: pedido de adaptador Go de 2026-09-26; AGENTS.md raiz,
00_nucleo/specs/forma-organizada.md e ADR-0002.

O executável Rust isolado consulta somente `go env` e `go list`, com configuração
explícita GOOS/GOARCH, tags, política de testes, workspace e módulos readonly.
Downloads e seleção automática de toolchain são bloqueados. cgo fica desligado.
Não há cache de grafos. A saída contém dois documentos separados: grafo aceito
por desserializar_grafo e metadados obrigatórios para interpretar sua cobertura.

Identidade = ImportPath integral do go list, incluindo qualificadores de teste.
Nós mod, visibilidade pub apenas para compatibilidade; imports diretos uses/import.
ImportMap resolve remapeamentos; Deps nunca gera arestas. IDs são atribuídos em
ordem lexicográfica e não são identidades persistentes entre extrações distintas.
Erros de carregamento, falhas de subprocesso e endpoints ausentes produzem
resultado parcial e diagnóstico. Endpoints ausentes recebem nós explicitamente
marcados como não carregados nos metadados. Código 0: carregamento sem erros;
2: parcial/falhou; 1: uso, I/O ou contrato inválido. Carregamento completo não
significa completude semântica, compilação bem-sucedida ou equivalência funcional.

Testes inline exercitam ferramentas oficiais sobre fixtures locais, sem executar
código Go. O núcleo local calcula a transitividade. Alterações se limitam a este
laboratório, inclusive documentação, fixtures e artefatos de build.
