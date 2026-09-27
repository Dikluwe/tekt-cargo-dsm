# Contrato de nucleação — adaptadores de extração
Hash do Código: 8c1ac0f0

Unidade: `03_infra/src/extracao.rs`. Camada L3. ADR-0007.
A fronteira `ExtratorGrafo::extrair(SolicitacaoExtracao)` devolve `ExtracaoGrafo`
ou `ErroProvedor`. Solicitação enumera JSON, grafo em memória, pacote Rust,
diretório Rust e membro Rust com raiz/contexto de toolchain já consultado.
Adaptadores Rust, JSON e memória são explícitos; seleção pertence ao L4.
Solicitação incompatível falha, sem executar I/O. Rust delega exatamente uma
operação existente por solicitação; uma instância compartilha CacheExtracao.
JSON usa desserializar_grafo. Memória clona o grafo válido fornecido.
Erro inclui identidade do provedor, contexto (sem despejar JSON), causa tipada
e Error::source. Helpers ler_texto/canonicalizar concentram filesystem em L3.
Testes controlam a operação Rust para verificar seleção/delegação, contexto e
causas; JSON inválido mantém o erro original; memória preserva dados.

## Interface Snapshot
<!-- GENERATED — não edite manualmente -->
<!-- crystalline-snapshot: {"functions":[{"name":"ler_texto","params":["&Path"],"return_type":"std::io::Result<String>"},{"name":"canonicalizar","params":["&Path"],"return_type":"std::io::Result<PathBuf>"}],"types":[{"name":"SolicitacaoExtracao","kind":"enum","members":["Ferramenta","Json","Memoria","RustPacote","RustDiretorio","RustMembro"]},{"name":"ExtratorGrafo","kind":"trait","members":["extrair"]},{"name":"CausaProvedor","kind":"enum","members":["Ferramenta","Fork","Adaptador","Workspace","SolicitacaoIncompativel"]},{"name":"ErroProvedor","kind":"struct","members":["provedor","contexto","causa"]},{"name":"ExtratorRust","kind":"struct","members":["cache"]},{"name":"ExtratorJson","kind":"struct","members":[]},{"name":"ExtratorMemoria","kind":"struct","members":[]}],"reexports":[]} -->

## Integração aditiva de provedores (2026-09-26)

SolicitacaoExtracao::Ferramenta e CausaProvedor::Ferramenta ampliam a fronteira. O transporte L1 é Json; identidade real e contexto rico ficam no extrator L3 e são obrigatórios no envelope de saída L4. Os adaptadores Rust/JSON/memória conservam seu comportamento.

Instalação, protocolo e matriz verificada: `docs/adaptadores.md`. ADR-0008.
