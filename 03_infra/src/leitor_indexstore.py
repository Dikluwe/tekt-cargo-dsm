# Crystalline Lineage
# @prompt 00_nucleo/prompts/infra-leitor-indexstore.md
# @prompt-hash 03a6cdb1
# @layer L3
# @updated 2026-11-26

"""Read compiler IndexStore records through the public C API."""
import ctypes as c
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys


class StringRef(c.Structure):
    _fields_ = [("data", c.c_void_p), ("length", c.c_size_t)]

    def bytes(self):
        return c.string_at(self.data, self.length) if self.length else b""

    def text(self):
        return self.bytes().decode("utf-8")


def biblioteca():
    if caminho := os.getenv("LENTE_LIB_INDEXSTORE"):
        return Path(caminho).resolve(strict=True)
    executaveis = []
    if sys.platform == "darwin" and shutil.which("xcrun"):
        saida = subprocess.run(["xcrun", "--find", "swiftc"], text=True,
                               capture_output=True, check=False)
        if saida.returncode == 0:
            executaveis.append(saida.stdout.strip())
    if swiftc := shutil.which("swiftc"):
        executaveis.append(swiftc)
    for executavel in executaveis:
        pasta = Path(executavel).resolve().parent.parent / "lib"
        for nome in ("libIndexStore.so", "libIndexStore.dylib"):
            candidato = pasta / nome
            if candidato.is_file():
                return candidato
    raise ValueError("libIndexStore não localizada; indique LENTE_LIB_INDEXSTORE")


PTR = c.c_void_p
UNIDADE = c.CFUNCTYPE(c.c_bool, PTR, StringRef)
ITEM = c.CFUNCTYPE(c.c_bool, PTR, PTR)
DEFINICAO, REFERENCIA, CHAMADA, IMPLICITA = 1 << 1, 1 << 2, 1 << 5, 1 << 8
CHAMADO_POR, CONTIDO_EM = 1 << 13, 1 << 16
FUNCOES = {12, 16, 17, 18, 22, 23, 24}
FONTES = {".swift", ".c", ".C", ".cc", ".cpp", ".cp", ".cxx", ".h", ".hh", ".hpp", ".hxx", ".m", ".mm"}


