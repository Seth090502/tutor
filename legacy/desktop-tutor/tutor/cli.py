"""Tutor CLI V1.0: learn/explain/quiz/popquiz/test/review/progress/search."""

import argparse
import random
import sys
import time

from . import curriculum as CU
from . import question_bank as QB
from . import quiz as QZ
from . import store as ST
from . import corpus as CO


def _run_session(items, state, interactive=True, mode="quiz"):
    correct = 0
    for k, it in enumerate(items, 1):
        print(f"\n[{k}/{len(items)}] ({it['id']} | {it['topic']} | d{it['difficulty']})")
        print(it["prompt"])
        for c in it.get("choices", []) or []:
            print(f"  {c}")
        if interactive:
            try:
                given = input("Your answer (letter, or q to quit): ").strip()
            except EOFError:
                given = ""
            if given.lower() == "q":
                break
        else:
            # non-interactive demo: answer correctly (proves pipeline, not mastery)
            given = it["answer"]
            print(f"  > auto-answer: {given}")
        ok = QZ.check(it, given)
        correct += 1 if ok else 0
        print("  Correct!" if ok else f"  Not quite. Answer: {it['answer']}")
        print(f"  Why: {it['explanation']}")
        print(f"  Watch out: {it['misconception']}")
        ST.record_attempt(state, it["id"], it["domain"], ok)
    ST.save(state)
    grade = QZ.grade_letter(correct, len(items))
    print(f"\n== {mode} done: {correct}/{len(items)} grade {grade} ==")
    return correct


def cmd_learn(args):
    topics = CU.topics_for(args.domain)
    if args.topic and args.topic in topics:
        tid = args.topic
    else:
        mastered = (ST.load().get("mastered_topics", {}).get(args.domain, []))
        tid = CU.next_topic(args.domain, mastered) or next(iter(topics), None)
    if not tid:
        print(f"No curriculum for domain '{args.domain}'. Try: {', '.join(CU.get_domains())}")
        return 1
    t = topics[tid]
    print(f"# {CU.DOMAINS[args.domain]['label']} — {t['label']}  [{tid}]")
    print(f"Prereqs: {', '.join(t.get('prereq', [])) or 'none'} | Level: {t.get('level')}")
    expl = CU.EXPLAINERS.get(tid)
    if expl:
        print(f"\n{expl}")
    else:
        print("\nFirst-principles pass: define terms -> state mechanism -> work one tiny example -> 3 checks.")
        rel = [i for i in QB.by_domain(args.domain) if i["topic"] == tid][:3]
        for i in rel:
            print(f"- Example check [{i['id']}]: {i['prompt']} (ans {i['answer']}; {i['explanation']})")
    print("\nNext: run `python -m tutor.cli quiz --domain "
          f"{args.domain} --n 5` for retrieval practice.")
    return 0


def cmd_explain(args):
    q = (args.topic or "").lower().replace(" ", "")
    key = None
    for k in CU.EXPLAINERS:
        if k in q or q in k:
            key = k
            break
    if key:
        print(f"# {key}\n\n{CU.EXPLAINERS[key]}")
        return 0
    hits = CO.search(args.topic or "", limit=3)
    if hits:
        print(f"# {args.topic} (from local corpus)")
        for h in hits:
            print(f"\n- {h['path']} (score {h['score']}): {h['snippet']}...")
        print("\nSocratic checks: (1) define it in one sentence (2) give one example "
              "(3) state one common misconception.")
        return 0
    print("No local explainer yet. Try topics: quadratics, photosynthesis, stoichiometry; "
          "or `learn --domain math`.")
    return 0


def cmd_quiz(args):
    pool = QB.by_domain(args.domain)
    if not pool:
        print(f"Unknown domain '{args.domain}'. Use: {', '.join(CU.get_domains())}")
        return 1
    items = QZ.select(pool, n=args.n, seed=args.seed)
    state = ST.load()
    _run_session(items, state, interactive=not args.non_interactive, mode="quiz")
    return 0


