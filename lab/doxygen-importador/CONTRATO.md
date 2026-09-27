# Importador documental Doxygen — contrato local v1

Linhagem: pedido de 2026-09-26; AGENTS.md, spec forma-organizada, ADR-0007/0008,
docs/adaptadores.md e docs/analises-artefatos.md. Só esta Arena. Nenhuma dependência
Cargo de componentes da Lente ou dos outros experimentos. Intercâmbio só JSON.

## Referência e perfil suportado

Manual oficial consultado: https://www.doxygen.nl/manual/customize.html,
https://www.doxygen.nl/manual/starting.html. A distribuição XML separa índice e
compounds. Referência versionada consultada:
https://raw.githubusercontent.com/doxygen/doxygen/Release_1_9_8/templates/xml/compound.xsd
(index.xsd da mesma distribuição é indicado pelo manual; não foi obtido pelo
navegador). Fixtures são manuscritas locais baseadas nessa estrutura, NÃO saída
real de Doxygen executado nesta tarefa. Não validar XSD nem buscar schemaLocation.

Perfil experimental: raízes doxygenindex/doxygen, version="1.9.8" quando presente.
Outra versão explícita, namespace de elementos ou raiz é erro incompatível.
Versão ausente aceita estrutura do perfil, producer.version=null no índice e
aviso producer_version_unknown; não inferir versão de executável ou ambiente.
O subconjunto não promete compatibilidade com versões futuras ou todo o XSD.

## Interface e JSON

`lente-doxygen-importador --xml-dir DIR --output INVENTARIO.json`.
Ler somente index.xml e arquivos <compound refid>.xml citados pelo índice.
Não ler fontes, HTML, XSD, Doxyfile, configurações, URLs, refs transitivas ou
executar Doxygen. configuration_digest e documentation_url=null: não deduzir.
Saída exata do contrato lente.doxygen.inventory, version=1, produtor name=doxygen.
Não produz grafo estrutural, catálogo ou matches. Não calcula digest de JSON
resserializado: não há campo de digest de inventário neste contrato. Consumidores
devem hashear os bytes recebidos quando construírem correspondências.

compounddef e memberdef geram símbolos; id/kind preservados literalmente.
compound.name vem de compoundname; member.name vem de name. qualified_name vem
somente de qualifiedname explícito (não reconstruir namespace). signature guarda
argsstring verbatim: assinatura de argumentos/qualificadores, NÃO assinatura
completa com retorno. Ausente => null, sem reconstrução por param/definition.
location requer file e line inteiro positivo; incompleta => null e diagnóstico.
Documentação ausente => strings vazias, não uma perda inferida. brief/details vêm
dos respectivos elementos, com inbodydescription anexado a details quando houver.
IDs são locais; nomes de sobrecargas podem coincidir, seus IDs não são fundidos.

Relações explícitas suportadas: basecompoundref, derivedcompoundref, includes,
includedby, innerclass, innernamespace, innerfile, innerdir, innerpage, innergroup,
innerconcept, innermodule, references, referencedby, reimplements, reimplementedby,
ref. from é o símbolo contenedor mais próximo; to é refid; kind é o nome original
do elemento XML. referencedby NÃO é invertido nem traduzido para chamada.
Refs documentais/tipos são ref, não Uses. Sem refid: diagnóstico, sem endpoint
inventado. Refs explicitamente externas não são convertidas em relações locais.
Alvo não resolvido permanece como refid na relação com unresolved_reference;
nenhum símbolo fantasma é criado. Inventário documental pode conter refs pendentes.

Referências do índice/member e listofallmembers/sectiondef member são verificadas,
mas não geram símbolos substitutos. Repetição de referência não é definição
repetida. Definições duplicadas idênticas: deduplicar com diagnóstico; conflitantes:
quarentena do ID (omitir todas as versões), diagnóstico e coverage partial.
Declaração de kind/name divergente entre índice e definição também é diagnosticada.
Nenhuma escolha first-wins. Coleções ordenadas e relações/diagnósticos deduplicados.

## Perdas, texto e cobertura

