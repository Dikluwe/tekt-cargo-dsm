# Crystalline Lineage
# @prompt 00_nucleo/prompts/infra-outras-ffi.md
# @prompt-hash e7027c98
# @layer L3
# @updated 2026-11-26

"""Recognize direct, literal FFI bindings in indexed sources."""
import collections
import json
import pathlib
import re
import sys


LEXEMA = re.compile(
    r'\s+|//[^\n]*|/\*[\s\S]*?\*/|"""[\s\S]*?"""'
    r'|@"(?:[^"]|"")*"|"(?:\\.|[^"\\\n])*"|\'(?:\\.|[^\'\\\n])*\''
    r'|\x60[^\x60]*\x60|(?:[^\W\d]|\$)[\w$]*|=>|::|[^\s]'
)
IDENTIFICADOR = re.compile(r'(?:[^\W\d]|\$)[\w$]*\Z')
LITERAL = re.compile(r'''(["'])([^"'\\\n]+)\1\Z''')
REGEX_JS = re.compile(r'/(?:\\.|\[(?:\\.|[^\]\\\n])*\]|[^/\[\\\n])+/[A-Za-z]*')
MULTILINHA_ZIG = re.compile(r'\\\\[^\r\n]*')
JS = {".js", ".jsx", ".cjs", ".mjs", ".ts", ".tsx", ".cts", ".mts"}
INICIO_EXPRESSAO = {"(", "[", "{", ",", ";", "=", ":", "!", "?", "=>", "return", "throw"}
ALINHAMENTO = {"(": ")", "[": "]", "{": "}"}


