import csv, json, os, glob, math, sys

THRESHOLD_US = float(os.environ.get('P95_US', '50'))
rows = []

# 1) Tenta p95 a partir de raw.csv (amostras)
for raw in glob.glob('target/criterion/**/raw.csv', recursive=True):
    sub = raw.split('target/criterion/')[-1]
    parts = [p for p in sub.split('/') if p]
    group = parts[0] if len(parts) > 0 else 'unknown'
    func  = parts[1] if len(parts) > 1 else 'all'
    try:
        with open(raw, newline='') as f:
            r = csv.DictReader(f)
            samples_ns = []
            for row in r:
                for key in ('time', 'sample', 'value'):
                    v = row.get(key)
                    if v:
                        try:
                            ns = float(v)
                            samples_ns.append(ns)
                            break
                        except ValueError:
                            pass
            if samples_ns:
                samples_ns.sort()
                idx = max(0, min(len(samples_ns) - 1, int(math.ceil(0.95 * len(samples_ns)) - 1)))
                p95_ns = samples_ns[idx]
                rows.append((f"{group}/{func}", p95_ns / 1000.0))  # µs
    except Exception as e:
        print(f"warn: falha lendo {raw}: {e}", file=sys.stderr)

# 2) Se não houve raw, tenta estimates.json (aproxima p95 via normal)
if not rows:
    for est in glob.glob('target/criterion/**/estimates.json', recursive=True):
        sub = est.split('target/criterion/')[-1]
        parts = [p for p in sub.split('/') if p]
        group = parts[0] if len(parts) > 0 else 'unknown'
        func  = parts[1] if len(parts) > 1 else 'all'
        try:
            with open(est) as f:
                data = json.load(f)
            mean_ns = None
            std_ns  = None

            if isinstance(data.get('mean'), dict) and 'point_estimate' in data['mean']:
                mean_ns = float(data['mean']['point_estimate'])
            if isinstance(data.get('std_dev'), dict) and 'point_estimate' in data['std_dev']:
                std_ns = float(data['std_dev']['point_estimate'])

            if mean_ns is None and isinstance(data.get('median'), dict) and 'point_estimate' in data['median']:
                mean_ns = float(data['median']['point_estimate'])
            if std_ns is None and isinstance(data.get('median_abs_dev'), dict) and 'point_estimate' in data['median_abs_dev']:
                mad = float(data['median_abs_dev']['point_estimate'])
                std_ns = mad / 0.674489750196082  # ~N(0,1): MAD -> σ

            z95 = 1.6448536269514722
            p95_ns = mean_ns if std_ns is None else (mean_ns + z95 * std_ns)
            if mean_ns is not None:
                rows.append((f"{group}/{func}", p95_ns / 1000.0))  # µs
        except Exception as e:
            print(f"warn: falha lendo {est}: {e}", file=sys.stderr)

if not rows:
    print("warn: nenhum benchmark encontrado em target/criterion", file=sys.stderr)
    sys.exit(0)

rows.sort(key=lambda x: x[0])

print("bench,p95_us,threshold_us,ok")
any_fail = False
for name, us in rows:
    ok = us <= THRESHOLD_US
    print(f"{name},{us:.3f},{THRESHOLD_US:.3f},{'true' if ok else 'false'}")
    if not ok:
        any_fail = True

sys.exit(1 if any_fail else 0)
