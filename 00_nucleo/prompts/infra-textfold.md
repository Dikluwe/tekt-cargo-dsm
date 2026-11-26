# Adaptador TextFold para prompts Tekt (L3)
Hash do Código: dd1b42d9

Processa somente prompts locais admitidos pela linhagem `@prompt`. Confere
caminho e texto antes da chamada e executa uma vez por caminho canônico.

Executa `python3 -I -m textfold get <arquivo> --json --no-generate` sem shell.
Admite `textfold/doc` versão 0, preserva a origem dos níveis gerados e resolve
evidências para as frases originais. Falha da ferramenta ou entrada inválida
identifica o prompt ao interromper a exportação solicitada.

## Verificação

- Um documento no formato oficial produz níveis e evidências na ordem da exportação.
- Versão incompatível, evidência inválida ou prompt alterado são rejeitados.
- A chamada usa `--no-generate` e o caminho local validado.