class Documento:
    def __init__(self, indice, arquivo, texto):
        self.indice = indice
        self.arquivo = pathlib.PurePosixPath(arquivo)
        self.ext = self.arquivo.suffix
        self.tokens = []
        self.posicoes = []
        linha, coluna = 1, 0
        cursor = 0
        while cursor < len(texto):
            achado = LEXEMA.match(texto, cursor)
            if not achado:
                raise ValueError("incomplete token")
            if (self.ext in JS and achado[0] == "/"
                    and (not self.tokens or self.tokens[-1] in INICIO_EXPRESSAO)):
                achado = REGEX_JS.match(texto, cursor) or achado
            if self.ext == ".zig":
                bloco = MULTILINHA_ZIG.match(texto, cursor)
                if bloco:
                    achado = bloco
            token = achado[0]
            cursor = achado.end()
            bytes_token = len(token.encode())
            if not token.isspace() and not token.startswith(("//", "/*")):
                self.tokens.append(token)
                self.posicoes.append([indice, linha, coluna, coluna + bytes_token])
            if "\n" in token:
                linha += token.count("\n")
                coluna = len(token.rsplit("\n", 1)[1].encode())
            else:
                coluna += bytes_token
        self.total = len(self.tokens)
        self.tokens.extend([""] * 12)
        self.pares = {}
        pilha = []
        for i in range(self.total):
            token = self.tokens[i]
            if token in ALINHAMENTO:
                pilha.append(i)
            elif token in ALINHAMENTO.values():
                if not pilha or ALINHAMENTO[self.tokens[pilha[-1]]] != token:
                    raise ValueError("incomplete source")
                self.pares[pilha.pop()] = i
        if pilha:
            raise ValueError("incomplete source")

    def literal(self, i):
        achado = LITERAL.fullmatch(self.tokens[i])
        if not achado or (self.ext in {".kt", ".dart"} and "$" in achado[2]):
            return None
        return achado[2]

    def ident(self, i):
        token = self.tokens[i]
        return IDENTIFICADOR.fullmatch(token) is not None

    def partes(self, inicio, fim, separador=","):
        """Read immediate arguments/properties without descending into expressions."""
        a = inicio
        i = inicio
        while i < fim:
            if self.tokens[i] == separador:
                yield a, i
                a = i + 1
            i = self.pares.get(i, i) + 1
        if a < fim:
            yield a, fim

    def imediato(self, inicio, fim):
        while inicio < fim:
            yield inicio
            inicio = self.pares.get(inicio, inicio) + 1

    def registro(self, i, ponte=None, declaracao=False):
        return dict(local=self.posicoes[i], ponte=ponte, somente_declaracao=declaracao)

    def chamada_em(self, i, nome=None):
        return (nome is None or self.tokens[i] == nome) and self.ident(i) and self.tokens[i + 1] == "("

    def fim_tipo_zig(self, i):
        v = self.tokens
        while i < self.total:
            if v[i] in {"struct", "union", "enum", "opaque", "error", "fn"}:
                break
            if v[i] in {"callconv", "align", "addrspace", "linksection"} and v[i + 1] == "(":
                i = self.pares[i + 1] + 1
            elif v[i] == "[":
                i = self.pares[i] + 1
            elif self.ident(i) or v[i] in {"?", "*", "!", "."}:
                i += 1
            else:
                break
        return i

    def funcoes(self):
        palavras = {"if", "for", "while", "when", "switch", "catch", "with", "synchronized", "return", "new", "using"}
        v = self.tokens
        for i in range(1, self.total):
            if not self.chamada_em(i) or v[i] in palavras:
                continue
            fim_args = self.pares[i + 1]
            anterior = v[i - 1]
            tipos_go = self.ext == ".go" and anterior == "func"
            if anterior in {"(", ".", "::", "=", "=>"}:
                continue
            tipo_zig = self.ext == ".zig" and anterior == "fn"
            if anterior not in {"func", "fun", "function"} and not tipo_zig and not (
                self.ext in {".cs", ".dart"} and self.ident(i - 1)
            ):
                continue
            if anterior in palavras:
                continue
            corpo = fim_args + 1
            if tipo_zig:
                corpo = self.fim_tipo_zig(corpo)
            elif v[corpo] == ":" or tipos_go:
                if v[corpo] == ":":
                    corpo += 1
                while self.ident(corpo) or v[corpo] in {"?", "*", "."}:
                    corpo += 1
            if v[corpo] == "{":
                yield i, corpo + 1, self.pares[corpo]
            elif v[corpo] == "=>" or (self.ext == ".kt" and v[corpo] == "="):
                linha = self.posicoes[corpo][1]
                fim = corpo + 1
                while fim < self.total and v[fim] not in {";", "}"}:
                    if self.posicoes[fim][1] != linha and self.ext == ".kt":
                        break
                    fim = self.pares.get(fim, fim) + 1
                yield i, corpo + 1, fim


def ponte(doc, i, linker, biblioteca, **dados):
    return doc.registro(i, dict(nome=doc.tokens[i], linker=linker, biblioteca=biblioteca, **dados))


def declaracao(doc, i, linker, biblioteca, **dados):
    loc = doc.posicoes[i]
    return doc.registro(i, dict(nome=doc.tokens[i], linker=linker, biblioteca=biblioteca,
                                declaracao=loc, **dados), declaracao=True)


def go(doc):
    v = doc.tokens
    importou_c = False
    for i in range(doc.total):
        if v[i] != "import":
            continue
        if doc.literal(i + 1) == "C":
            importou_c = True
        if v[i + 1] == "(":
            for j in range(i + 2, doc.pares[i + 1]):
                if doc.literal(j) == "C" and (
                    j == i + 2 or v[j - 1] == ";" or
                    doc.posicoes[j - 1][1] < doc.posicoes[j][1]
                ):
                    importou_c = True
    if not importou_c:
        return
    usos = [i for i in range(doc.total) if v[i] == "C"]
    if any((i > 0 and v[i - 1] == ".") or v[i + 1] != "." for i in usos):
        return
    for i in usos:
        if doc.chamada_em(i + 2):
            yield ponte(doc, i + 2, v[i + 2], "", tipo="cgo")


