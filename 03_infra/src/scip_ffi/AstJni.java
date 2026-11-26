// Crystalline Lineage
// @prompt 00_nucleo/prompts/infra-ast-jni.md
// @prompt-hash de5ee624
// @layer L3
// @updated 2026-11-26

import com.sun.source.tree.*;
import com.sun.source.util.*;
import java.io.*;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.regex.Pattern;
import javax.lang.model.element.*;
import javax.tools.*;

class AstJni {
    record Biblioteca(String nome, boolean logica) {}

    static final class Fonte extends SimpleJavaFileObject {
        final int indice;
        final String texto;

        Fonte(int indice, String caminho, String texto) throws Exception {
            super(new URI("mem", null, "/" + caminho, null), Kind.SOURCE);
            this.indice = indice;
            this.texto = texto;
        }

        @Override public CharSequence getCharContent(boolean ignoreEncodingErrors) {
            return texto;
        }
    }

    static String ler(DataInputStream in) throws IOException {
        int tamanho = in.readInt();
        byte[] bytes = in.readNBytes(tamanho);
        if (bytes.length != tamanho) throw new EOFException();
        return new String(bytes, StandardCharsets.UTF_8);
    }

    public static void main(String[] args) throws Exception {
        var input = new DataInputStream(new BufferedInputStream(System.in));
        var fontes = new ArrayList<Fonte>();
        for (int total = input.readInt(), i = 0; i < total; i++) {
            fontes.add(new Fonte(i, ler(input), ler(input)));
        }
        var compilador = ToolProvider.getSystemJavaCompiler();
        if (compilador == null) return;
        var diagnosticos = new DiagnosticCollector<JavaFileObject>();
        try (var arquivos = compilador.getStandardFileManager(diagnosticos, null, StandardCharsets.UTF_8)) {
            JavacTask tarefa = (JavacTask) compilador.getTask(null, arquivos, diagnosticos,
                List.of("-proc:none", "-classpath", ""), null, fontes);
            var unidades = new ArrayList<CompilationUnitTree>();
            tarefa.parse().forEach(unidades::add);
            tarefa.analyze();
            var extrator = new Extrator(tarefa, fontes);
            unidades.forEach(unidade -> extrator.scan(unidade, null));
            System.out.print("[" + String.join(",", extrator.registros) + "]");
        }
    }

    static final class Extrator extends TreePathScanner<Void, Void> {
        final JavacTask tarefa;
        final Trees arvores;
        final SourcePositions posicoes;
        final Map<URI, Fonte> fontes = new HashMap<>();
        final Map<TypeElement, Optional<Biblioteca>> bibliotecas = new HashMap<>();
        final List<String> registros = new ArrayList<>();
        String chamador;

        Extrator(JavacTask tarefa, List<Fonte> fontes) {
            this.tarefa = tarefa;
            this.arvores = Trees.instance(tarefa);
            this.posicoes = arvores.getSourcePositions();
            fontes.forEach(fonte -> this.fontes.put(fonte.toUri(), fonte));
        }

        String local(CompilationUnitTree unidade, long inicio, String nome) {
            Fonte fonte = fontes.get(unidade.getSourceFile().toUri());
            if (fonte == null || inicio < 0 || inicio > fonte.texto.length()) return null;
            int byteInicial = (int) inicio;
            if (!fonte.texto.startsWith(nome, byteInicial)) return null;
            int inicioLinha = fonte.texto.lastIndexOf('\n', byteInicial - 1) + 1;
            int coluna = fonte.texto.substring(inicioLinha, byteInicial).getBytes(StandardCharsets.UTF_8).length;
            int fim = coluna + nome.getBytes(StandardCharsets.UTF_8).length;
            return "[" + fonte.indice + "," + unidade.getLineMap().getLineNumber(inicio)
                + "," + coluna + "," + fim + "]";
        }

        String declaracao(TreePath caminho) {
            if (caminho == null || !(caminho.getLeaf() instanceof MethodTree metodo)
                || metodo.getReturnType() == null) return null;
            CompilationUnitTree unidade = caminho.getCompilationUnit();
            Fonte fonte = fontes.get(unidade.getSourceFile().toUri());
            if (fonte == null) return null;
            long inicio = posicoes.getEndPosition(unidade, metodo.getReturnType());
            if (inicio < 0 || inicio > fonte.texto.length()) return null;
            String nome = metodo.getName().toString();
            String espaco = "(?:\\s|/\\*.*?\\*/|//[^\\r\\n]*(?:\\r?\\n|$))*";
            var padrao = Pattern.compile(espaco + "(" + Pattern.quote(nome) + ")" + espaco + "\\(", Pattern.DOTALL);
            var match = padrao.matcher(fonte.texto).region((int) inicio, fonte.texto.length());
            return match.lookingAt() ? local(unidade, match.start(1), nome) : null;
        }

        @Override public Void visitMethod(MethodTree no, Void dado) {
            String anterior = chamador;
            String posicao = declaracao(getCurrentPath());
            chamador = posicao == null ? null : "[" + aspas(no.getName().toString()) + "," + posicao + "]";
            if (no.getBody() != null) scan(no.getBody(), dado);
            chamador = anterior;
            return null;
        }

