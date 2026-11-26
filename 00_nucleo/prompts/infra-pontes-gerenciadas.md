# Pontes de linguagem para a Lente FFI (L3)
Hash do Código: ffa8442b

As pontes recebem o texto dos documentos Python e Java do índice.
O extrator Python usa a AST de python3 sem importar ou executar o
projeto. Reconhece CDLL importado de ctypes com caminho literal
e a chamada direta de atributo do objeto carregado. A biblioteca
precisa corresponder ao arquivo fornecido. Ligações reatribuídas,
getattr e nomes de biblioteca calculados ficam fora do recorte.

O extrator Java usa o compilador do JDK com processamento de anotações
desativado, sem carregar ou executar as classes analisadas. Resolve
chamadas a métodos static native sem sobrecarga de outros métodos
nativos de mesmo nome e constata a carga literal de biblioteca na
classe por java.lang.System.load ou loadLibrary. Usa o nome JNI
curto escapado. Exige coincidência da biblioteca, símbolo dinâmico
exportado e posição ELF/DWARF. Registro dinâmico fica fora do recorte.

O extrator adicional lê Go, C#, TypeScript, JavaScript, Kotlin/JVM e
Dart e Zig. Usa binários vinculados para cgo, Zig e addons Node-API e símbolos
dinâmicos para as demais pontes. C# admite o nome de arquivo .so ou
a convenção libnome.so. Node-API exige que o callback confirmado por
DWARF corresponda ao arquivo que contém o registro candidato.

Os extratores devolvem linhas e colunas UTF-8 dos identificadores
e das funções chamadoras, mais a declaração JNI quando existente.
Ligações não confirmadas preservam as referências originais do SCIP.

```
Dado CDLL literal de outra biblioteca Então não ligar ao ELF fornecido.
Dada chamada Java de método native carregado em outra biblioteca Então não ligar ao ELF fornecido.
Dada carga literal e exportação confirmada Então registrar o local da chamada na fronteira correta.
```
