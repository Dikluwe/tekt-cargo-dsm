> Estado em 2026-09-27: proposta histórica não materializada nas unidades
> indicadas abaixo. Preservada para rastreabilidade; não descreve API disponível.
> Implementação vigente: ADR-0008, infra-provedores.md e wiring-provedores.md.

# Adaptador TypeScript: dependências estáticas entre arquivos (L3)

**Unidades**: `03_infra/typescript/extrair.cjs`, `03_infra/src/typescript.rs`.
**Data**: 2026-09-26. Contrato de implementação; integração com tarefas 5 e 6 pendente de identificação.

## Escopo e fonte

Node.js >=20 e TypeScript **5.9.3**, fixado em package.json/package-lock.json
próprios do adaptador. A análise nunca instala ferramentas nem executa módulos,
scripts, plugins ou compilação do projeto. O helper carrega somente sua própria
dependência TypeScript. Instalação explícita separada: `npm ci --ignore-scripts`.

`getParsedCommandLineOfConfigFile` lê tsconfig, extends, files/include/exclude e
opções. `createProgram` fornece ASTs e contexto de resolução;
`resolveModuleName` + `program.getModeForUsageLocation` resolvem imports segundo
as opções, inclusive paths/baseUrl, extensões, index e condições NodeNext.
Nenhum resolvedor baseado em regex. Nenhum emit, typecheck ou build.

## Universo e relações

Os nós analisados são os arquivos raiz selecionados pelo tsconfig e o fechamento
transitivo de **imports/reexports estáticos suportados** que resolvam para arquivos
locais JS/TS. Exclude restringe a seleção inicial, não proíbe um arquivo importado
(comportamento TypeScript). Bibliotecas padrão e pacotes externos não são
percorridos. `.d.ts` locais são arquivos; declarations de pacotes são externas.

Imports normais, side-effect, `import type`, especificadores `type`, `export from`,
`export *`, `export type` são dependências entre arquivos, nunca chamadas.
O helper preserva natureza (import/reexport), type-only e posição da declaração
em ocorrências separadas. A tradução para o grafo usa somente relações que a
fronteira vigente suporte; não atribui semântica de chamada nem exportação pública
a arquivos por inferência. Distinções não representáveis permanecem em metadados.

Política externa: resoluções marcadas externas pelo TypeScript ou dentro de
node_modules são registradas em ocorrências, com destino resolvido e diagnóstico
`external-excluded`; não há nó nem aresta para código externo. Caminhos locais
fora da raiz do tsconfig são incluídos quando importados. Módulos não resolvidos
ficam em diagnósticos com arquivo, especificador e posição; nunca viram nós
sintéticos nem arestas inventadas. Recursos não JS/TS resolvidos (JSON etc.)
são registrados como fora do escopo, sem aresta.

IDs e caminhos normalizados são ordenados lexicalmente, sem dependência de locale;
linhas/colunas são 1-based. IDs inteiros estáveis na repetição da mesma extração.
Nós trazem posição do arquivo, ocorrências trazem posição do import/reexport.
Erros de leitura/configuração/sintaxe impedem sucesso silencioso. Diagnósticos e
limitações são parte da saída, não confundidos com ausência de dependências.

## Limitações declaradas

Não extrai chamadas, fluxo de dados, dynamic import(), require/CommonJS,
import-equals, import type em expressão (`type T = import(...)`), referências
triple-slash, módulos ambiente, plugins de bundler, URLs remotas, arquivos Vue/Svelte,
emissão ou semântica em runtime. Construções reconhecidas como não suportadas
recebem diagnóstico sem aresta. Project references não são expandidas: erro
explícito pede analisar cada tsconfig separadamente. Sem cache inicialmente.

## Verificação

Fixtures locais: import relativo/index; paths; reexport/type-only; externo;
não resolvido; ciclo; JS/JSX/TSX com allowJs; configurações estendidas e exclude;
opções NodeNext; repetição byte a byte; código top-level nunca executado;
erro de configuração/sintaxe; impacto transitivo no núcleo.

## Interface Snapshot
<!-- crystalline-snapshot: {"functions":[],"types":[],"reexports":[]} -->
