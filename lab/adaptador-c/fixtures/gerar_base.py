#!/usr/bin/env python3
# Crystalline Lineage
# @prompt lab/adaptador-c/CONTRATO.md
# @layer Lab
# @updated 2026-09-26
"""Materializa paths absolutos da fixture; não executa compilador ou código C."""
import json
import pathlib
import sys

root = pathlib.Path(__file__).resolve().parent
output = pathlib.Path(sys.argv[1]).resolve()
output.mkdir(parents=True, exist_ok=True)
entries = []
for source, flags in [("main.c", []), ("main.c", ["-DFEATURE=1"]), ("missing.c", [])]:
    entries.append({
        "directory": str(root),
        "file": source,
        "arguments": ["clang", "-std=c11", "-Iinclude", "-isystem", "system",
                      *flags, "-c", source, "-o", "unused.o"],
    })
(output / "compile_commands.json").write_text(json.dumps(entries, indent=2) + "\n")
