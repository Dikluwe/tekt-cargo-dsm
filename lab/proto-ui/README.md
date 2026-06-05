# proto-ui — protótipo de Arena (laudo 0029)

Página web descartável que consome o JSON da CLI da lente. Sem build,
sem CDN, sem framework. Cabe num único `index.html`.

**Não é produção.** Vive em `lab/` por design — é experimento para
**aprender o que falta no JSON** antes de nuclear uma UI de verdade.

---

## Como rodar

Por causa de `fetch()` em arquivos locais (CORS), precisa de servidor HTTP
mínimo:

```bash
cd lab/proto-ui
python3 -m http.server 8080
# abrir http://localhost:8080/
```

Qualquer servidor estático serve (Caddy, `npx http-server`, `busybox httpd`).

---

## O que a página mostra

Painel esquerdo — **Ranking**: tabela ordenável (posição, impacto,
classificação, path). Seleciona-se a fonte (`lente_core` top-15 ou `egui`
top-30). Clicar numa linha abre o raio daquele nó, **se** houver dump.

Painel direito — **Raio do nó**: classificação, números (diretos /
transitivos) e a lista plana de `impactados` — quem depende do nó.

---

## Dumps capturados em `dados/`

| Arquivo | O que é | Cmd usado |
|---------|---------|-----------|
| `ranking-lente-core.json` | Ranking top-15 do `lente_core` | `lente --pacote lente_core --ranking --top 15` |
| `ranking-egui.json` | Ranking top-30 do `egui` v0.34.3 | `lente --pacote egui --ranking --top 30` (do diretório `crates/egui` do repo egui) |
| `raio-path.json` | Raio de `lente_core::entities::grafo::Path` (top do ranking lente_core) | `lente --pacote lente_core --alvo lente_core::entities::grafo::Path --verbose` |
| `raio-kind.json` | Raio de `lente_core::entities::grafo::Kind` (#2) | idem com `Kind` |
| `raio-classificacao.json` | Raio de `lente_core::domain::raio::Classificacao` (#6) | idem |

Output JSON é o **default** da CLI (sem `--text`). `--verbose` no raio é
necessário para que o campo `impactados` apareça (sem ele só vem
contagem).

---

## Achados (resumo — laudo 0029 tem o detalhe)

1. **`impactados` chega como lista plana de paths.** Sem profundidade
   por item, sem arestas entre eles. Suficiente para uma vista de
   **lista**. Para grafo ou camadas-por-profundidade, faltaria:
   - profundidade (BFS já está computada no `Raio.montante: HashMap<Path,
     usize>` — só não é emitida);
   - arestas entre os impactados (não está nem na memória; precisaria
     filtragem do grafo).
2. **Classificação diverge entre ranking e raio-do-mesmo-nó.** Ex.: o
   `Path` do `lente_core` é **Base** no ranking e **Intermediário** no
   `--alvo`. Razão: o pipeline `--ranking` aplica `filtrar_stdlib` antes;
   o `--alvo` **não**. No grafo cru, `Path` tem `uses` saindo para
   stdlib → Intermediário. No grafo filtrado, esses uses somem → Base.
   Não é bug, é divergência **semântica** entre os dois caminhos. UI
   precisa explicitar isso, ou o pipeline precisa convergir.
3. **JSON `--verbose` ordena os impactados alfabeticamente.** Útil para
   leitura na UI, mas não preserva ordem de descoberta (BFS por
   profundidade). Se a UI quiser agrupar por profundidade no futuro, o
   contrato JSON precisa mudar (achado 1).

---

## O que NÃO está aqui

- Vista de grafo (depende de achado 1).
- Comparação entre dois grafos (depende de captura em diferentes commits).
- Polimento visual.
- Build / framework / componentização.
- Testes automatizados.

Tudo isso é responsabilidade do próximo passo — **nuclear** a UI — se e
quando este protótipo tiver mostrado que vale a pena.

---

## Convenção de aposentadoria

Padrão da Arena (laudos 0021 / 0027 / candidato a LESSON do Tekt):

- O **componente** que esta Arena vier a inspirar (se houver) vive no
  workspace.
- A Arena **fica** no `lab/` como registro do experimento. Não é
  duplicação — instrumento de medição não é produto.

Se a UI nuclear, atualizar o laudo 0029 dizendo qual componente nasceu;
manter este protótipo intocado.
