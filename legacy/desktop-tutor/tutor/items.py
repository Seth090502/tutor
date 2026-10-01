"""Item validation (pydantic-ai pattern, stdlib only): typed quiz items without deps."""

REQUIRED = (
    "id",
    "domain",
    "topic",
    "difficulty",
    "prompt",
    "answer",
    "explanation",
    "misconception",
)
DOMAINS = ("math", "english", "science", "chemistry")


def validate_item(it):
    """Return list of error strings (empty = valid)."""
    errs = []
    if not isinstance(it, dict):
        return ["item must be a dict"]
    for k in REQUIRED:
        if k not in it or it[k] in (None, ""):
            errs.append(f"missing {k}")
    if it.get("domain") not in DOMAINS:
        errs.append(f"bad domain {it.get('domain')!r}")
    d = it.get("difficulty")
    if d not in (1, 2, 3):
        errs.append(f"bad difficulty {d!r} (want 1/2/3)")
    choices = it.get("choices") or []
    ans = str(it.get("answer", "")).strip().upper()
    if choices:
        letters = [str(c).strip()[:1].upper() for c in choices]
        if ans not in letters:
            errs.append(f"answer {ans!r} not in choices {letters}")
    if len(str(it.get("prompt", ""))) < 8:
        errs.append("prompt too short (one skill, atomic)")
    return errs


def validate_bank(items):
    """Return {id: [errors]} for invalid items only."""
    bad = {}
    seen = set()
    for it in items:
        iid = it.get("id", "?")
        if iid in seen:
            bad.setdefault(iid, []).append("duplicate id")
        seen.add(iid)
        errs = validate_item(it)
        if errs:
            bad[iid] = errs
    return bad
