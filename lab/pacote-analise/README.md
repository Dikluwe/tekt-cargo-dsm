# Pacote experimental de análise

Workspace Cargo independente. Só lê arquivos existentes. Não executa Cargo,
compiladores, adaptadores ou código analisado durante create/verify. Nenhum cache,
integração com CLI/MCP, alteração do núcleo ou exigência sobre outros laboratórios.
O contrato normativo local é [CONTRATO.md](CONTRATO.md).

## Preparação e uso

Requisitos: Linux e filesystem local com renameat2(RENAME_NOREPLACE), Rust ≥1.91
para as dependências locais da Lente. Build abaixo usa dependências previamente
obtidas e o lockfile local; não instala ferramentas nem baixa dependências.
Execute a partir da raiz do repositório:

```sh
cargo build --offline --locked --manifest-path lab/pacote-analise/Cargo.toml
mkdir -p lab/pacote-analise/out
lab/pacote-analise/target/debug/lente-pacote-analise create \
  --graph lab/pacote-analise/fixtures/graph.json \
  --metadata lab/pacote-analise/fixtures/metadata.json \
  --evidence lab/pacote-analise/fixtures/README.md \
  --out lab/pacote-analise/out/exemplo
lab/pacote-analise/target/debug/lente-pacote-analise verify \
  lab/pacote-analise/out/exemplo --require-bound
mv lab/pacote-analise/out/exemplo lab/pacote-analise/out/movido
lab/pacote-analise/target/debug/lente-pacote-analise verify \
  lab/pacote-analise/out/movido --require-bound
```

O diretório pai deve existir. Repetir create para a mesma saída falha, mesmo se
ela for um diretório vazio. `--evidence` é repetível; metadados e evidências são
opcionais. Exit 0 = integridade verificada, 1 = erro; nenhuma execução completa
ou cobertura semântica é inferida do exit code. stdout JSON inclui integrity,
source_binding, evidence_source_binding e summary. Erros ficam em stderr.

`verify --manifest-sha256 HEX` compara o manifesto com um hash recebido por
canal independente confiável. Sem essa âncora, hashes detectam corrupção/trocas,
mas não autenticam origem nem impedem uma pessoa de reescrever todo o pacote.

## Conteúdo e significado

```
pacote/
  manifest.json
  graph.json
  metadata.json        # opcional, bytes originais
  evidence-0000.bin    # opcionais, bytes originais
```

Manifesto v1 tem inventário com role, path, size e SHA-256 de cada artefato,
identidade/versionamento do provedor, versão conhecida da ferramenta, granularidade,
estado de execução, coberturas separadas, identificação da configuração e política
de privacidade. O próprio manifesto não se inclui recursivamente no inventário.
Campos desconhecidos do resumo são null; cada fato conhecido indica seu ponteiro
nos metadados. Diagnósticos, capacidades e extensões permanecem no original.
Configuração identificada é hash do fragmento declarado; nenhum valor completo
é repetido no resumo. Arquivos referidos pela configuração não são lidos.

Exemplo de distinção: Python pode declarar execution=complete e
semantic_coverage=partial. TS declara cobertura dentro de seu escopo, mas não um
estado de execução independente: execution e semantic_coverage ficam null.
Zig declara semântica not-evaluated. Go loading=complete só significa carregamento.
C++/Python não fornecem provider_version: clang_version e metadata_version não
são substitutos. C version é versão do libclang, registrada como tool_version.
O envelope L1 atual contém grafo, provedor e diagnósticos: por si só não fornece
identidade de execução, configuração ou cobertura para preencher esses campos.

## Associação entre os arquivos

- TS, Zig, Python e C++ consultados declaram graph_sha256. O digest precisa
  corresponder aos bytes exatos, inclusive espaços e newline. Divergência sempre
  impede publicação e verificação, mesmo com opção de associação desconhecida.
- C e Go consultados não declaram esse vínculo. Para guardar seus metadados,
  use `--allow-unbound-metadata`: source_binding permanece unknown. Sem a opção,
  create recusa o par. `verify --require-bound` também recusa associação unknown.
