# Associação documental conservadora — contrato local v1

Registrado antes da implementação. Escopo exclusivo: lab/doxygen-correspondencia.
Workspace Cargo independente, sem crates de produção ou de outros experimentos.
Comunicação somente por JSON. Não lê fontes, executa Doxygen/compilador ou instala
ferramentas. Referências lidas: AGENTS.md, spec forma-organizada, ADR-0007/0008,
docs/adaptadores.md e docs/analises-artefatos.md. Enriquecimento documental não
substitui provedores e não comprova equivalência semântica.

## Interface e formatos

`lente-doxygen-correspondencia --inventory INVENTARIO.json --catalog CATALOGO.json
--output CORRESPONDENCIAS.json`

Entradas: lente.doxygen.inventory v1 e lente.symbol_catalog v1 conforme contrato
comum da rodada. Saída: lente.doxygen.matches v1, com os campos comuns, uma entrada
por node_id. Campos opcionais (qualified_name, signature, location, documentation_url,
producer.version/configuration_digest, diagnostic.symbol_id) ausentes equivalem a
null, nunca a fatos inferidos. Demais campos são obrigatórios e tipados. Campos
extras são recusados nesta versão. kind e relation.kind continuam strings opacas;
brief/details são strings de texto simples, não interpretadas como HTML.

Erros de schema/versão, JSON, tipos, IDs duplicados (mesmo com registros idênticos),
IDs documentais vazios, localização inválida, digest de grafo malformado ou relações
com endpoints desconhecidos interrompem a operação sem produzir relatório válido.
node_id é inteiro u64. Linha é inteiro positivo; arquivo de localização não vazio.
IDs documentais são strings locais ao inventário. Não conferem identidade entre
versões. graph_digest deve ter prefixo sha256: e 64 hexadecimais; não se verifica
o grafo, pois não é uma entrada desta tarefa.

Lê cada arquivo uma vez em bytes; os mesmos bytes são usados para SHA-256 e parse.
Digests da saída são dos bytes recebidos, incluindo whitespace/ordem, não do JSON
resserializado. Não autenticam produtor, nem provam coerência do catálogo com fontes.
Saída é criada exclusivamente: arquivo existente não é sobrescrito, inclusive os
arquivos de entrada. Falhas retornam exit 1 e diagnóstico stderr; --help retorna 0.
Não cria diretórios de saída automaticamente. Sem cache ou relógio no resultado.

## Política

1. Só examina documentos com qualified_name exatamente igual e não branco ao nó.
   Nome simples, IDs, kind, texto documental e similaridade não são evidências.
2. Um candidato é suficiente quando há nome qualificado exato e:
   localização exata (file e line); OU assinatura exata e não branca.
3. Se ambos declaram signature (não null) e as strings diferem, há veto mesmo com
   localização igual. String vazia nunca fornece evidência positiva; duas strings
   diferentes, ainda que uma seja vazia, são conflito conservador.
4. Nenhuma normalização: nem separadores, caminhos relativos/absolutos, Unicode,
   caixa, espaços, aliases, nem assinaturas C++. Testar se é branco não altera a
   string usada na igualdade. Localização diferente não veta a regra por assinatura,
   mas sempre produz evidência/diagnóstico da divergência.
5. Com um candidato suficiente: matched provisório; com vários: ambiguous.
   candidates contém os suficientes quando existem; caso contrário, os de mesmo
   nome qualificado apenas para inspeção, com status unmatched.
6. Política global 1:1 conservadora: documento suficiente para mais de um node_id
   gera conflito. Todos os nós que o reivindicam ficam ambiguous, inclusive um
   nó com candidato único que conflita com candidato de outro nó ambíguo.
   Não resolve atribuições globais nem seleciona outro candidato para esconder
   conflito. Só matched possui document_id. IDs duplicados são erro, não dedup.
7. evidence lista fatos comuns a TODOS os candidatos publicados, ordenados. Para
   matched, são os fatos do documento escolhido. Flags positivas: location_exact,
   qualified_name_exact, signature_exact. Negativas: signature_conflict,
   location_missing, location_difference, file_difference, line_difference,
   signature_missing_or_blank. Conflito global adiciona document_claim_conflict.
   Sem candidatos: qualified_name_missing ou no_qualified_name_candidate.
8. Cada candidato de mesmo nome, inclusive vetado, tem diagnóstico
   candidate_evaluation com node_id/document_id, valores originais comparados,
   facts e sufficient em JSON dentro da mensagem textual. Assim a lista plana
   evidence não finge que a união de evidências de documentos diferentes sustenta
   um candidato. Diagnósticos node_unmatched, node_ambiguous e document_conflict
   explicitam o motivo; IDs do nó constam da mensagem, pois o contrato comum
   só permite symbol_id documental.

unmatched_node_ids inclui TODO nó sem matched, inclusive ambiguous.
unmatched_document_ids inclui TODO documento não escolhido por matched, inclusive
candidatos ambíguos. São resíduos de associação automática, não falta de documentação.

matches ordenado por node_id; candidates/document IDs lexicograficamente; resíduos
numéricos/textuais; evidence lexicográfica; diagnostics por code/symbol_id/message.
Diagnósticos de entrada são conservados com contexto inventory_diagnostic; produtor
e coverage são declarados em inventory_context. Não se deduplicam diagnósticos.
coverage complete só significa cobertura declarada do conjunto de entrada e jamais
é inferida de execução bem-sucedida. Igual entrada em bytes gera saída igual.
Reordenar as coleções mantém decisões, mas muda seus digests por definição.

## Verificação planejada

Localização+nome; assinatura+nome; overloads; namespaces; múltiplos candidatos;
assinatura divergente; localização ausente; nome isolado; zero candidatos;
conflito global; IDs duplicados em ambos os arquivos; versões inválidas; campos
opcionais ausentes; sem normalização; permutação; digests dos bytes exatos;
CLI real e preservação de saída preexistente. Fixtures locais independentes.

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
