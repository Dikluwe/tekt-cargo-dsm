# Contrato experimental — dependências estáticas de arquivos JS/TS

2026-09-26. Arena independente: nenhum componente do workspace principal muda.
Compatibilidade: DTOs e validações de `lente_infra::desserializar_grafo` atuais.

O compilador TypeScript 5.9.3 lê tsconfig/extends e seleciona arquivos; a mesma
API resolve módulos com suas opções (incluindo aliases, NodeNext e index).
O universo inclui raízes do tsconfig e fechamento de imports/reexports estáticos
locais suportados, inclusive arquivos importados fora de include/exclude.
Não expandir project references silenciosamente: erro explícito.

Cada arquivo é um nó `mod`, `visibility=priv` como valor de compatibilidade,
sem afirmar visibilidade de exports. Relações são `uses`/`uses_kind=import`,
nunca chamadas. Preservar tipo de ocorrência (import/reexport, type-only),
origem e posição em metadados separados. Uma aresta por par de arquivos.
IDs ordenados pelo caminho normalizado; from/to iguais ao path do respectivo ID.

Externos resolvidos pelo TypeScript ou em node_modules: registrar, excluir
nós e arestas, não percorrer. Módulo não resolvido: diagnóstico, sem nó fictício.
Imports dinâmicos, CommonJS, import-equals/import-types, referências triple-slash,
módulos ambiente, import.meta e recursos não JS/TS: diagnóstico, sem inventar
arestas. Não executar projeto, emit, typecheck, npm ou downloads na análise.
Nenhum cache persistente; somente estruturas internas do compilador por rodada.

JSON do grafo contém somente crate/nodes/edges. Metadados incluem provedor,
versões, granularidade arquivo, configuração, limites, diagnósticos, ocorrências,
cobertura e SHA-256 dos bytes do grafo. Erro fatal não publica grafo de sucesso.
CLI exige diretório de saída sem grafo/metadados anteriores para evitar confusão
com resultados antigos. Arquivos publicados são exclusivos, sem sobrescrita.

Verificação: fixtures locais com imports, aliases, index, reexports, tipos,
externos, irresolvidos, ciclos, JS/JSX/TSX, NodeNext, determinismo e ausência de
execução. Harness Cargo isolado apenas em compat/ chama desserializar_grafo,
calcular_raio e detectar_ciclos existentes via dependências por caminho.
