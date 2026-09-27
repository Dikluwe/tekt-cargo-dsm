# Painel opcional documental (ADR-0009)

Contrato prévio à alteração de serve.py e web/app.js/index.html. Contrato
estrutural original permanece no README. Não modifica células/pesos/raio.

Inicialização explícita recebe --graph (bytes originais), --inventory, --catalog,
--matches e --explorer (executável confiável). Tudo ou nada. Compara SHA-256 do
grafo ao catálogo, IDs exatos do catálogo aos nós, e grafo JSON ao source_graph.
Reprepara bundle com binário DSM e compara campos estruturais para impedir uma
matriz divergente da fonte. Explorador real --output valida os três artefatos
congelados em diretório temporário; timeout 30s, sem shell/servidor implícito.
Se qualquer vínculo falha, não abre porta nem habilita painel. Hashes não
são autenticação; entradas devem ser confiáveis. O caminho da ferramenta é
configuração do operador, nunca uma rota/argumento fornecido pelo navegador.

Seleção por node_id expõe estado, documento confirmado, candidatos separados,
evidências, cobertura, diagnósticos e referências pendentes relevantes. Pendências
não são links navegáveis. Texto via textContent; sem URLs automáticas ou HTML.
Modo sem documentação conserva interface e APIs existentes. Somente loopback.