def cmd_popquiz(args):
    pool = QB.by_domain(args.domain)
    if not pool:
        print(f"Unknown domain '{args.domain}'.")
        return 1
    # pop quiz: 3 random, mixed difficulty, random seed each run
    items = QZ.select(pool, n=3, seed=random.randint(1, 10**9))
    print("== POP QUIZ (3 items, no warm-up) ==")
    state = ST.load()
    _run_session(items, state, interactive=not args.non_interactive, mode="popquiz")
    return 0


def cmd_test(args):
    pool = QB.by_domain(args.domain)
    if not pool:
        print(f"Unknown domain '{args.domain}'.")
        return 1
    # test: stratified by difficulty when possible
    by_d = {1: [], 2: [], 3: []}
    for i in pool:
        by_d.setdefault(i["difficulty"], []).append(i)
    items, rng = [], random.Random(args.seed)
    per = max(1, args.n // 3)
    for d in (1, 2, 3):
        rng.shuffle(by_d.get(d, []))
        items += by_d.get(d, [])[:per]
    rng.shuffle(items)
    items = items[: args.n] or QZ.select(pool, n=args.n, seed=args.seed)
    print(f"== TEST ({args.domain}, {len(items)} items, mixed difficulty) ==")
    state = ST.load()
    correct = _run_session(items, state, interactive=not args.non_interactive, mode="test")
    # mark topic progress hint
    acc = correct / max(1, len(items))
    if acc >= 0.8:
        print("Mastery signal: >=80%. Mark topics reviewed in your notes; re-test in 7 days via `review`.")
    return 0


def cmd_review(args):
    state = ST.load()
    now = int(time.time())
    due = [(iid, m) for iid, m in state.get("items", {}).items() if m.get("due", 0) <= now]
    due.sort(key=lambda kv: kv[1].get("due", 0))
    if not due:
        print("Nothing due. Spaced repetition is working — do a fresh `quiz` instead.")
        return 0
    print(f"== REVIEW: {min(len(due), args.n)} due of {len(due)} tracked ==")
    items = [QB.get(iid) for iid, _ in due[: args.n]]
    items = [i for i in items if i]
    if not items:
        print("Due items reference retired IDs. Starting fresh quiz set.")
        items = QZ.select(QB.ITEMS, n=min(args.n, 5), seed=1)
    _run_session(items, state, interactive=not args.non_interactive, mode="review")
    return 0


def cmd_progress(_args):
    state = ST.load()
    acc = ST.accuracy_by_domain(state)
    print("== PROGRESS ==")
    if not acc:
        print("No attempts yet. Run a quiz first.")
        return 0
    for d in sorted(acc):
        v = acc[d]
        print(f"- {d}: {v['correct']}/{v['n']} ({100*v['acc']:.0f}%)")
    print(f"Tracked items: {len(state.get('items', {}))} | Total attempts: {len(state.get('attempts', []))}")
    weak = min(acc.items(), key=lambda kv: kv[1]["acc"])[0] if acc else None
    if weak:
        print(f"Weakest domain: {weak} -> `python -m tutor.cli quiz --domain {weak} --n 5`")
    return 0


def cmd_agent(args):
    from . import agents as AG
    state = ST.load()
    if args.non_interactive:
        res = AG.run_lesson(args.domain, state, topic=args.topic, n=args.n, seed=args.seed)
    else:
        topic, items = AG.preview_items(args.domain, state, topic=args.topic, n=args.n, seed=args.seed)
        if topic is None:
            print(f"Unknown domain '{args.domain}'. Try: {', '.join(CU.get_domains())}")
            return 1
        expl = CU.EXPLAINERS.get(topic, "")
        if expl:
            print(f"# {topic}\n\n{expl}")
        answers = []
        for it in items:
            print(f"\n({it['id']}) {it['prompt']}")
            for c in it.get("choices", []) or []:
                print(f"  {c}")
            try:
                given = input("Your answer (letter, or q to quit): ").strip()
            except EOFError:
                given = ""
            if given.lower() == "q":
                break
            answers.append(given)
        res = AG.run_lesson(args.domain, state, topic=topic, n=args.n, seed=args.seed, answers=answers)
    # interactive grading pass reuses the same session printer for feedback
    print(f"== AGENT lesson ({args.domain}/{res['trace'][0].get('topic')}) ==")
    for t in res["trace"]:
        if t["node"] == "explain" and t.get("text"):
            print(f"\n# explain\n{t['text']}")
        elif t["node"] == "grade":
            print(f"- {t['item']}: {'ok' if t['correct'] else 'miss'} — {t['why']}")
    print(f"\nlesson: {res['correct']}/{res['total']} | due now: "
          f"{[t for t in res['trace'] if t['node'] == 'review'][0]['due']} | "
          f"next: {[t for t in res['trace'] if t['node'] == 'next'][0]['next_topic']}")
    ST.save(state)
    return 0


def cmd_serve(args):
    from . import api as API
    API.run(port=args.port)
    return 0


def cmd_eval(args):
    from . import eval as EV
    ids = None
    if args.split != "all":
        train, test = EV.split_items(seed=args.seed)
        ids = train if args.split == "train" else test
    r = EV.evaluate(k=args.k, ids=ids)
    print(f"hit@{r['k']} [{args.split}]: {r['hit']}/{r['n']} = {100*r['hit_rate']:.1f}%")
    for m in r["misses"]:
        print(f"  MISS {m['id']}: want {m['expected']}, got {m['got']}")
    return 0


def cmd_search(args):
    hits = CO.search(args.q, limit=args.n)
    if not hits:
        print("No local hits. Add notes to corpus/domains/ or corpus/ingest/ and rebuild index.")
        return 0
    for h in hits:
        print(f"- {h['path']} (score {h['score']}): {h['snippet']}...")
    return 0


def build_parser():
    p = argparse.ArgumentParser(prog="tutor", description="Institutional-grade personal tutor V1.0")
    sub = p.add_subparsers(dest="cmd", required=True)

    a = sub.add_parser("learn", help="Guided topic (prereq-aware)")
    a.add_argument("--domain", default="math", choices=CU.get_domains())
    a.add_argument("--topic", default=None)
    a.set_defaults(func=cmd_learn)

    a = sub.add_parser("explain", help="First-principles explanation")
    a.add_argument("--topic", required=True)
    a.set_defaults(func=cmd_explain)

    for name, fn, hlp in (("quiz", cmd_quiz, "Retrieval quiz"), ("popquiz", cmd_popquiz, "3-item surprise quiz"),
                          ("test", cmd_test, "Graded mixed test"), ("review", cmd_review, "Due spaced items")):
        a = sub.add_parser(name, help=hlp)
        a.add_argument("--domain", default="math", choices=CU.get_domains())
        a.add_argument("--n", type=int, default=5)
        a.add_argument("--seed", type=int, default=7)
        a.add_argument("--non-interactive", action="store_true")
        a.set_defaults(func=fn)

    a = sub.add_parser("progress", help="Accuracy by domain")
    a.set_defaults(func=cmd_progress)

    a = sub.add_parser("search", help="Keyword search local corpus")
    a.add_argument("--q", required=True)
    a.add_argument("--n", type=int, default=5)
    a.set_defaults(func=cmd_search)

    a = sub.add_parser("agent", help="Guided agent lesson (explain->quiz->grade->next)")
    a.add_argument("--domain", default="math", choices=CU.get_domains())
    a.add_argument("--topic", default=None)
    a.add_argument("--n", type=int, default=3)
    a.add_argument("--seed", type=int, default=7)
    a.add_argument("--non-interactive", action="store_true")
    a.set_defaults(func=cmd_agent)

    a = sub.add_parser("serve", help="Start local JSON API for GUIs")
    a.add_argument("--port", type=int, default=8797)
    a.set_defaults(func=cmd_serve)

    a = sub.add_parser("eval-search", help="Ground-truth search eval (hit@k)")
    a.add_argument("--k", type=int, default=3)
    a.add_argument("--split", choices=("all", "train", "test"), default="all")
    a.add_argument("--seed", type=int, default=7)
    a.set_defaults(func=cmd_eval)
    return p


def main(argv=None):
    args = build_parser().parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