def zig(doc):
    v = doc.tokens
    for i in doc.imediato(0, doc.total):
        if v[i:i + 2] == ["extern", "fn"] and doc.chamada_em(i + 2):
            nome = i + 2
            inicio_tipo = doc.pares[nome + 1] + 1
            fim_tipo = doc.fim_tipo_zig(inicio_tipo)
            if v[fim_tipo] != ";" or inicio_tipo == fim_tipo:
                continue
            convencoes = [j for j in range(inicio_tipo, fim_tipo) if v[j] == "callconv"]
            if any(v[j + 1:j + 5] not in (["(", ".", "c", ")"], ["(", ".", "C", ")"])
                   for j in convencoes):
                continue
            yield declaracao(doc, nome, v[nome], "", tipo="zig")
            for uso in ligacao_constante(doc, nome, []) or []:
                yield ponte(doc, uso, v[nome], "", tipo="zig", declaracao=doc.posicoes[nome])
            continue
        if not (v[i] == "const" and doc.ident(i + 1)
                and v[i + 2:i + 7] == ["=", "@", "cImport", "(", "{"]):
            continue
        fim = doc.pares[i + 6]
        if v[fim + 1:fim + 3] != [")", ";"]:
            continue
        includes = range(i + 7, fim, 6)
        if not includes or any(v[j:j + 3] != ["@", "cInclude", "("]
                               or not doc.literal(j + 3)
                               or v[j + 4:j + 6] != [")", ";"]
                               for j in includes):
            continue
        for uso in ligacao_constante(doc, i + 1, ["."], permite_membros=True) or []:
            yield ponte(doc, uso, v[uso], "", tipo="zig")


def csharp(doc):
    v = doc.tokens
    namespace = ["System", ".", "Runtime", ".", "InteropServices"]
    importado = any(v[i:i + 7] == ["using", *namespace, ";"] for i in range(doc.total))
    atributos = {"DllImport": "extern", "DllImportAttribute": "extern",
                 "LibraryImport": "partial", "LibraryImportAttribute": "partial"}
    ocultos = {
        v[i + 1].removesuffix("Attribute")
        for i in range(doc.total)
        if v[i] in {"class", "struct", "record"} or (v[i] == "using" and v[i + 2] == "=")
    }
    for i in range(doc.total):
        if v[i] != "[":
            continue
        inicio = i + 1
        if v[inicio:inicio + 2] == ["global", "::"]:
            inicio += 2
        if v[inicio:inicio + 6] == [*namespace, "."]:
            inicio += 6
        elif not importado or v[inicio].removesuffix("Attribute") in ocultos:
            continue
        modificador = atributos.get(v[inicio])
        if not modificador or v[inicio + 1] != "(":
            continue
        fim = doc.pares[inicio + 1]
        if fim != doc.pares[i] - 1:
            continue
        args = list(doc.partes(inicio + 2, fim))
        if not args or args[0] != (inicio + 2, inicio + 3):
            continue
        biblioteca = doc.literal(inicio + 2)
        if not biblioteca:
            continue
        pontos = [(a, b) for a, b in args[1:] if v[a] == "EntryPoint"]
        alias = None
        if pontos:
            a, b = pontos[0]
            if len(pontos) != 1 or b != a + 3 or v[a + 1] != "=":
                continue
            alias = doc.literal(a + 2)
            if not alias:
                continue
        j = fim + 2
        while v[j] == "[":
            j = doc.pares[j] + 1
        inicio_metodo = j
        while j < doc.total and (doc.ident(j) or v[j] in {"*", ".", "?"}):
            j += 1
        nome = j - 1
        if j <= inicio_metodo or not doc.chamada_em(nome) or v[doc.pares[j] + 1] != ";":
            continue
        modificadores = v[inicio_metodo:nome]
        if "static" in modificadores and modificador in modificadores:
            yield declaracao(doc, nome, alias or v[nome], biblioteca, tipo="pinvoke")


