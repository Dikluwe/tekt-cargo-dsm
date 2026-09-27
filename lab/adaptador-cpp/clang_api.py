# Crystalline Lineage
# @prompt lab/adaptador-cpp/README.md
# @layer Lab
"""Binding mínimo da API C oficial libclang; sem pacote Python externo."""
import ctypes as C
from pathlib import Path


class String(C.Structure):
    _fields_ = [("data", C.c_void_p), ("private_flags", C.c_uint)]


class Cursor(C.Structure):
    _fields_ = [("kind", C.c_uint), ("xdata", C.c_int), ("data", C.c_void_p * 3)]


class Location(C.Structure):
    _fields_ = [("ptr_data", C.c_void_p * 2), ("int_data", C.c_uint)]


Visitor = C.CFUNCTYPE(C.c_uint, Cursor, Cursor, C.c_void_p)


class Clang:
    def __init__(self, library):
        self.lib = C.CDLL(str(library))
        signatures = {
            "getCString": (C.c_char_p, [String]), "disposeString": (None, [String]),
            "getClangVersion": (String, []),
            "createIndex": (C.c_void_p, [C.c_int, C.c_int]), "disposeIndex": (None, [C.c_void_p]),
            "parseTranslationUnit2FullArgv": (C.c_int, [C.c_void_p, C.c_char_p, C.POINTER(C.c_char_p), C.c_int, C.c_void_p, C.c_uint, C.c_uint, C.POINTER(C.c_void_p)]),
            "disposeTranslationUnit": (None, [C.c_void_p]),
            "getTranslationUnitCursor": (Cursor, [C.c_void_p]),
            "visitChildren": (C.c_uint, [Cursor, Visitor, C.c_void_p]),
            "getCursorKindSpelling": (String, [C.c_uint]),
            "getCursorSpelling": (String, [Cursor]), "getCursorDisplayName": (String, [Cursor]),
            "getCursorUSR": (String, [Cursor]),
            "getCursorReferenced": (Cursor, [Cursor]), "getCanonicalCursor": (Cursor, [Cursor]),
            "getSpecializedCursorTemplate": (Cursor, [Cursor]),
            "Cursor_isNull": (C.c_int, [Cursor]), "isDeclaration": (C.c_uint, [C.c_uint]),
            "isCursorDefinition": (C.c_uint, [Cursor]), "CXXMethod_isVirtual": (C.c_uint, [Cursor]),
            "getCursorLocation": (Location, [Cursor]),
            "getExpansionLocation": (None, [Location, C.POINTER(C.c_void_p), C.POINTER(C.c_uint), C.POINTER(C.c_uint), C.POINTER(C.c_uint)]),
            "getFileName": (String, [C.c_void_p]), "getIncludedFile": (C.c_void_p, [Cursor]),
            "getNumDiagnostics": (C.c_uint, [C.c_void_p]),
            "getDiagnostic": (C.c_void_p, [C.c_void_p, C.c_uint]),
            "getDiagnosticSeverity": (C.c_uint, [C.c_void_p]),
            "formatDiagnostic": (String, [C.c_void_p, C.c_uint]), "disposeDiagnostic": (None, [C.c_void_p]),
        }
        for name, (restype, argtypes) in signatures.items():
            fn = getattr(self.lib, "clang_" + name)
            fn.restype, fn.argtypes = restype, argtypes
            setattr(self, name, fn)
        self.version = self.text(self.getClangVersion())

    def text(self, value):
        try:
            raw = self.getCString(value)
            return raw.decode("utf-8", errors="replace") if raw else ""
        finally:
            self.disposeString(value)

    def kind(self, cursor):
        return self.text(self.getCursorKindSpelling(cursor.kind))

    def location(self, cursor, directory):
        file, line, column, offset = C.c_void_p(), C.c_uint(), C.c_uint(), C.c_uint()
        self.getExpansionLocation(self.getCursorLocation(cursor), C.byref(file), C.byref(line), C.byref(column), C.byref(offset))
        if not file:
            return None
        path = Path(self.text(self.getFileName(file)))
        path = (directory / path).resolve() if not path.is_absolute() else path.resolve()
        return dict(file=str(path), line=line.value, column=column.value, offset=offset.value)

    def children(self, cursor):
        result = []
        @Visitor
        def visit(child, parent, data):
            result.append(child)
            return 1  # Continue, recursion controlled by the Python visitor.
        self.visitChildren(cursor, visit, None)
        return result

    def parse(self, source, argv):
        index = self.createIndex(0, 0)
        unit = C.c_void_p()
        arguments = (C.c_char_p * len(argv))(*(arg.encode() for arg in argv))
        # DetailedPreprocessingRecord: include directives and macro expansions.
        code = self.parseTranslationUnit2FullArgv(index, str(source).encode(), arguments, len(argv), None, 0, 1, C.byref(unit))
        return index, unit, code
