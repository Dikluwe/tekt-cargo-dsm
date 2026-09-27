# Contrato local — pacote de análise v1

Origem: pedido de 2026-09-26; AGENTS.md, spec forma-organizada, ADR-0002,
contratos dos seis adaptadores experimentais e envelope `ExtracaoGrafo`.
Este workspace não altera nem integra esses componentes.

## Manifesto e confiança

`manifest.json`: format `lente-analysis-package`, version inteiro 1; inventário
fechado de artefatos (role graph/metadata/evidence, path relativo simples,
sha256 hexadecimal minúsculo dos bytes, size); summary com fatos opcionais e
ponteiros JSON de origem; source_binding; privacy. Null significa desconhecido.
Exatamente um graph, zero ou um metadata, zero ou mais evidence. Arquivos extras,
symlinks, caminhos externos, roles/versões desconhecidas são recusados.
Metadados são JSON objeto e preservados byte a byte; evidências são bytes opacos.
Hashes não autenticam o produtor: quem reescreve manifesto e artefatos pode
forjar outro pacote. Distribuir o hash do manifesto por canal confiável permite
usar `verify --manifest-sha256 HEX` para ancorar a integridade externa.

`source_binding=graph-digest` somente se /graph_sha256 dos metadados corresponde
aos bytes do grafo. Digest divergente ou malformado é sempre erro. Isso vincula
conteúdo, NÃO prova mesma execução quando execuções distintas produzem os mesmos
bytes. Sem identificador de execução comum, identidade da execução é desconhecida.
Metadados sem digest exigem `--allow-unbound-metadata`: binding unknown, nunca
promovido a verificado. Ausência de metadados: binding unknown e fatos null.
A verificação repete o vínculo e a derivação do summary, além de todos os hashes.
`verify --require-bound` recusa unknown. Evidência tem associação de origem
unknown: o manifesto apenas associa seus bytes ao snapshot do pacote.

## Normalização conservadora

Provider, provider_version e granularity vêm dos campos homônimos strings.
Versões de ferramentas NÃO substituem versão do provedor. Ferramentas conhecidas:
TS typescript_version, Zig zig_version, C version, C++ clang_version.
Interpretar semântica apenas dos provedores conhecidos nos contratos consultados:

| Provedor | Estado da execução | Cobertura declarada | Configuração identificada |
|---|---|---|---|
| typescript-compiler-api-experimental | desconhecido | /coverage (escopo do adaptador) | /project (localizador apenas) |
| zig-ast-experimental | desconhecido | /completeness/semantics; sintaxe separada | /configuration |
| experimental-python-ast | /execution/status | /semantic_coverage | /roots |
| experimental-libclang-cpp | /execution/status | /semantic_coverage | /selected_configuration |
| experimental-go-list | /loading (carregamento) | /semantic_completeness | /configuration |
| libclang | /status (TU) | desconhecido | /configuration |

Mapeamentos exigem schema_version=1 (TS/Zig), metadata_version=1 (Python/C++)
ou contract_version=1 (Go). C ainda tem contrato sem versão de schema. Versões
novas/ausentes mantêm a interpretação semântica desconhecida e os bytes intactos.

Estado de execução aceita complete/partial/error/failed; demais ficam null.
Completo nunca implica cobertura semântica completa. TS coverage e Zig syntax
ficam em scope_coverage, separados de semantic_coverage. Configuração é digest
do fragmento JSON declarado (serialização compacta serde_json, chaves ordenadas),
com ponteiro e escopo, NÃO fingerprint dos arquivos referidos nem reprodução.
Não ler configurações, arquivos referidos, ambiente atual ou executar subprocessos.

## Privacidade

Não copiar valores de configuração/ambiente para summary ou saída do terminal.
Metadados contendo valores não nulos sob chaves env/environment/go_env/
environment_policy/variables/vars (ou nomes contendo env) exigem
`--include-environment-values`, registrado no manifesto. Nunca redigir o original.
Detecção por nomes é conservadora, NÃO detector de segredos; revisar manualmente
metadados e evidências antes de distribuir (evidências são opacas, opt-in explícito).

## Publicação

Somente Linux com filesystem local suportando renameat2(RENAME_NOREPLACE).
Criar staging irmão exclusivo modo 0700, escrever cópias e manifesto com create_new,
validar TODO o staging com o leitor real da Lente, então renameat2 sem substituição.
Nenhum fallback check-then-rename. Duas criações concorrentes têm no máximo um
vencedor. Falhas normais removem staging; interrupção abrupta pode deixar staging
oculto, nunca diretório final parcial. Sem fsync: não há promessa de durabilidade
após queda de energia. Não se suporta diretório sendo alterado por terceiros durante
criação/verificação, filesystem remoto ou verificação como sandbox contra atacante.
O diretório pai deve existir e estar sob controle do usuário. Pacote pode ser movido.

Referências: https://man7.org/linux/man-pages/man2/rename.2.html e
https://man7.org/linux/man-pages/man2/fsync.2.html (consultadas 2026-09-26).