def jni(texto):
    utf16 = texto.encode("utf-16-be")
    especiais = {"/": "_", "_": "_1", ";": "_2", "[": "_3"}
    resultado = []
    for i in range(0, len(utf16), 2):
        unidade = int.from_bytes(utf16[i:i + 2], "big")
        c = chr(unidade)
        resultado.append(especiais.get(c, c if c.isascii() and c.isalnum() else f"_0{unidade:04x}"))
    return "".join(resultado)


def kotlin(doc):
    v = doc.tokens
    pacote = ""
    for i in range(doc.total):
        if v[i] != "package":
            continue
        partes = []
        while doc.ident(i + 1):
            partes.append(v[i + 1])
            i += 2
            if v[i] != ".":
                break
        pacote = "/".join(partes) + "/" if partes else ""
        break
    if any(v[i] == "System" and v[i + 1:i + 3] != [".", "loadLibrary"] for i in range(doc.total)):
        return
    for i in doc.imediato(0, doc.total):
        if i and v[i - 1] == "companion":
            continue
        if v[i] != "object" or not doc.ident(i + 1) or v[i + 2] != "{":
            continue
        fim = doc.pares[i + 2]
        biblioteca = []
        metodos = collections.defaultdict(list)
        for a in doc.imediato(i + 3, fim):
            if v[a:a + 2] == ["init", "{"]:
                j = a + 2
                if v[j:j + 4] == ["java", ".", "lang", "."]:
                    j += 4
                if v[j:j + 4] != ["System", ".", "loadLibrary", "("]:
                    continue
                if v[j + 5] != ")" or v[j + 6:doc.pares[a + 1]] not in ([], [";"]):
                    continue
                if (literal := doc.literal(j + 4)):
                    biblioteca.append(literal)
            if v[a] == "fun" and doc.chamada_em(a + 1):
                metodos[v[a + 1]].append(a)
        if len(biblioteca) != 1:
            continue
        for nome, declaracoes in metodos.items():
            if len(declaracoes) == 1 and v[declaracoes[0] - 1] == "external":
                metodo = declaracoes[0] + 1
                linker = f"Java_{jni(pacote + v[i + 1])}_{jni(nome)}"
                yield declaracao(doc, metodo, linker, biblioteca[0], logica=True)


def ligacao_constante(doc, i, prefixo, permite_fechar=False, permite_membros=False):
    """Reject escaped or rebound values, including simple shadowing."""
    v = doc.tokens
    nome = v[i]
    usos = []
    for j in range(doc.total):
        if v[j] != nome or j == i:
            continue
        if permite_membros and j > 0 and v[j - 1] == ".":
            continue
        if j == 0 or v[j - 1] in {".", "?.", "fn"}:
            return None
        if permite_fechar and v[j + 1:j + 5] == [".", "close", "(", ")"]:
            continue
        if v[j + 1:j + 1 + len(prefixo)] != prefixo:
            return None
        alvo = j + len(prefixo) + 1 if prefixo else j
        if not doc.chamada_em(alvo):
            if permite_membros and doc.ident(alvo):
                continue
            return None
        usos.append(alvo)
    return usos


