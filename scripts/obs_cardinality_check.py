import json
import sys
import time
from urllib.error import HTTPError, URLError
from urllib.request import urlopen

BASE_URL = "http://localhost:9090"
METRICS = [
    "data_freshness_seconds",
    "ce:data_freshness_seconds:max_by_source"
]
WHITELIST = {"__name__", "source", "domain", "service", "env"}
LIMIT = 200
OUTPUT_PATH = "out/obs_gatecheck/evidence/t7_cardinality.json"


def fetch_series(metric, start_ts, end_ts):
    encoded_metric = metric.replace(":", "%3A")
    url = f"{BASE_URL}/api/v1/series?match[]={encoded_metric}&start={start_ts}&end={end_ts}"
    try:
        with urlopen(url, timeout=10) as response:
            payload = response.read()
    except (HTTPError, URLError):
        return None, "http"
    try:
        data = json.loads(payload.decode("utf-8"))
    except json.JSONDecodeError:
        return None, "http"
    if data.get("status") != "success" or not isinstance(data.get("data"), list):
        return None, "http"
    return data["data"], None


def normalize_metric(metric_entry):
    ordered = {}
    for key in sorted(metric_entry):
        ordered[key] = metric_entry[key]
    return ordered


def evaluate():
    now = int(time.time())
    start_ts = now - 3600
    results = {
        "obs": "OBS-6",
        "thread": "T7",
        "labels_whitelist": ["source", "domain", "service", "env"],
        "limits": {"max_series_per_metric": LIMIT},
        "metrics": {
            "data_freshness_seconds": {"series": 0, "violations": []},
            "ce:data_freshness_seconds:max_by_source": {"series": 0, "violations": []}
        },
        "ok": False,
        "reason": ""
    }
    overall_ok = True
    flags = {"labels": False, "count": False, "empty": False, "http": False}
    for metric in METRICS:
        entries, error = fetch_series(metric, start_ts, now)
        metric_result = results["metrics"][metric]
        if error is not None:
            overall_ok = False
            flags["http"] = True
            continue
        series_count = len(entries)
        metric_result["series"] = series_count
        violations = []
        for entry in entries:
            invalid = []
            for key in entry:
                if key not in WHITELIST:
                    invalid.append(key)
            if invalid:
                violation = {
                    "metric": normalize_metric(entry),
                    "invalid_labels": sorted(invalid)
                }
                violations.append(violation)
        if violations:
            violations.sort(key=lambda item: json.dumps(item, sort_keys=True))
        metric_result["violations"] = violations
        if series_count == 0:
            overall_ok = False
            flags["empty"] = True
        elif series_count > LIMIT:
            overall_ok = False
            flags["count"] = True
        if violations:
            overall_ok = False
            flags["labels"] = True
    reason = ""
    if not overall_ok:
        for key in ["labels", "count", "empty", "http"]:
            if flags[key]:
                reason = key
                break
    results["ok"] = overall_ok
    results["reason"] = reason
    with open(OUTPUT_PATH, "w", encoding="utf-8") as handle:
        json.dump(results, handle, indent=2, ensure_ascii=False)
        handle.write("\n")
    return 0 if overall_ok else 5


if __name__ == "__main__":
    sys.exit(evaluate())
