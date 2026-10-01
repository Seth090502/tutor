"""Summarise docs/phase0/vector-bench-results.jsonl into markdown tables (stdlib only).

Usage: python summarize.py [results.jsonl] [--data d1024]
Recall comes from the 12-thread run with --recall (recall does not depend on threads);
latency columns come from the run with the matching thread count.
"""
import json
import sys
from collections import defaultdict

path = r"C:\Users\sethy\Projects\vault\docs\phase0\vector-bench-results.jsonl"
data_tag = "d1024"
args = sys.argv[1:]
if args and not args[0].startswith("--"):
    path = args.pop(0)
if "--data" in args:
    data_tag = args[args.index("--data") + 1]

recs = [json.loads(l) for l in open(path, encoding="utf-8") if l.strip()]
recs = [r for r in recs if f":{data_tag}:" in r.get("run_id", "") or r.get("run_id", "").endswith(f":{data_tag}")]


def idx_name(r):
    return r["index"]["name"]


def qkey(r):
    q = r["query"]
    if r["candidate"] == "B_usearch":
        return f"ef={q['ef']} cand={q['cand']} {'rescore' if q['rescore'] else 'no-rescore'}"
    return f"nprobes={q['nprobes']} refine={q['refine_factor'] or '-'}"


def mo(r):
    return r["index"].get("metric_override", "native")


def f(x, n=3):
    return "-" if x is None else f"{x:.{n}f}"


builds = [r for r in recs if r["kind"] in ("build", "ingest")]
print("## Builds\n")
print("| candidate | index | build s | threads | file/index GiB | RAM (usearch memory_usage) GiB | peak WS GiB |")
print("|---|---|---|---|---|---|---|")
for r in builds:
    if r["kind"] == "ingest":
        print(f"| A | ingest {r['rows']} rows f16 | {r['ingest_seconds']:.0f} (+{r['scalar_indexes']['build_seconds']:.0f} bitmap) | {r['env']['threads']} | {r['data_bytes']/2**30:.2f} data | - | {r['mem']['peak_working_set_gib']:.2f} |")
    elif r["candidate"] == "B_usearch":
        print(f"| B | {idx_name(r)} | {r['build_seconds']:.0f} | {r['build_threads']} | {r['file_bytes']/2**30:.2f} | {r['memory_usage_bytes']/2**30:.2f} | {r['mem']['peak_working_set_gib']:.2f} |")
    else:
        print(f"| A | {idx_name(r)} | {r['build_seconds']:.0f} | {r['build_threads']} | {r['index_bytes']/2**30:.2f} | - | {r['mem']['peak_working_set_gib']:.2f} |")

pts = [r for r in recs if r["kind"] == "query_point"]
recall = {}
for r in pts:
    if r.get("recall"):
        recall[(idx_name(r), mo(r), qkey(r))] = r["recall"]
lat = defaultdict(dict)
for r in pts:
    lat[(idx_name(r), mo(r), qkey(r))][r["env"]["threads"]] = r

print("\n## Query points (k=100; recall vs exact top-100 of 1000 queries)\n")
print("| index | metric | params | R@10 | R@100 | R@10 centre q | T6 p50 | T6 p95 | T6 p99 | T6 batch4 p95 | T12 p95 | T6 qps c8 | T6 qps c16 | T12 qps c16 | peak WS GiB |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
best = {}
for key in sorted(lat, key=lambda k: (k[0], k[1], k[2])):
    rc = recall.get(key)
    t6, t12 = lat[key].get(6), lat[key].get(12)
    g = lambda r, *p: None if r is None else _get(r, p)

    def _get(r, p):
        x = r
        for k in p:
            if x is None:
                return None
            x = x.get(k) if isinstance(x, dict) else None
        return x

    r10 = rc["all"]["r10"] if rc else None
    r100 = rc["all"]["r100"] if rc else None
    p95_6 = g(t6, "latency_single", "p95_ms")
    print("| " + " | ".join([
        key[0], key[1], key[2], f(r10), f(r100), f(rc["centre"]["r10"] if rc else None),
        f(g(t6, "latency_single", "p50_ms"), 2), f(p95_6, 2), f(g(t6, "latency_single", "p99_ms"), 2),
        f(g(t6, "latency_batch4", "p95_ms"), 2), f(g(t12, "latency_single", "p95_ms"), 2),
        f(g(t6, "throughput", "clients8", "qps"), 0), f(g(t6, "throughput", "clients16", "qps"), 0),
        f(g(t12, "throughput", "clients16", "qps"), 0),
        f(max([x["mem"]["peak_working_set_gib"] for x in lat[key].values()]), 2),
    ]) + " |")
    if r10 is not None and r10 >= 0.95 and r100 >= 0.95 and p95_6 is not None:
        b = best.get((key[0], key[1]))
        if b is None or p95_6 < b[1]:
            best[(key[0], key[1])] = (key[2], p95_6, r10, r100)

print("\n## Cheapest point per index meeting R@10 >= 0.95 and R@100 >= 0.95 (by T6 single-query p95)\n")
print("| index | metric | params | R@10 | R@100 | T6 p95 ms |")
print("|---|---|---|---|---|---|")
for (i, m), (p, l, a, b) in sorted(best.items(), key=lambda x: x[1][1]):
    print(f"| {i} | {m} | {p} | {a:.3f} | {b:.3f} | {l:.2f} |")

fl = [r for r in recs if r["kind"] == "filter_point"]
print("\n## Filtered search (~10% of rows kept)\n")
print("| index | filter | params | threads | run | R@10 | R@100 | R@10 q in field | R@10 q outside | p50 ms (mixed q) | p95 ms (mixed q) | p50 ms (q in field) | p95 ms (q in field) |")
print("|---|---|---|---|---|---|---|---|---|---|---|---|---|")
for r in sorted(fl, key=lambda r: (idx_name(r), r["filter"]["type"], qkey(r), r["env"]["threads"])):
    rc = r.get("recall")
    print("| " + " | ".join([
        idx_name(r), r["filter"]["type"], qkey(r), str(r["env"]["threads"]),
        "filter-only" if ":infield" in r["run_id"] else "main",
        f(rc["all"]["r10"] if rc else None), f(rc["all"]["r100"] if rc else None),
        f(rc["query_in_filter_field"]["r10"] if rc else None), f(rc["query_outside_filter_field"]["r10"] if rc else None),
        f(r["latency_single"]["p50_ms"], 2), f(r["latency_single"]["p95_ms"], 2),
        f((r.get("latency_single_query_in_field") or {}).get("p50_ms"), 2),
        f((r.get("latency_single_query_in_field") or {}).get("p95_ms"), 2),
    ]) + " |")

cold = [r for r in recs if r["kind"] == "cold_start"]
print("\n## Cold start (approximate: fresh process, OS file cache not dropped)\n")
print("| index | metric | threads | load/open s | first query ms | first-20 p50 ms | first-20 max ms |")
print("|---|---|---|---|---|---|---|")
for r in cold:
    ls = r.get("load_seconds", r.get("open_seconds"))
    print(f"| {idx_name(r)} | {mo(r)} | {r['env']['threads']} | {ls:.2f} | {r['first_query_ms']:.1f} | {r['first20']['p50_ms']:.2f} | {r['first20']['max_ms']:.1f} |")