Sem declaração externa de escopo suficiente, coverage=unknown; esta interface
não tem opção para complete. scope_unknown sempre explica a limitação. Perdas
conhecidas elevam para partial: arquivo referenciado ausente, ID conflitante,
referência não resolvida, estrutura/elemento não suportado, localização incompleta.
XML malformado, recurso excedido, caminho proibido e versão incompatível abortam
sem publicar. Índice ausente é erro fatal distinto de compound referenciado ausente.
Um índice válido vazio produz inventário vazio unknown, nunca sucesso completo.

Converter parágrafos/listas/seções/linhas de código em texto com separadores;
ênfase/ref/ulink viram conteúdo textual, sem tags/URLs ativas. HTML-only, script,
style, iframe, object, SVG e diagramas opacos são omitidos com diagnóstico.
Marcação desconhecida não ativa conserva seu texto, com diagnóstico. Texto literal
como <T> continua texto: consumidores DEVEM escapar ao renderizar HTML. Não usar
brief/details como HTML. Nomes, assinaturas e caminhos não são normalizados além
da decodificação XML obrigatória. Whitespace documental pode ser formatado.

Atributos como visibilidade/static/const não têm campo no inventário e ficam fora
desta projeção. type/definition/param/qualifier não são assinatura sintetizada;
refs internas são preservadas. Documentação de parâmetros fora de brief/details,
enumvalue, listings de compound, graphs e extensões recebem unsupported_element
quando contêm informação fora do perfil. Não fingir suporte semântico ao XML.

## Leitura restrita e recursos

Plataforma suportada: Linux, filesystem local. Abrir raiz canonicalizada e fixar
seu descritor de diretório. Leitura openat de nomes simples com O_NOFOLLOW e
O_NONBLOCK; recusar symlinks (mesmo internos), subdiretórios, caminhos absolutos,
separadores e arquivos não regulares. O caminho da raiz pode ser um symlink
explicitamente fornecido pelo usuário: a raiz autorizada é seu destino canônico.
Nenhum endpoint/documentation location autoriza leitura. Diretório de entrada
não deve ser alterado durante a importação; não há snapshot transacional.

roxmltree 0.21.1, allow_dtd=false, entity_resolver=None; recusar ainda marcadores
<!DOCTYPE/<!ENTITY antes do parser (inclusive em comentários/CDATA, política
conservadora). Nenhuma biblioteca de rede/resolução de entidades, XInclude ou XSLT.
Limites fixos: 4 MiB por XML, 32 MiB totais, 1024 compounds, 100.000 nós XML por
arquivo, profundidade 128, 50.000 definições, 100.000 relações, 20.000 diagnósticos,
64 KiB por campo de texto/atributo, 512 bytes por id/refid e 16 MiB JSON final. Exceder limite é erro, não truncamento.
Uma árvore XML por vez; nenhum cache persistente. Sem limite de CPU/wall-clock.

## Publicação

Nunca substituir saída existente, mesmo vazia ou symlink. Gerar JSON integral,
validar contrato em memória, escrever temp irmão exclusivo modo 0600, sync_all,
então hard_link exclusivo no nome final e remover temp. Hard link publica bytes
completos atomicamente e falha se destino existir, inclusive sob concorrência.
Falha normal limpa temp. Queda abrupta pode deixar temp, não JSON final truncado.
Filesystem deve suportar hard links locais; sem fallback inseguro. Sem fsync do
diretório, não prometer durabilidade do nome após queda de energia. Um resultado
partial é publicação deliberada SOMENTE em caminho novo, com exit 2. Unknown:
exit 0; fatal: exit 1. Diagnóstico fatal estruturado em stderr, sem novo inventário.

## Verificação

Testes inline e fixtures pequenas: funções/classes/namespaces, sobrecargas,
documentação ausente/Unicode, relações pendentes, XML inválido/arquivo ausente,
duplicatas, marcação textual, cobertura, determinismo, limites, DTD/symlink,
publicação sem substituição e concorrência. Nenhum teste executa Doxygen.

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