        @Override public Void visitLambdaExpression(LambdaExpressionTree no, Void dado) {
            return null;
        }

        @Override public Void visitMethodInvocation(MethodInvocationTree no, Void dado) {
            var caminho = getCurrentPath();
            Element elemento = arvores.getElement(caminho);
            if (elemento instanceof ExecutableElement metodo && chamador != null
                && metodo.getKind() == ElementKind.METHOD) {
                String nome = metodo.getSimpleName().toString();
                var unidade = caminho.getCompilationUnit();
                long fim = posicoes.getEndPosition(unidade, no.getMethodSelect());
                String local = local(unidade, fim - nome.length(), nome);
                if (local != null) {
                    String ponte = ponte(metodo);
                    registros.add("{\"local\":" + local + ",\"chamador\":" + chamador + ",\"ponte\":" + ponte + "}");
                }
            }
            return super.visitMethodInvocation(no, dado);
        }

        String ponte(ExecutableElement metodo) {
            Set<Modifier> modificadores = metodo.getModifiers();
            if (!modificadores.containsAll(Set.of(Modifier.NATIVE, Modifier.STATIC))
                || !(metodo.getEnclosingElement() instanceof TypeElement classe)) return "null";
            long homonimos = classe.getEnclosedElements().stream()
                .filter(e -> e.getKind() == ElementKind.METHOD && e.getModifiers().contains(Modifier.NATIVE))
                .filter(e -> e.getSimpleName().equals(metodo.getSimpleName())).count();
            if (homonimos != 1) return "null";
            String posicao = declaracao(arvores.getPath(metodo));
            if (posicao == null) return "null";
            var biblioteca = bibliotecas.computeIfAbsent(classe, this::biblioteca).orElse(null);
            if (biblioteca == null) return "null";
            String nome = metodo.getSimpleName().toString();
            String classeBinaria = tarefa.getElements().getBinaryName(classe).toString().replace('.', '/');
            String escClasse = escapeJni(classeBinaria);
            String escMetodo = escapeJni(nome);
            if (escClasse == null || escMetodo == null) return "null";
            String linker = "Java_" + escClasse + "_" + escMetodo;
            return "{\"nome\":" + aspas(nome) + ",\"linker\":" + aspas(linker) +
                ",\"biblioteca\":" + aspas(biblioteca.nome()) + ",\"logica\":" + biblioteca.logica() +
                ",\"declaracao\":" + posicao + "}";
        }

        Optional<Biblioteca> biblioteca(TypeElement classe) {
            TreePath caminho = arvores.getPath(classe);
            if (caminho == null || !(caminho.getLeaf() instanceof ClassTree no)) return Optional.empty();
            var resultados = new HashSet<Biblioteca>();
            for (Tree membro : no.getMembers()) {
                if (!(membro instanceof BlockTree bloco) || !bloco.isStatic()) continue;
                for (StatementTree stmt : bloco.getStatements()) {
                    if (!(stmt instanceof ExpressionStatementTree expr)
                        || !(expr.getExpression() instanceof MethodInvocationTree chamada)) continue;
                    var alvo = arvores.getElement(TreePath.getPath(caminho.getCompilationUnit(), chamada));
                    if (!(alvo instanceof ExecutableElement metodo)
                        || !metodo.getEnclosingElement().toString().equals("java.lang.System")) continue;
                    String nome = metodo.getSimpleName().toString();
                    if (!nome.equals("load") && !nome.equals("loadLibrary")) continue;
                    if (chamada.getArguments().size() != 1
                        || !(chamada.getArguments().get(0) instanceof LiteralTree literal)
                        || !(literal.getValue() instanceof String biblioteca)) return Optional.empty();
                    resultados.add(new Biblioteca(biblioteca, nome.equals("loadLibrary")));
                }
            }
            return resultados.size() == 1 ? resultados.stream().findFirst() : Optional.empty();
        }
    }

    static String escapeJni(String nome) {
        StringBuilder saida = new StringBuilder();
        for (char c : nome.toCharArray()) {
            if (c == '/') saida.append('_');
            else if (c == '_') saida.append("_1");
            else if (c >= 'a' && c <= 'z' || c >= 'A' && c <= 'Z') saida.append(c);
            else if (c >= '0' && c <= '9') {
                if (c <= '3' && (saida.isEmpty() || saida.charAt(saida.length() - 1) == '_')) return null;
                saida.append(c);
            } else saida.append(String.format("_0%04x", (int) c));
        }
        return saida.toString();
    }

    static String aspas(String texto) {
        StringBuilder saida = new StringBuilder("\"");
        for (char c : texto.toCharArray()) {
            if (c == '"' || c == '\\') saida.append('\\').append(c);
            else if (c < 0x20) saida.append(String.format("\\u%04x", (int) c));
            else saida.append(c);
        }
        return saida.append('"').toString();
    }
}
