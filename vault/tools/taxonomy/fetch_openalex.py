"""Snapshot OpenAlex's topic taxonomy (domains, fields, subfields, topics) and the arXiv category list.

Usage: python fetch_openalex.py <out_dir>
Writes openalex/{domains,fields,subfields,topics}.json and arxiv/categories.json, each with fetch metadata.
OpenAlex data is CC0. Keyless requests; one cursor-paged pass over ~4,500 topics.
"""

import html
import json
from pathlib import Path
import re
import sys
import time
import urllib.request

API = "https://api.openalex.org"
UA = {"User-Agent": "vault-taxonomy/1 (personal research tool)"}


def get_json(url):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=UA), timeout=60) as r:
                return json.load(r)
        except Exception as e:  # network hiccup: back off and retry, then fail loudly
            if attempt == 3:
                raise
            time.sleep(2**attempt)
            last = e
    raise last


def fetch_all(entity, select):
    out, cursor = [], "*"
    while cursor:
        page = get_json(f"{API}/{entity}?per_page=200&cursor={cursor}&select={select}")
        out.extend(page["results"])
        cursor = page["meta"].get("next_cursor")
        if not page["results"]:
            break
    return out


def write(path, records, source):
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "source": source,
        "fetched": time.strftime("%Y-%m-%d"),
        "license": "CC0-1.0",
        "count": len(records),
        "records": records,
    }
    path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=1), encoding="utf-8", newline="\n"
    )
    print(f"{path.name}: {len(records)}")


def arxiv_categories():
    with urllib.request.urlopen(
        urllib.request.Request("https://arxiv.org/category_taxonomy", headers=UA), timeout=60
    ) as r:
        page = r.read().decode("utf-8")
    # Entries look like: <h4>cs.AI <span>(Artificial Intelligence)</span></h4>
    cats = []
    for code, name in re.findall(
        r"<h4>\s*([a-z\-]+(?:\.[A-Za-z\-]+)?)\s*<span>\(([^)]*)\)</span>", page
    ):
        cats.append({"code": code, "name": html.unescape(name).strip()})
    return cats


def main():
    out = Path(sys.argv[1])
    oa = out / "openalex"
    write(
        oa / "domains.json", fetch_all("domains", "id,display_name,description"), f"{API}/domains"
    )
    write(
        oa / "fields.json",
        fetch_all("fields", "id,display_name,description,domain"),
        f"{API}/fields",
    )
    write(
        oa / "subfields.json",
        fetch_all("subfields", "id,display_name,description,field,domain"),
        f"{API}/subfields",
    )
    write(
        oa / "topics.json",
        fetch_all(
            "topics", "id,display_name,description,keywords,subfield,field,domain,works_count"
        ),
        f"{API}/topics",
    )
    write(
        out / "arxiv" / "categories.json", arxiv_categories(), "https://arxiv.org/category_taxonomy"
    )


if __name__ == "__main__":
    main()
