# Extração das pontes FFI por convenção de fonte (L3)
Hash do Código: 70340355

Recebe os documentos SCIP como texto, sem importar ou executar o projeto.
Extrai candidatos com posições UTF-8. Emite as declarações C# e Kotlin
para confirmação pela identidade SCIP, inclusive chamadas em outros arquivos.

- Go: import cgo sem alias, uso direto `C.nome(...)` e wrapper de cgo no ELF.
- Zig: `extern fn` de topo com ABI C e seleção direta de função num alias
  `const` de `@cImport` com `@cInclude` literal. Conserva a posição
  do callee e a declaração estrangeira quando presente. Declarações
  externas com biblioteca nomeada e aliases sombreados ficam fora do recorte.
- C#: DllImport ou LibraryImport de Runtime.InteropServices, biblioteca
  literal e EntryPoint literal opcional, sem sufixos de busca dinâmica.
- Kotlin/JVM: external fun em objeto com carga literal no inicializador
  e nome JNI curto sem sobrecarga.
- JavaScript/TypeScript: const com Deno.dlopen de caminho literal e
  chamada lib.symbols.nome, com nome nativo literal opcional no descritor.
- Dart: import de dart:ffi, DynamicLibrary.open literal e alias final
  de lookupFunction com tipo explícito e nome nativo literal.
- Node-API: require de caminho literal .node e exportação direta
  por NAPI_MODULE_INIT, napi_create_function e napi_set_named_property,
  verificando a identidade do arquivo nativo do callback e o export
  do inicializador. A correspondência exige um único registro candidato.

Escopos ambíguos, ligações reatribuídas, sobrecargas Kotlin, símbolos
calculados e outras formas de registrar callbacks ficam fora do recorte.
Chamadas sem evidência suficiente preservam a navegação SCIP habitual.
