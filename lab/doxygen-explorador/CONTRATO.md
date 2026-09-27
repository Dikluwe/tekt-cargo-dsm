# Explorador documental experimental — contrato local v1

Este workspace independente consome somente JSON do intercâmbio v1:
`lente.doxygen.inventory`, `lente.symbol_catalog`, `lente.doxygen.matches`.
Segue a spec da Lente e ADRs 0007/0008: documentação é enriquecimento opcional,
não provedor estrutural, prova de completude ou equivalência semântica.

Antes de abrir a porta, lê os três arquivos uma vez, valida tipos, versões,
IDs únicos, endpoints documentais, referências de diagnósticos e correspondências
exatamente uma por nó. Confere SHA-256 dos bytes originais de inventário e
catálogo. O graph_digest é apenas declaração: não há arquivo de grafo nesta
entrada para verificá-lo. Hashes não autenticam o produtor.

Matched exige document_id existente e presente nos candidates; ambiguous exige
candidatos e nenhum document_id; unmatched exige nenhum candidato/document_id.
As listas unmatched devem corresponder aos nós sem associação confirmada e aos
documentos não associados de forma confirmada, inclusive os ambíguos. IDs são
locais ao snapshot. Não se infere associação nem se normalizam nomes, paths ou
assinaturas. Opcionais ausentes permanecem desconhecidos. Coleções apresentadas
são ordenadas por ID, evidências e diagnósticos por texto. Campos extras são
aceitos como extensões, sem interpretá-los como fatos.

HTTP mínimo somente GET, rotas exatas /, /app.js, /style.css, /api/data.
Bind fixo 127.0.0.1 com --port explícita (1..65535). Assets embutidos; nenhuma
rota lê arquivos. Entradas permanecem intactas e não há recarga automática,
cache persistente, subprocessos, Doxygen, download ou CDN. Limite de cabeçalho
8 KiB e timeout de leitura de 2 segundos; servidor sequencial para inspeção
local, não serviço de produção. Encerramento por Ctrl-C.

Documentação é inserida exclusivamente com textContent. Candidatos aparecem
como identidades não confirmadas, nunca como documentação associada. Links
exigem http:// ou https:// minúsculo, URL ASCII sem espaços, controles,
barra invertida ou credenciais, host DNS/IPv4 ASCII e porta numérica válida
opcional. IPv6/IDN não são habilitados nesta versão. Abertura somente por clique,
em outra aba com noopener/noreferrer; URL recusada aparece como texto.
CSP bloqueia recursos externos. Busca é filtro visual, não correspondência.
IDs numéricos são convertidos em strings na API de apresentação para evitar
perda de precisão JavaScript. Cobertura é a declaração do inventário; complete
refere-se somente à entrada declarada, nunca à semântica integral do programa.

Testes locais cobrem integridade, estados, Unicode, documentação vazia, URLs,
HTTP real em loopback, determinismo e erros. Inspeção visual será registrada
separadamente com seus limites. Nenhuma alteração fora deste diretório.

## Revisão de integração (ADR-0009, prevalece sobre v1 acima)

Inventário v2 exige pending_relations [{from,to,kind,reason}], com reason
missing_from/missing_to/missing_both verificado. relations só contém endpoints
resolvidos. Pendências exigem partial; nunca geram identidade ou dependência.
Consumidores aceitam v1 resolvido; v1 pendente exige migração explícita, sem
alterar bytes para calcular digests. IDs duplicados são erro. Diagnósticos de
IDs pendentes são contexto, não símbolos fictícios. Catálogo aceita diagnostics
opcional. Unmatched pode ter candidatos para inspeção, sem documento confirmado.
Prioridade emite priority_report v2, preservando pending_relations; explorador
expõe ambas as listas e modo finito --output, sem abrir servidor. Publicações
em arquivo usam temporário irmão + hard link exclusivo (Linux), sem sobrescrita
ou promessa de durabilidade do nome após queda de energia.
