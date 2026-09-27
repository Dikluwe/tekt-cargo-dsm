# Crystalline Lineage
# @prompt lab/avaliacao-adaptadores/CONTRATO.md
"""Consulta versão da biblioteca explícita; não instala nem compila nada."""
import ctypes
import sys

class CXString(ctypes.Structure):
    _fields_ = [("data", ctypes.c_void_p), ("private_flags", ctypes.c_uint)]

lib = ctypes.CDLL(sys.argv[1])
lib.clang_getClangVersion.restype = CXString
lib.clang_getCString.argtypes = [CXString]
lib.clang_getCString.restype = ctypes.c_char_p
lib.clang_disposeString.argtypes = [CXString]
version = lib.clang_getClangVersion()
try:
    print(lib.clang_getCString(version).decode("utf-8"))
finally:
    lib.clang_disposeString(version)
