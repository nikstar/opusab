#!/usr/bin/env python3
"""Verify all native binaries depend only on macOS or their own bundle."""
import json
import pathlib
import subprocess
import sys

root = pathlib.Path(sys.argv[1]).resolve()
manifest = json.loads(subprocess.check_output([str(root / "opusab"), "doctor", "--json"]))
for tool in manifest["tools"]:
    path = tool["path"]
    assert path is not None and pathlib.Path(path).is_relative_to(root), f"Unbundled tool: {tool}"
count = 0
for file in root.rglob("*"):
    if not file.is_file() or file.is_symlink():
        continue
    with file.open("rb") as f:
        magic = f.read(4)
    if magic not in (b"\xcf\xfa\xed\xfe", b"\xca\xfe\xba\xbe", b"\xce\xfa\xed\xfe"):
        continue
    result = subprocess.check_output(["otool", "-L", str(file)], text=True)
    # otool -L includes a dylib's own install ID, which is not a dependency.
    ids = subprocess.check_output(["otool", "-D", str(file)], text=True)
    own_ids = {line.strip() for line in ids.splitlines() if not line.endswith(":")}
    for line in result.splitlines():
        if not line.startswith("\t"):
            continue
        dependency = line.strip().split(" (", 1)[0]
        if dependency in own_ids:
            continue
        assert dependency.startswith(("/usr/lib/", "/System/Library/", "@")), (file, dependency)
    count += 1
assert count > 3, "Incomplete bundle"
print(f"Verified {count} Mach-O files and all three bundled tools")