class IndexStore:
    def __init__(self, raiz):
        lib = c.CDLL(str(biblioteca()))
        self.raiz = raiz

        def ligar(nome, retorno, *args):
            fn = getattr(lib, "indexstore_" + nome)
            fn.restype, fn.argtypes = retorno, args
            setattr(self, nome, fn)

        erro = c.POINTER(PTR)
        ligar("error_get_description", c.c_char_p, PTR)
        ligar("error_dispose", None, PTR)
        ligar("store_create", PTR, c.c_char_p, erro)
        ligar("store_dispose", None, PTR)
        ligar("store_units_apply_f", c.c_bool, PTR, c.c_uint, PTR, UNIDADE)
        ligar("unit_reader_create", PTR, PTR, c.c_char_p, erro)
        ligar("unit_reader_dispose", None, PTR)
        ligar("unit_reader_get_working_dir", StringRef, PTR)
        ligar("unit_reader_dependencies_apply_f", c.c_bool, PTR, PTR, ITEM)
        ligar("unit_dependency_get_kind", c.c_uint, PTR)
        ligar("unit_dependency_is_system", c.c_bool, PTR)
        ligar("unit_dependency_get_filepath", StringRef, PTR)
        ligar("unit_dependency_get_name", StringRef, PTR)
        ligar("record_reader_create", PTR, PTR, c.c_char_p, erro)
        ligar("record_reader_dispose", None, PTR)
        ligar("record_reader_occurrences_apply_f", c.c_bool, PTR, PTR, ITEM)
        ligar("occurrence_get_roles", c.c_uint64, PTR)
        ligar("occurrence_get_symbol", PTR, PTR)
        ligar("occurrence_get_line_col", None, PTR, c.POINTER(c.c_uint), c.POINTER(c.c_uint))
        ligar("occurrence_relations_apply_f", c.c_bool, PTR, PTR, ITEM)
        ligar("symbol_relation_get_roles", c.c_uint64, PTR)
        ligar("symbol_relation_get_symbol", PTR, PTR)
        ligar("symbol_get_kind", c.c_uint, PTR)
        ligar("symbol_get_language", c.c_uint, PTR)
        ligar("symbol_get_name", StringRef, PTR)
        ligar("symbol_get_usr", StringRef, PTR)

    def abrir(self, funcao, *args):
        erro = PTR()
        objeto = funcao(*args, c.byref(erro))
        if objeto:
            return objeto
        if not erro:
            raise ValueError("não foi possível ler o IndexStore")
        try:
            raise ValueError(self.error_get_description(erro).decode("utf-8", "replace"))
        finally:
            self.error_dispose(erro)

    @staticmethod
    def coletar(funcao, tipo_callback, extrair, *args):
        itens, falhas = [], []

        @tipo_callback
        def callback(_, item):
            try:
                valor = extrair(item)
                if valor is not None:
                    itens.append(valor)
                return True
            except (OSError, ValueError) as erro:
                falhas.append(erro)
                return False

        if not funcao(*args, None, callback):
            raise falhas[0] if falhas else ValueError("IndexStore incompleto")
        return itens

    def registros(self, loja, nome):
        unidade = self.abrir(self.unit_reader_create, loja, nome)
        try:
            diretorio = Path(self.unit_reader_get_working_dir(unidade).text())

            def registro(dep):
                if self.unit_dependency_get_kind(dep) != 2 or self.unit_dependency_is_system(dep):
                    return None
                arquivo = diretorio / self.unit_dependency_get_filepath(dep).text()
                if arquivo.suffix not in FONTES:
                    return None
                real = arquivo.resolve()
                if not real.is_relative_to(self.raiz):
                    return None
                return (self.unit_dependency_get_name(dep).bytes(), real.relative_to(self.raiz).as_posix())

            return self.coletar(self.unit_reader_dependencies_apply_f, ITEM, registro, unidade)
        finally:
            self.unit_reader_dispose(unidade)

    def ocorrencia(self, item):
        papel = self.occurrence_get_roles(item)
        if papel & IMPLICITA or not papel & (DEFINICAO | REFERENCIA | CHAMADA):
            return None
        simbolo = self.occurrence_get_symbol(item)
        if self.symbol_get_kind(simbolo) not in FUNCOES:
            return None
        usr = self.symbol_get_usr(simbolo).text()
        if not usr:
            return None
        linha, coluna = c.c_uint(), c.c_uint()
        self.occurrence_get_line_col(item, c.byref(linha), c.byref(coluna))

        def contexto(relacao):
            papeis = self.symbol_relation_get_roles(relacao)
            if not papeis & (CHAMADO_POR | CONTIDO_EM):
                return None
            origem = self.symbol_relation_get_symbol(relacao)
            if self.symbol_get_kind(origem) in FUNCOES:
                return bool(papeis & CHAMADO_POR), self.symbol_get_usr(origem).text()
            return None

        chamadores = []
        if papel & CHAMADA:
            relacoes = self.coletar(self.occurrence_relations_apply_f, ITEM, contexto, item)
            chamadores = sorted({usr for direto, usr in relacoes if direto} or {usr for _, usr in relacoes})
        return dict(usr=usr, nome=self.symbol_get_name(simbolo).text(),
                    linguagem=self.symbol_get_language(simbolo), linha=linha.value,
                    coluna=coluna.value, definicao=bool(papel & DEFINICAO),
                    chamada=bool(papel & CHAMADA), chamadores=chamadores)

    def documentos(self, caminho):
        loja = self.abrir(self.store_create, os.fsencode(caminho))
        try:
            unidades = self.coletar(self.store_units_apply_f, UNIDADE, StringRef.bytes, loja, 1)
            registros = {par for unidade in unidades for par in self.registros(loja, unidade)}
            for registro, arquivo in sorted(registros):
                leitor = self.abrir(self.record_reader_create, loja, registro)
                try:
                    locais = self.coletar(self.record_reader_occurrences_apply_f, ITEM,
                                          self.ocorrencia, leitor)
                    yield dict(arquivo=arquivo, ocorrencias=locais)
                finally:
                    self.record_reader_dispose(leitor)
        finally:
            self.store_dispose(loja)


if __name__ == "__main__":
    try:
        raiz = Path(sys.argv[1])
        json.dump(list(IndexStore(raiz).documentos(sys.argv[2])), sys.stdout)
    except (OSError, ValueError, AttributeError) as erro:
        sys.exit(str(erro))
