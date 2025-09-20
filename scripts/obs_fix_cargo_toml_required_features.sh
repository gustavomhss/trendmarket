#!/usr/bin/env bash
set -euo pipefail
T="Cargo.toml"
cp "$T" "$T.bak"

python3 - "$T" <<'PY'
import io, sys, re

path = sys.argv[1]
s = io.open(path, encoding="utf-8").read().splitlines()

out = []
in_deps = False
in_bin = False
bin_buf = []
bins = []

for i, line in enumerate(s):
    if re.match(r'^\s*\[dependencies\]\s*$', line):
        in_deps = True
        in_bin = False
        out.append(line)
        continue

    if re.match(r'^\s*\[\[bin\]\]\s*$', line):
        in_bin = True
        in_deps = False
        if bin_buf:
            bins.append(bin_buf)
            bin_buf = []
        bin_buf = [line]
        continue

    if re.match(r'^\s*\[', line) and not re.match(r'^\s*\[\[bin\]\]\s*$', line):
        if in_bin:
            bins.append(bin_buf)
            bin_buf = []
        in_bin = False
        in_deps = False
        out.append(line)
        continue

    if in_bin:
        bin_buf.append(line)
        continue

    if in_deps and re.match(r'^\s*required-features\s*=', line):
        continue

    out.append(line)

if bin_buf:
    bins.append(bin_buf)

bins_clean = []
for b in bins:
    name = None
    for ln in b:
        m = re.match(r'^\s*name\s*=\s*"([^"]+)"', ln)
        if m:
            name = m.group(1)
            break
    if name != "obs_demo":
        bins_clean.append(b)

txt = ("\n".join(out)).rstrip() + "\n"

for b in bins_clean:
    block = "\n".join(b).rstrip() + "\n"
    txt += block

txt += (
    "\n[[bin]]\n"
    "name = \"obs_demo\"\n"
    "path = \"src/bin/obs_demo.rs\"\n"
    "required-features = [\"obs-bin\"]\n"
)

io.open(path, "w", encoding="utf-8").write(txt)
print("OK")
PY

echo "OK"
