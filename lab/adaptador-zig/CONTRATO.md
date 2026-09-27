# Contrato experimental Zig — 2026-09-26

Somente esta Arena. Parser real: std.zig.Ast da instalação **Zig 0.15.2**;
`Ast.parse`, `builtinCallParams`, nodeTag/nodeMainToken/firstToken/lastToken,
`std.zig.string_literal.parseAlloc`. API versionada, não suposta estável.
Não existe dependência de exportação de grafo semântico do compilador.

Helper Zig parseia um arquivo e relata chamadas AST de @import, @cImport e
@embedFile, strings literais e spans/contextos de controle. Orquestrador Python
recebe raiz e mapa JSON explícito e gera grafo/metadata. Nunca compila/executa
projeto ou build.zig; compilação explícita prepara somente o helper conhecido.
Não instala ferramentas. Sem cache de extração. Cache interno inevitável da
compilação do helper fica temporário nesta pasta e é removido no preparo.

Grafo é de candidatos sintáticos: todos os imports têm ativação desconhecida,
mesmo fora de condições (declarações Zig são lazy). if/switch/loops/comptime,
funções/testes/short-circuit são anotados por contenção de spans de nós AST.
Isso é sobreaproximação de contexto, não avaliação de ramos/comptime.
Imports literais sob condições são incluídos como candidatos e diagnosticados.
Expressões não literais não são avaliadas e não produzem aresta.

std/builtin são externos excluídos; root aponta para a raiz explícita. Mapa de
módulos é global, fornecido pelo usuário: string de caminho ou {path,generated}.
Caminhos do mapa são relativos ao arquivo JSON do mapa; imports de arquivo são
relativos ao arquivo importador. Mapas por módulo, limites de pacote e opções
(target/optimize/features) não são inferidos. build deve ser descrito pelo mapa.
Arquivos .zig existentes são percorridos; .zon e @embedFile são fronteiras de
dados diagnosticadas, sem parsing de seus conteúdos. @cImport é fronteira C
externa, sem clang, headers nem nós sintéticos. Módulos gerados ausentes são
identificados quando alcançados se explicitamente marcados generated.

Nós kind=mod, visibility=priv de compatibilidade (sem inferência de exports),
arestas uses/import. IDs e paths ordenados e determinísticos; endpoints coerentes.
Metadados separados: provedor/versão/configuração/capacidades/limites,
completude sintática e semântica não avaliada, ocorrências e diagnósticos,
SHA-256 dos bytes do grafo. Erro da raiz não gera grafo; erro de dependência
preserva resultado parcial explicitamente marcado. Nunca inventar nó ausente.

Testes: fixtures de relativo, nomeado, std/builtin, repetido, ausente/gerado,
condicional comptime, C; comentários e strings não viram imports; sintaxe,
expressões, escapes, determinismo, ciclos e integração via harness Cargo isolado
chamando desserializar_grafo e algoritmos atuais, sem mudanças no principal.
