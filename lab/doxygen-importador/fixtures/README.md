# Origem das fixtures

`basico/*.xml` foram escritos manualmente para este experimento, com base na
organização descrita no manual oficial Doxygen e no `compound.xsd` 1.9.8:

- https://www.doxygen.nl/manual/starting.html
- https://www.doxygen.nl/manual/customize.html
- https://raw.githubusercontent.com/doxygen/doxygen/Release_1_9_8/templates/xml/compound.xsd

Não são saída de uma execução do Doxygen, nem foram validados contra todo o XSD.
São exemplos mínimos do subconjunto documentado em `../CONTRATO.md`.

O índice referencia um namespace e uma classe. O namespace contém duas funções
sobrecarregadas, com IDs distintos, documentação Unicode e localizações fictícias.
A segunda sobrecarga não tem documentação. Há relações `innerclass`, `references`
e `ref`. Os caminhos de código-fonte são dados; nenhum arquivo fonte é necessário
ou lido. O resultado esperado tem quatro símbolos, três relações e cobertura
`unknown`, com o diagnóstico `scope_unknown`.

Casos de erro e perdas são construídos pelos testes inline em `src/main.rs`, em
diretórios temporários locais ao workspace, sem downloads nem programas externos.
