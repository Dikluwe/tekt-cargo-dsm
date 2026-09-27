# Crystalline Lineage
# @prompt lab/adaptador-cpp/README.md
# Fixture generator: paths in a compilation database must match the local checkout.
import json
from pathlib import Path


def create(root, destination):
    root = Path(root).resolve()
    entries = [dict(directory=str(root), file=f"src/{name}.cpp", arguments=["clang++", "-std=c++17", "-Iinclude", "-DFIXTURE_CONFIG=1", "-c", f"src/{name}.cpp", "-o", f"{name}.o"]) for name in ("one", "two", "definitions")]
    Path(destination).write_text(json.dumps(entries, indent=2) + "\n")
    return entries


if __name__ == "__main__":
    import sys
    create(Path(__file__).parent / "fixtures", sys.argv[1])
