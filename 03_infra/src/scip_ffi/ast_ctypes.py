# Crystalline Lineage
# @prompt 00_nucleo/prompts/infra-ast-ctypes.md
# @prompt-hash ecc62f5f
# @layer L3
# @updated 2026-11-26

import ast
import collections
import json
import sys
import tokenize
from io import StringIO


class Extrator(ast.NodeVisitor):
    def __init__(self, arquivo, fonte):
        self.arquivo = arquivo
        self.linhas = fonte.splitlines()
        self.registros = []
        self.chamador = None
        self.locais = {}
        self.sombras = set()
        self.imports = {}
        self.bibliotecas = {}

    def posicao(self, linha, coluna, nome):
        return [self.arquivo, linha, coluna, coluna + len(nome.encode("utf-8"))]

    @staticmethod
    def escritas(corpo):
        nomes = collections.Counter()
        for raiz in corpo:
            for no in ast.walk(raiz):
                if isinstance(no, ast.Name) and isinstance(no.ctx, (ast.Store, ast.Del)):
                    nomes[no.id] += 1
                if isinstance(no, (ast.Attribute, ast.Subscript)) and isinstance(no.value, ast.Name):
                    if isinstance(no.ctx, (ast.Store, ast.Del)):
                        nomes[no.value.id] += 1
                if isinstance(no, (ast.Import, ast.ImportFrom)):
                    nomes.update(n.asname or n.name.split(".")[0] for n in no.names)
                if isinstance(no, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
                    nomes[no.name] += 1
                if isinstance(no, (ast.ExceptHandler, ast.MatchAs, ast.MatchStar, ast.MatchMapping)):
                    nome = getattr(no, "name", None) or getattr(no, "rest", None)
                    if nome:
                        nomes[nome] += 1
        return nomes

    def caminho_cdll(self, no):
        if not isinstance(no, ast.Call) or not no.args:
            return None
        func = no.func
        if isinstance(func, ast.Name):
            valido = func.id not in self.sombras and self.imports.get(func.id) == "CDLL"
        elif isinstance(func, ast.Attribute) and isinstance(func.value, ast.Name):
            nome = func.value.id
            valido = func.attr == "CDLL" and nome not in self.sombras and self.imports.get(nome) == "modulo"
        else:
            valido = False
        arg = no.args[0]
        if valido and isinstance(arg, ast.Constant) and isinstance(arg.value, str):
            return arg.value
        return None

    @staticmethod
    def atribuicao(no):
        if isinstance(no, ast.Assign) and len(no.targets) == 1 and isinstance(no.targets[0], ast.Name):
            return no.targets[0].id, no.value
        if isinstance(no, ast.AnnAssign) and isinstance(no.target, ast.Name):
            return no.target.id, no.value
        return None, None

    def visit_Module(self, no):
        escritas = self.escritas(no.body)
        for stmt in no.body:
            self.visit(stmt)
            self.anotar_import(stmt, escritas)
            nome, valor = self.atribuicao(stmt)
            caminho = self.caminho_cdll(valor)
            if caminho and escritas[nome] == 1:
                self.bibliotecas[nome] = caminho

    def anotar_import(self, no, escritas):
        if isinstance(no, ast.Import):
            nomes = no.names
            esperado, tipo = "ctypes", "modulo"
        elif isinstance(no, ast.ImportFrom) and no.module == "ctypes" and not no.level:
            nomes = no.names
            esperado, tipo = "CDLL", "CDLL"
        else:
            return
        for nome in nomes:
            apelido = nome.asname or nome.name
            if nome.name == esperado and escritas[apelido] == 1:
                self.imports[apelido] = tipo

    def visit_FunctionDef(self, no):
        linha = self.linhas[no.lineno - 1]
        trecho = linha.encode("utf-8")[no.col_offset:].decode("utf-8")
        tokens = tokenize.generate_tokens(StringIO(trecho).readline)
        for token in tokens:
            if token.type == tokenize.NAME and token.string == "def":
                nome = next(tokens)
                inicio = no.col_offset + len(trecho[:nome.start[1]].encode("utf-8"))
                break
        else:
            return
        estado = self.chamador, self.locais, self.sombras
        self.chamador = [no.name, self.posicao(no.lineno, inicio, no.name)]
        self.locais = {}
        escritas = self.escritas(no.body)
        argumentos = {arg.arg for arg in ast.walk(no.args) if isinstance(arg, ast.arg)}
        self.sombras = set(escritas) | argumentos | self.sombras
        for stmt in no.body:
            self.visit(stmt)
            nome, valor = self.atribuicao(stmt)
            caminho = self.caminho_cdll(valor)
            if caminho and escritas[nome] == 1 and nome not in argumentos:
                self.locais[nome] = caminho
        self.chamador, self.locais, self.sombras = estado

    visit_AsyncFunctionDef = visit_FunctionDef

    def visit_Lambda(self, no):
        return

    def visit_Call(self, no):
        func = no.func
        ponte = None
        if isinstance(func, ast.Name):
            local = self.posicao(func.lineno, func.col_offset, func.id)
        elif isinstance(func, ast.Attribute):
            nome = func.attr
            coluna = func.end_col_offset - len(nome.encode("utf-8"))
            local = self.posicao(func.end_lineno, coluna, nome)
            base = func.value
            if isinstance(base, ast.Name):
                caminho = self.locais.get(base.id)
                if base.id not in self.sombras:
                    caminho = caminho or self.bibliotecas.get(base.id)
            else:
                caminho = self.caminho_cdll(base)
            if caminho:
                ponte = {"nome": nome, "linker": nome, "biblioteca": caminho}
        else:
            self.generic_visit(no)
            return
        self.registros.append({"local": local, "chamador": self.chamador, "ponte": ponte})
        self.generic_visit(no)


saida = []
for i, fonte in enumerate(json.load(sys.stdin)):
    try:
        arvore = ast.parse(fonte)
        visitor = Extrator(i, fonte)
        visitor.visit(arvore)
        saida.extend(visitor.registros)
    except (SyntaxError, RecursionError, StopIteration, tokenize.TokenError):
        continue
json.dump(saida, sys.stdout)