def deno(doc):
    v = doc.tokens
    if any(v[i] == "Deno" and v[i + 1] != "." for i in range(doc.total)):
        return
    if any(v[i:i + 4] == ["Deno", ".", "dlopen", "="] for i in range(doc.total)):
        return
    for i in range(doc.total):
        if v[i] != "const" or not doc.ident(i + 1) or v[i + 2:i + 7] != ["=", "Deno", ".", "dlopen", "("]:
            continue
        biblioteca = doc.literal(i + 7)
        if not biblioteca or v[i + 8:i + 10] != [",", "{"]:
            continue
        fim = doc.pares[i + 9]
        if v[fim + 1:fim + 2] == [","]:
            fim += 1
        if v[fim + 1] != ")" or v[fim + 2] in {".", "[", "(", "?"}:
            continue
        simbolos = {}
        for a, b in doc.partes(i + 10, doc.pares[i + 9]):
            if not doc.ident(a) or v[a + 1:a + 3] != [":", "{"] or doc.pares[a + 2] != b - 1:
                break
            campos = list(doc.partes(a + 3, b - 1))
            if any(not doc.ident(x) or v[x + 1] != ":" for x, _ in campos):
                break
            chaves = [v[x] for x, _ in campos]
            if len(chaves) != len(set(chaves)) or not {"parameters", "result"}.issubset(chaves):
                break
            aliases = [(x, y) for x, y in campos if v[x] == "name"]
            alias = v[a]
            if aliases:
                x, y = aliases[0]
                alias = doc.literal(x + 2) if y == x + 3 else None
            if not alias or v[a] in simbolos:
                break
            simbolos[v[a]] = alias
        else:
            for uso in ligacao_constante(doc, i + 1, [".", "symbols", "."], permite_fechar=True) or []:
                if v[uso] in simbolos:
                    yield ponte(doc, uso, simbolos[v[uso]], biblioteca)


def dart(doc):
    v = doc.tokens
    imports = []
    for i in range(doc.total):
        if v[i] != "import" or doc.literal(i + 1) != "dart:ffi":
            continue
        if v[i + 2] == ";":
            imports.append([])
        elif v[i + 2] == "as" and doc.ident(i + 3) and v[i + 4] == ";":
            imports.append([v[i + 3], "."])
    for prefixo in imports:
        abertura = [*prefixo, "DynamicLibrary", ".", "open", "("]
        for i in range(doc.total):
            j = i + 3 + len(abertura)
            if v[i] != "final" or not doc.ident(i + 1) or v[i + 2:j] != ["=", *abertura]:
                continue
            biblioteca = doc.literal(j)
            if not biblioteca or v[j + 1:j + 3] != [")", ";"]:
                continue
            usos_biblioteca = [a for a in range(doc.total) if v[a] == v[i + 1] and a != i + 1]
            for a in usos_biblioteca:
                if a < 3 or v[a - 3] != "final" or v[a - 1] != "=" or v[a + 1:a + 4] != [".", "lookupFunction", "<"]:
                    break
            else:
                for a in usos_biblioteca:
                    b = a + 4
                    profundidade = 1
                    while b < doc.total and v[b] not in {";", "{", "}"}:
                        profundidade += (v[b] == "<") - (v[b] == ">")
                        b = doc.pares.get(b, b) + 1
                        if not profundidade:
                            break
                    if profundidade or v[b] != "(" or v[b + 2:b + 4] != [")", ";"]:
                        continue
                    linker = doc.literal(b + 1)
                    if linker:
                        for uso in ligacao_constante(doc, a - 2, []) or []:
                            yield ponte(doc, uso, linker, biblioteca)


def registro_node(doc):
    v = doc.tokens
    if not any(v[i:i + 7] == ["#", "include", "<", "node_api", ".", "h", ">"] for i in range(doc.total)):
        return
    if any(v[i] == "#" and v[i + 1] not in {"include", "pragma"} for i in range(doc.total)):
        return
    for i in range(doc.total):
        if v[i:i + 4] != ["NAPI_MODULE_INIT", "(", ")", "{"]:
            continue
        a = i + 4
        fim_corpo = doc.pares[i + 3]
        registros = []
        while v[a] == "napi_value" and doc.ident(a + 1) and v[a + 2] == ";":
            variavel = v[a + 1]
            a += 3
            if not doc.chamada_em(a, "napi_create_function"):
                break
            fim = doc.pares[a + 1]
            args = list(doc.partes(a + 2, fim))
            if len(args) != 6 or v[fim + 1] != ";":
                break
            env, nome, tamanho, callback, dados, resultado = [v[x:y] for x, y in args]
            exportada = doc.literal(args[1][0])
            if (env != ["env"] or len(nome) != 1 or not exportada
                    or tamanho != ["NAPI_AUTO_LENGTH"] or len(callback) != 1 or dados != ["NULL"]
                    or resultado != ["&", variavel] or not doc.ident(args[3][0])):
                break
            b = fim + 2
            if not doc.chamada_em(b, "napi_set_named_property"):
                break
            fim_b = doc.pares[b + 1]
            if v[b + 2:fim_b] != ["env", ",", "exports", ",", nome[0], ",", variavel] or v[fim_b + 1] != ";":
                break
            registros.append((exportada, callback[0], str(doc.arquivo)))
            a = fim_b + 2
        if v[a:fim_corpo] == ["return", "exports", ";"]:
            yield from registros

