# Tarefa: rodar `lente --comparar` no par vanilla vs cristalino

**Repositório de trabalho**: typst-crystalline (raiz do workspace cristalino).
**Tipo de tarefa**: execução e coleta de dados. **Nenhum código será escrito.**
**Ferramenta externa**: o binário `lente` (projeto tekt-cargo-dsm) e o fork
`cargo-modules`, ambos já instalados na máquina.

---

## Objetivo

Comparar a estrutura de módulos do Typst vanilla (`lab/typst-original/`)
com a do cristalino (o workspace na raiz deste repositório), usando o modo
`--comparar` da lente, e produzir um relatório com três dados:

1. Contagens do pareamento: módulos pareados, sem-par no lado antes,
   sem-par no lado depois.
2. Falhas de pacote na extração (quais crates ficaram fora e por quê).
3. Tempo de execução.

## Restrições obrigatórias

- **Não modificar nenhum arquivo do repositório**, com uma única exceção:
  o symlink temporário do passo 2, que deve ser removido ao final
  (inclusive se a tarefa falhar no meio).
- **Não escrever código.** Esta tarefa não passa pelo protocolo de
  nucleação porque não produz código L1–L4.
- **Não ler** `00_nucleo/materialization/` nem `00_nucleo/context/`
  (restrição do CLAUDE.md do repositório).
- Se a `lente` ou o fork não estiverem instalados, **parar e reportar**.
  Não instalar, não compilar, não improvisar alternativa.
- Salvar saídas em `/tmp/`, não dentro do repositório.

## Passos

### 1. Verificar as ferramentas

```bash
lente --help
cargo modules --version
```

- Confirmar no `--help` a sintaxe real do modo de comparação (flags
  `--comparar`, `--antes`, `--depois`, `--json`). Se a sintaxe real
  divergir da usada abaixo, usar a real e **registrar a divergência no
  relatório**.
- Se qualquer um dos dois binários faltar: parar e reportar.

### 2. Preparar o lado "antes" (vanilla)

```bash
ls lab/typst-original/Cargo.toml*
```

- Se existir apenas `Cargo.toml.original`: criar o symlink
  `ln -s Cargo.toml.original lab/typst-original/Cargo.toml` e anotar que
  ele precisa ser removido no passo 6.
- Se `Cargo.toml` já existir: não tocar, e anotar que não foi criado por
  esta tarefa (logo não deve ser removido no passo 6).

### 3. Verificar que o lado "depois" não engole o vanilla

```bash
grep -A 20 "members" Cargo.toml
```

- Confirmar que `lab/` (ou `lab/typst-original`) **não** está nos members
  do workspace da raiz. Se estiver: parar e reportar, porque o vanilla
  entraria nos dois lados e as contagens perderiam o sentido.

### 4. Rodar a comparação

Da raiz do repositório, medindo o tempo:

```bash
time lente --comparar --antes lab/typst-original --depois . | tee /tmp/comparar-typst.txt
time lente --comparar --antes lab/typst-original --depois . --json > /tmp/comparar-typst.json
```

- Se um crate específico falhar na extração, registrar o nome do crate e a
  mensagem de erro exata. Não tentar consertar o crate.
- Se a execução inteira falhar, capturar stderr completo e ir direto ao
  passo 6 (desfazer o symlink) antes de reportar.

### 5. Extrair os números do resultado

Do texto e/ou do JSON, extrair:

- Total de módulos no lado antes e no lado depois.
- Quantos pareados.
- Quantos sem-par no antes e quantos sem-par no depois.
- Os 10 primeiros itens de cada lista de sem-par (amostra, não a lista
  inteira).
- Quais crates falharam na extração, se algum.

### 6. Desfazer o symlink

Apenas se ele foi criado no passo 2:

```bash
rm lab/typst-original/Cargo.toml
```

Confirmar com `ls lab/typst-original/Cargo.toml*` que sobrou apenas o
`Cargo.toml.original`.

### 7. Relatório final

Responder com esta estrutura, valores literais, sem interpretação:

```
## Ferramentas
- lente: <versão ou primeira linha do --help>
- fork cargo modules: <versão>
- Sintaxe do --comparar divergiu do esperado? <não / sim: qual>

## Pareamento
- Módulos no antes: <n>
- Módulos no depois: <n>
- Pareados: <n>
- Sem-par no antes: <n>
- Sem-par no depois: <n>
- Amostra sem-par antes (10): <lista>
- Amostra sem-par depois (10): <lista>

## Falhas de extração
- <crate>: <erro exato>  (ou "nenhuma")

## Execução
- Tempo (texto): <real do time>
- Tempo (json): <real do time>
- Arquivos: /tmp/comparar-typst.txt, /tmp/comparar-typst.json

## Estado do repositório
- Symlink criado e removido? <sim/não/não foi necessário>
- git status limpo? <sim/não: o que sobrou>
```

O relatório não deve concluir nada sobre a migração — só apresentar os
dados. A interpretação acontece na conversa, não aqui.
