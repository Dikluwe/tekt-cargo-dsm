# Contrato local — avaliação independente v1

Origem: pedido de avaliação reproduzível dos seis adaptadores, 2026-09-26.
Leituras: AGENTS.md raiz; READMEs TypeScript, Python, C, C++, Go, Zig;
CONTRATO.md TypeScript/C/Go/Zig; README é contrato Python/C++;
spec forma-organizada, desserializar_grafo e calcular_raio atuais.

Somente este diretório pode mudar. Workspace Cargo próprio; bibliotecas locais
existentes de leitura do grafo e raio são reutilizadas. Nenhuma dependência do
pacote de análise paralelo ou alteração no harness/adaptadores.

Cenários JSON versionados definem argv (nunca shell), cwd, ambiente controlado,
requisitos/versões, preparação, artefatos, expectativas e timeout. Arquivos de
preparação usam templates de paths e JSON, sem executar build ou descobrir
comandos em manifests do projeto. Cenários são código confiável revisável:
o executor não é sandbox para um comando arbitrário aprovado no JSON.

Etapas independentes: requisitos, preparação, comando do adaptador, validação
posterior pelo leitor real, análise posterior de raios. Não atribuir tempo a
subetapas internas do adaptador. Todas as amostras e códigos originais persistem.
Status: aprovado, reprovado, ferramenta_ausente, nao_suportado. Parcial é esperado
quando declarado; não confundir resultado semântico parcial com falha do teste.

IDs/endpoints/enums são validados como forma crua; paths repetidos impedem a
análise de raio, não tornam a forma crua inválida. Fixtures analíticas usam paths
e nomes únicos. Comparações só explícitas, com granularidade, semântica de
relação e universo compatíveis, e identidade/correspondência declarada.

Execuções criam diretórios exclusivos, com logs de processo em arquivos para
não bloquear pipes. Unix usa grupo de processos e SIGKILL no timeout, seguido de
wait no filho direto. Descendentes que escapam do grupo não são contidos: não é
sandbox. Plataformas sem esse mecanismo ficam não suportadas, sem executar.

Repetições: primeira amostra identificada (não alegar cache frio), subsequentes
separadas, mediana/MAD/mínimo/máximo das aprovadas; falhas não entram como tempo
zero. Sem downloads/preparo automático de ferramentas. Disponibilidade diferente
não gera resultado inventado. Fixtures pequenas não sustentam promessa de latência.
