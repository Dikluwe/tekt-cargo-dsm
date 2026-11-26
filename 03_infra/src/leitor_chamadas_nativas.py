# Crystalline Lineage
# @prompt 00_nucleo/prompts/infra-leitor-chamadas-nativas.md
# @layer L3
"""Extrai ocorrências de chamadas da AST da libclang, sem executar o driver do banco."""

import ctypes as c
from ctypes.util import find_library
from concurrent.futures import ProcessPoolExecutor
import json
import multiprocessing
import os
import sys


class String(c.Structure):
    _fields_ = [("data", c.c_void_p), ("flags", c.c_uint)]


class Cursor(c.Structure):
    _fields_ = [("kind", c.c_uint), ("xdata", c.c_int), ("data", c.c_void_p * 3)]


class Location(c.Structure):
    _fields_ = [("pointers", c.c_void_p * 2), ("offset", c.c_uint)]


Visitor = c.CFUNCTYPE(c.c_uint, Cursor, Cursor, c.c_void_p)
FUNCOES = {8, 21, 24, 25, 26, 30}
CALL, DECL_REF, MEMBER_REF, LAMBDA = 103, 101, 102, 144
api = None


class Clang:
    def __init__(self):
        caminho = os.environ.get("LENTE_LIBCLANG")
        if not caminho:
            for nome in ("clang", "clang-22", "clang-21", "clang-20", "clang-19", "clang-18", "clang-17", "clang-16", "clang-15"):
                caminho = find_library(nome)
                if caminho:
                    break
        if not caminho:
            raise RuntimeError("libclang ausente; indique a biblioteca com LENTE_LIBCLANG")
        self.library = c.CDLL(caminho)
        pointer = c.c_void_p
        signatures = {
            "getCString": (c.c_char_p, [String]), "disposeString": (None, [String]),
            "getClangVersion": (String, []),
            "createIndex": (pointer, [c.c_int, c.c_int]), "disposeIndex": (None, [pointer]),
            "parseTranslationUnit2FullArgv": (c.c_int, [pointer, c.c_char_p,
                c.POINTER(c.c_char_p), c.c_int, pointer, c.c_uint, c.c_uint, c.POINTER(pointer)]),
            "disposeTranslationUnit": (None, [pointer]), "getTranslationUnitCursor": (Cursor, [pointer]),
            "visitChildren": (c.c_uint, [Cursor, Visitor, pointer]),
            "getCursorSpelling": (String, [Cursor]), "getCursorUSR": (String, [Cursor]),
            "getCursorSemanticParent": (Cursor, [Cursor]), "isCursorDefinition": (c.c_uint, [Cursor]),
            "getCursorReferenced": (Cursor, [Cursor]), "getCursorLocation": (Location, [Cursor]),
            "Location_isFromMainFile": (c.c_int, [Location]),
            "getSpellingLocation": (None, [Location, c.POINTER(pointer), *([c.POINTER(c.c_uint)] * 3)]),
            "getExpansionLocation": (None, [Location, c.POINTER(pointer), *([c.POINTER(c.c_uint)] * 3)]),
            "getFileName": (String, [pointer]),
            "getNumDiagnostics": (c.c_uint, [pointer]), "getDiagnostic": (pointer, [pointer, c.c_uint]),
            "getDiagnosticSeverity": (c.c_uint, [pointer]),
            "formatDiagnostic": (String, [pointer, c.c_uint]), "disposeDiagnostic": (None, [pointer]),
        }
        for name, (result, args) in signatures.items():
            function = getattr(self.library, "clang_" + name)
            function.restype, function.argtypes = result, args
            setattr(self, name, function)

    def text(self, value):
        try:
            raw = self.getCString(value)
            return raw.decode("utf-8", errors="replace") if raw else ""
        finally:
            self.disposeString(value)

    def children(self, cursor):
        result = []
        @Visitor
        def capture(child, _parent, _data):
            result.append(child)
            return 1
        self.visitChildren(cursor, capture, None)
        return result

    def source_point(self, location, spelling=False):
        arquivo, linha, coluna = c.c_void_p(), c.c_uint(), c.c_uint()
        localizar = self.getSpellingLocation if spelling else self.getExpansionLocation
        localizar(location, c.byref(arquivo), c.byref(linha), c.byref(coluna), None)
        return arquivo.value, linha.value, coluna.value

    def point(self, cursor):
        location = self.getCursorLocation(cursor)
        if not self.Location_isFromMainFile(location):
            return None
        expansion = self.source_point(location)
        if expansion != self.source_point(location, spelling=True):
            return None
        return expansion[1:]

    def reference_point(self, cursor):
        arquivo, linha, coluna = self.source_point(self.getCursorLocation(cursor), spelling=True)
        return [self.text(self.getFileName(arquivo)), linha, coluna] if arquivo else None

    def qualified_name(self, cursor, nome):
        partes = [nome]
        pai = self.getCursorSemanticParent(cursor)
        while pai.kind in {2, 3, 4, 5, 22, 31, 32}:
            nome_pai = self.text(self.getCursorSpelling(pai))
            if nome_pai:
                partes.append(nome_pai)
            pai = self.getCursorSemanticParent(pai)
        return "::".join(reversed(partes))

    def call_point(self, filhos, identidade):
        if not filhos:
            return None
        # A primeira subárvore denota o callee; o restante são os argumentos.
        candidatos = [filhos[0]]
        while candidatos:
            candidato = candidatos.pop()
            if candidato.kind == CALL:
                continue
            if candidato.kind in {DECL_REF, MEMBER_REF}:
                ref = self.getCursorReferenced(candidato)
                if identidade == self.text(self.getCursorUSR(ref)):
                    return self.point(candidato)
            candidatos.extend(reversed(self.children(candidato)))
        return None

    def diagnostics(self, unit):
        erros = []
        for idx in range(self.getNumDiagnostics(unit)):
            diagnostic = self.getDiagnostic(unit, idx)
            try:
                if self.getDiagnosticSeverity(diagnostic) >= 3 and len(erros) < 3:
                    erros.append(self.text(self.formatDiagnostic(diagnostic, 0)))
            finally:
                self.disposeDiagnostic(diagnostic)
        return erros

    def extract(self, entrada):
        arquivo, diretorio, flags = entrada
        driver = "clang" if arquivo.endswith(".c") else "clang++"
        args = [driver, "--no-default-config", *flags, "-working-directory=" + diretorio]
        argv = (c.c_char_p * len(args))(*(arg.encode() for arg in args))
        index, unit = self.createIndex(0, 0), c.c_void_p()
        try:
            codigo = self.parseTranslationUnit2FullArgv(
                index, arquivo.encode(), argv, len(args), None, 0, 0, c.byref(unit))
            if codigo or not unit:
                return {"erro": f"libclang retornou {codigo}"}
            erros = self.diagnostics(unit)
            if erros:
                return {"erro": "; ".join(erros)}
            funcoes, chamadas = [], []

            def visit(cursor, envolvente=None):
                loc = self.getCursorLocation(cursor)
                if not self.Location_isFromMainFile(loc) or cursor.kind == LAMBDA:
                    return
                if cursor.kind in FUNCOES and self.isCursorDefinition(cursor):
                    identidade = self.text(self.getCursorUSR(cursor))
                    nome = self.text(self.getCursorSpelling(cursor))
                    ponto = self.point(cursor)
                    envolvente = ponto if identidade and nome.isidentifier() and ponto else None
                    if envolvente:
                        funcoes.append([identidade, self.qualified_name(cursor, nome), nome, ponto])
                filhos = self.children(cursor)
                if envolvente and cursor.kind == CALL:
                    ref = self.getCursorReferenced(cursor)
                    if ref.kind in FUNCOES:
                        identidade = self.text(self.getCursorUSR(ref))
                        nome = self.text(self.getCursorSpelling(ref))
                        if identidade and nome.isidentifier():
                            ponto = self.call_point(filhos, identidade) or self.point(cursor)
                            if ponto:
                                chamadas.append([envolvente, ponto, nome, identidade, self.reference_point(ref)])
                for filho in filhos:
                    visit(filho, envolvente)

            for cursor in self.children(self.getTranslationUnitCursor(unit)):
                visit(cursor)
            return {"funcoes": funcoes, "chamadas": chamadas}
        finally:
            if unit:
                self.disposeTranslationUnit(unit)
            self.disposeIndex(index)


def extrair(entrada):
    global api
    if api is None:
        api = Clang()
    return api.extract(entrada)


def main():
    entradas = json.load(sys.stdin)
    versao = Clang()
    print(json.dumps(versao.text(versao.getClangVersion())))
    trabalhadores = min(len(entradas), os.cpu_count() or 1, 8)
    with ProcessPoolExecutor(trabalhadores, mp_context=multiprocessing.get_context("fork")) as executor:
        for resultado in executor.map(extrair, entradas, chunksize=1):
            print(json.dumps(resultado, ensure_ascii=False))


if __name__ == "__main__":
    main()