def node(doc, exportacoes):
    v = doc.tokens
    if any(
        v[i] == "require" and (v[i + 1] != "(" or v[i - 1] in {"function", ".", "?."})
        for i in range(doc.total)
    ):
        return
    for i in range(doc.total):
        if v[i] != "const" or not doc.ident(i + 1) or v[i + 2:i + 5] != ["=", "require", "("]:
            continue
        biblioteca = doc.literal(i + 5)
        if (not biblioteca or not biblioteca.endswith(".node") or v[i + 6] != ")"
                or v[i + 7] in {".", "[", "(", "?"}):
            continue
        if not biblioteca.startswith(("./", "../", "/")):
            continue
        caminho = str(doc.arquivo.parent / biblioteca)
        for uso in ligacao_constante(doc, i + 1, ["."]) or []:
            candidatos = exportacoes.get(v[uso], [])
            if len(candidatos) == 1:
                linker, nativo = candidatos[0]
                yield ponte(doc, uso, linker, caminho if "/" in caminho else "./" + caminho,
                             tipo="node", arquivo_nativo=nativo)


def extrair(entradas):
    documentos = []
    exportacoes = collections.defaultdict(list)
    for indice, (arquivo, texto) in enumerate(entradas):
        try:
            doc = Documento(indice, arquivo, texto)
        except ValueError:
            continue
        if doc.ext in {".c", ".cc", ".cpp", ".cxx", ".cp", ".C"}:
            for nome, linker, arquivo_nativo in registro_node(doc):
                exportacoes[nome].append((linker, arquivo_nativo))
        else:
            documentos.append(doc)
    extratores = {".go": go, ".cs": csharp, ".kt": kotlin, ".dart": dart, ".zig": zig}
    for doc in documentos:
        funcoes = sorted(doc.funcoes(), key=lambda f: (f[1], -f[2]))
        nao_chamadas = {nome for nome, _, _ in funcoes}
        if doc.ext == ".cs" and any(doc.tokens[i] == "#" for i in range(doc.total)):
            pontes = []
        elif doc.ext in extratores:
            pontes = list(extratores[doc.ext](doc))
        else:
            pontes = [*deno(doc), *node(doc, exportacoes)]
        funcoes = iter(funcoes)
        proxima = next(funcoes, None)
        escopo = []
        marcas = collections.defaultdict(list)
        for registro in pontes:
            posicao = registro["local"]
            marcas[tuple(posicao)].append(registro)
        for i in range(doc.total):
            while escopo and i >= escopo[-1][2]:
                escopo.pop()
            while proxima and proxima[1] <= i:
                if proxima[2] > i:
                    escopo.append(proxima)
                proxima = next(funcoes, None)
            if not doc.chamada_em(i) or i in nao_chamadas:
                continue
            registros = marcas.get(tuple(doc.posicoes[i]), [])
            registros = registros or [doc.registro(i)]
            if len(registros) > 1:
                continue
            registro = registros[0]
            if escopo:
                nome = escopo[-1][0]
                registro["chamador"] = [doc.tokens[nome], doc.posicoes[nome]]
            yield registro


if __name__ == "__main__":
    json.dump(list(extrair(json.load(sys.stdin))), sys.stdout)