- Grafo sem metadados pode ser empacotado; provider, execution etc. são null.
- Evidências têm origem unknown: hashes fixam os bytes escolhidos pelo usuário,
  não provam que vieram da extração. O original não é alterado.

`graph-digest` prova correspondência de conteúdo, não identidade da execução.
Duas execuções que emitiram exatamente os mesmos bytes são indistinguíveis por
esse mecanismo; execution_identity permanece null. Verificar antes de consumir
todo o conjunto evita combinar artefatos de pacotes diferentes por acidente.
Versões desconhecidas dos schemas de adaptadores são preservadas sem interpretar
seus estados/configuração/cobertura. Nenhum adaptador precisa adotar este pacote.

## Privacidade

A ferramenta não lê nem captura variáveis do ambiente atual. Campos de ambiente
conhecidos ou com nomes contendo `env` e valores não nulos nos metadados bloqueiam
create por padrão; `--include-environment-values` permite preservar o original,
registrando o opt-in. Isso inclui configuration.go_env do Go e environment do C
quando preenchido. Não há redaction silenciosa nem exposição desses valores no
manifesto ou stdout. A heurística pode recusar campos inocentes e não detecta
segredos arbitrários. Revise os originais e evidências opacas antes de distribuir:
argv, diagnósticos e caminhos também podem conter informação sensível.

## Publicação e limites

A ferramenta copia para staging irmão modo 0700, valida inventário, hashes,
metadados, associação e grafo com `lente_infra::desserializar_grafo`, e só então
publica via renameat2(RENAME_NOREPLACE). Não há janela check-then-overwrite.
Falhas tratadas removem staging; encerramento abrupto pode deixar diretório oculto
`.lente-package-*`, nunca saída final parcialmente construída. Não há fallback
para sistemas sem suporte; o erro é explícito. Duas tentativas concorrentes
produzem no máximo um vencedor. Veja [rename(2)](https://man7.org/linux/man-pages/man2/rename.2.html).

Esta implementação NÃO faz fsync nem promete durabilidade em queda de energia:
a publicação atômica observável por outros processos não garante persistência.
Veja [fsync(2)](https://man7.org/linux/man-pages/man2/fsync.2.html). Somente Linux
com filesystem local; NFS, diretórios modificados por terceiros durante a operação
e uso como sandbox de arquivos hostis ficam fora do contrato. Verificação é um
snapshot; alterações posteriores exigem nova verificação. Artefatos/manifesto
symlink, paths não canônicos internos e arquivos extras são rejeitados.
Arquivos são lidos em memória: sem streaming/limite de tamanho nesta fase.
O pacote não resolve caminhos absolutos contidos nos dados originais: movê-lo
preserva integridade, mas não transporta os fontes nem torna a análise reproduzível.
Permissão 0700 do diretório requer decisão explícita do dono para compartilhamento
entre usuários locais; transporte por arquivo/cópia deve preservar os bytes.

## Testes e resultado

```sh
cargo test --offline --locked --manifest-path lab/pacote-analise/Cargo.toml
cargo clippy --offline --locked --manifest-path lab/pacote-analise/Cargo.toml \
  --all-targets --no-deps -- -D warnings
cargo fmt --manifest-path lab/pacote-analise/Cargo.toml -- --check
```

Verificação em 2026-09-26, Linux/Rust 1.92: **21 testes passaram**, Clippy sem
avisos e formatação conferida. Os comandos reais create/verify e a verificação
após mover o pacote também passaram. Testes Rust inline incluem pacote
válido/evidência binária, preservação byte a byte, execução completa versus cobertura
parcial, par de execuções diferentes, artefatos alterados/ausentes, versão de
manifesto incompatível, grafo rejeitado pelo desserializador real, cópia falha,
concorrência sem sobrescrita, destino vazio/existente, pacote movido sem fontes,
metadados ausentes/sem hash, opt-in de ambiente, versões/provedores desconhecidos,
normalização dos seis contratos, adulteração do resumo e de caminhos, âncora do
manifesto, symlinks/extras e comandos create/verify. Nenhum teste executa provedores.
