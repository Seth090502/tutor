#!/usr/bin/env python
"""Batched, resumable downloader for the Stage 1 model comparison (Phase 0).

One process, a small thread pool, HTTP Range resume, SHA-256 verification against the
digest published by the host (GitHub release asset digest, Hugging Face LFS oid), and a
manifest at <models>/manifest.json recording what was verified.

Usage: python fetch_assets.py [--tag b11310] [--models DIR] [--bright DIR] [--only llama|models|bright]
Stdlib only.
"""
import argparse, hashlib, json, os, shutil, sys, threading, time, urllib.request, zipfile
from concurrent.futures import ThreadPoolExecutor, as_completed

UA = {"User-Agent": "vault-model-bench-prep"}
MAX_TOTAL_BYTES = 40 * 10**9

# (local subdir, hf repo, file name). Embedders: official Qwen repos. Rerankers: see the prep report.
HF_MODELS = [
    ("embedders", "Qwen/Qwen3-Embedding-0.6B-GGUF", "Qwen3-Embedding-0.6B-Q8_0.gguf"),
    ("embedders", "Qwen/Qwen3-Embedding-0.6B-GGUF", "Qwen3-Embedding-0.6B-f16.gguf"),
    ("embedders", "Qwen/Qwen3-Embedding-4B-GGUF", "Qwen3-Embedding-4B-Q8_0.gguf"),
    ("embedders", "Qwen/Qwen3-Embedding-8B-GGUF", "Qwen3-Embedding-8B-Q8_0.gguf"),
    ("embedders", "ggml-org/bge-m3-Q8_0-GGUF", "bge-m3-q8_0.gguf"),
    ("rerankers", "ggml-org/Qwen3-Reranker-0.6B-Q8_0-GGUF", "qwen3-reranker-0.6b-q8_0.gguf"),
    ("rerankers", "giladgd/Qwen3-Reranker-4B-GGUF", "Qwen3-Reranker-4B.Q8_0.gguf"),
    ("rerankers", "giladgd/Qwen3-Reranker-8B-GGUF", "Qwen3-Reranker-8B.Q8_0.gguf"),
    ("tokenizer", "Qwen/Qwen3-Embedding-0.6B", "tokenizer.json"),
]
BRIGHT_SPLITS = ["psychology", "biology", "economics"]


