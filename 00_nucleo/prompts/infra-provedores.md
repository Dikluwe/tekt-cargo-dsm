# Contrato — ferramentas de extração opcionais
Hash do Código: b34db8d1

Unidade `03_infra/src/provedores.rs`, L3; ADR-0007 e ADR-0008.
Implementa ExtratorGrafo para solicitação Ferramenta. O envelope L1 usa Json
como identidade do transporte; a identidade semântica real, metadados originais
e relatório do verificador permanecem no contexto L3 obrigatório entregue ao L4.
Não modifica L1 nem importa lab. Configuração JSON estrita, por provedor, caminhos
absolutos; rejeita campos desconhecidos e combinações antes de iniciar processos.

Provedores: python, typescript, zig, c, cpp, go e pacote-analise. Ferramentas
externas opcionais fornecidas explicitamente, sem instalação, shell ou cache.
Suporte de execução Unix: grupo de processos por comando; prazo total monotônico;
SIGKILL via /bin/kill e wait no líder, também limpa descendentes após saída do
líder. Não cobre ferramentas que deliberadamente escapem do grupo (setsid).
Saída capturada em arquivos privados por chamada, sem deadlock de pipes.

Cada adaptador conserva seu protocolo e código de saída. C exige índice de
variante explícito, conserva manifesto completo, nunca une TUs/configurações.
C++ escolhe configuração pelo hash quando solicitado; a validação semântica da
base é do próprio adaptador. Go exige alvo e política de testes/workspace.
Metadados originais acompanham resultados inclusive parciais.

Pacote: delega create e verify ao executável da tarefa 13. Não implementa seu
manifesto/hash/resumo de novo. create mantém bytes dos metadados; sem vínculo
(C/Go) exige opt-in explícito. Ambiente também exige opt-in explícito, sem redação.
Entrada pacote verifica antes de ler/calcular; diretório deve permanecer estável
(conforme contrato da ferramenta). Interpretação de cobertura usa exclusivamente
summary verificado; estados ausentes ficam desconhecidos, completos não implicam
cobertura semântica completa. Metadados e capacidades originais não são descartados.
Erros distinguem parâmetros, indisponibilidade, execução, timeout e saída inválida.

## Interface Snapshot

O executor é reutilizado internamente por `analises` para comandos conhecidos
de artefatos, com os mesmos prazos, isolamento temporário e encerramento de grupo.
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"executar_artefato","params":["&Path","&[OsString]","u64","&[i32]"],"return_type":"Result<(i32, Value, String), ErroFerramenta>"}],"types":[{"name":"ConfiguracaoFerramenta","kind":"struct","members":["executavel","helper","verificador","timeout_ms","permitir_metadados_sem_vinculo","incluir_ambiente","parametros"]},{"name":"PedidoFerramenta","kind":"struct","members":["provedor","configuracao"]},{"name":"ErroFerramenta","kind":"enum","members":["Parametros","Indisponivel","Execucao","Timeout","SaidaInvalida"]},{"name":"ContextoFerramenta","kind":"struct","members":["dados"]},{"name":"ExtratorFerramenta","kind":"struct","members":["contexto"]}],"reexports":[]} -->