def jget(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
        return json.load(r)


def log(msg):
    print(time.strftime("%H:%M:%S"), msg, flush=True)


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(8 << 20):
            h.update(chunk)
    return h.hexdigest()


def git_blob_sha1(path):
    h = hashlib.sha1()
    h.update(b"blob %d\x00" % os.path.getsize(path))
    with open(path, "rb") as f:
        while chunk := f.read(8 << 20):
            h.update(chunk)
    return h.hexdigest()


def download(url, dest, size, attempts=8):
    """Resumable download to dest (via dest.part). Returns when dest has `size` bytes."""
    part = dest + ".part"
    os.makedirs(os.path.dirname(dest), exist_ok=True)
    for attempt in range(attempts):
        have = os.path.getsize(part) if os.path.exists(part) else 0
        if have == size:
            break
        if have > size:
            os.remove(part)
            have = 0
        req = urllib.request.Request(url, headers={**UA, "Range": f"bytes={have}-"} if have else UA)
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                if have and r.status != 206:
                    have = 0
                with open(part, "ab" if have else "wb") as f:
                    while chunk := r.read(4 << 20):
                        f.write(chunk)
        except Exception as e:  # retry any transport error, re-raise on the last attempt
            log(f"retry {attempt + 1}/{attempts} {os.path.basename(dest)}: {e!r}")
            if attempt == attempts - 1:
                raise
            time.sleep(min(30, 2 ** attempt))
    if os.path.getsize(part) != size:
        raise RuntimeError(f"size mismatch for {dest}: got {os.path.getsize(part)} want {size}")
    os.replace(part, dest)


def load_manifest(path):
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    return {"files": {}}


_mlock = threading.Lock()


def record(manifest, path, key, entry):
    with _mlock:
        manifest["files"][key] = entry
        with open(path, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2)


def fetch_one(job, manifest, mpath):
    """job: dict(key,url,dest,size,algo,expected,source). Verifies and records."""
    dest, key = job["dest"], job["key"]
    prev = manifest["files"].get(key)
    if prev and prev.get("verified") and os.path.exists(dest) and os.path.getsize(dest) == job["size"] \
            and prev.get("expected") == job["expected"]:
        log(f"skip (already verified) {key}")
        return key
    if not (os.path.exists(dest) and os.path.getsize(dest) == job["size"]):
        log(f"download {key} ({job['size'] / 1e6:.0f} MB)")
        download(job["url"], dest, job["size"])
    got = sha256_file(dest) if job["algo"] == "sha256" else git_blob_sha1(dest)
    ok = got == job["expected"]
    record(manifest, mpath, key, {
        "path": dest, "size": os.path.getsize(dest), "algo": job["algo"], "expected": job["expected"],
        "actual": got, "verified": ok, "source": job["source"], "url": job["url"],
        "fetched_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
    })
    if not ok:
        raise RuntimeError(f"HASH MISMATCH {key}: expected {job['expected']} got {got}")
    log(f"verified {key} {job['algo']}={got[:16]}...")
    return key


def llama_jobs(tag, root):
    rel = jget(f"https://api.github.com/repos/ggml-org/llama.cpp/releases/tags/{tag}")
    want = {
        f"llama-{tag}-bin-win-cuda-13.4-x64.zip": f"llama-{tag}-cuda-13.4",
        # cudart DLLs go next to llama-server.exe in the same folder
        "cudart-llama-bin-win-cuda-13.4-x64.zip": f"llama-{tag}-cuda-13.4",
        f"llama-{tag}-bin-win-vulkan-x64.zip": f"llama-{tag}-vulkan",
        f"llama-{tag}-bin-win-cpu-x64.zip": f"llama-{tag}-cpu",
    }
    jobs, assets = [], {a["name"]: a for a in rel["assets"]}
    for name, folder in want.items():
        a = assets[name]
        assert a["digest"].startswith("sha256:"), a
        jobs.append(dict(key=f"llama.cpp/{tag}/{name}", url=a["browser_download_url"],
                         dest=os.path.join(root, "llama.cpp", "_zips", name), size=a["size"], algo="sha256",
                         expected=a["digest"][7:], source=f"github.com/ggml-org/llama.cpp release {tag}",
                         unzip_to=os.path.join(root, "llama.cpp", folder)))
    return jobs


def hf_jobs(root):
    jobs, revs = [], {}
    for sub, repo, fname in HF_MODELS:
        if repo not in revs:
            info = jget(f"https://huggingface.co/api/models/{repo}")
            revs[repo] = (info["sha"], {t["path"]: t for t in jget(f"https://huggingface.co/api/models/{repo}/tree/main")})
        sha, tree = revs[repo]
        t = tree[fname]
        if t.get("lfs"):
            algo, expected, size = "sha256", t["lfs"]["oid"], t["lfs"]["size"]
        else:
            algo, expected, size = "gitsha1", t["oid"], t["size"]
        dest = os.path.join(root, sub, repo.replace("/", "__") + "__" + fname)
        jobs.append(dict(key=f"hf/{repo}/{fname}", url=f"https://huggingface.co/{repo}/resolve/{sha}/{fname}",
                         dest=dest, size=size, algo=algo, expected=expected, source=f"huggingface.co/{repo}@{sha}"))
    return jobs


def bright_jobs(root):
    info = jget("https://huggingface.co/api/datasets/xlangai/BRIGHT")
    sha = info["sha"]
    jobs = []
    for kind in ("documents", "examples"):
        tree = {t["path"]: t for t in jget(f"https://huggingface.co/api/datasets/xlangai/BRIGHT/tree/main/{kind}")}
        for s in BRIGHT_SPLITS:
            p = f"{kind}/{s}-00000-of-00001.parquet"
            t = tree[p]
            jobs.append(dict(key=f"bright/{p}", url=f"https://huggingface.co/datasets/xlangai/BRIGHT/resolve/{sha}/{p}",
                             dest=os.path.join(root, "raw", kind, f"{s}.parquet"), size=t["lfs"]["size"], algo="sha256",
                             expected=t["lfs"]["oid"], source=f"huggingface.co/datasets/xlangai/BRIGHT@{sha}"))
    return jobs, sha


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--tag", default="b11310")
    ap.add_argument("--models", default=r"C:\Users\sethy\Projects\vault-models")
    ap.add_argument("--bright", default=r"C:\Users\sethy\Projects\vault-eval\external\bright")
    ap.add_argument("--only", choices=["llama", "models", "bright"])
    ap.add_argument("--threads", type=int, default=3)
    a = ap.parse_args()
    os.makedirs(a.models, exist_ok=True)
    os.makedirs(a.bright, exist_ok=True)
    mpath = os.path.join(a.models, "manifest.json")
    manifest = load_manifest(mpath)
    bpath = os.path.join(a.bright, "manifest.json")
    bman = load_manifest(bpath)

    jobs = []  # (job, manifest, manifest path)
    if a.only in (None, "llama"):
        jobs += [(j, manifest, mpath) for j in llama_jobs(a.tag, a.models)]
    if a.only in (None, "models"):
        jobs += [(j, manifest, mpath) for j in hf_jobs(a.models)]
    if a.only in (None, "bright"):
        bj, bsha = bright_jobs(a.bright)
        bman["dataset_revision"] = bsha
        jobs += [(j, bman, bpath) for j in bj]
    total = sum(j["size"] for j, _, _ in jobs)
    free = shutil.disk_usage(a.models).free
    log(f"{len(jobs)} files, {total / 1e9:.2f} GB to fetch; free on drive {free / 1e9:.1f} GB")
    if total > MAX_TOTAL_BYTES or total * 2 > free:
        sys.exit(f"refusing: total {total / 1e9:.1f} GB exceeds the 40 GB budget or free disk")

    failed = []
    with ThreadPoolExecutor(a.threads) as ex:
        futs = {ex.submit(fetch_one, j, m, p): j for j, m, p in jobs}
        for f in as_completed(futs):
            try:
                f.result()
            except Exception as e:
                failed.append((futs[f]["key"], repr(e)))
                log(f"FAILED {futs[f]['key']}: {e!r}")
    failed_keys = {k for k, _ in failed}
    for j, m, _ in jobs:
        if j.get("unzip_to") and j["key"] not in failed_keys:
            marker = os.path.join(j["unzip_to"], f".unzipped-{os.path.basename(j['dest'])}")
            if not os.path.exists(marker):
                log(f"unzip {os.path.basename(j['dest'])} -> {j['unzip_to']}")
                with zipfile.ZipFile(j["dest"]) as z:
                    z.extractall(j["unzip_to"])
                open(marker, "w").close()
    if failed:
        sys.exit("FAILED: " + json.dumps(failed, indent=1))
    log("all files verified")


if __name__ == "__main__":
    main()
