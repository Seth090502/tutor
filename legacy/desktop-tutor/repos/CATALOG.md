# Agentic-teaching repo catalog (24, curated)

Policy: catalog first, clone opt-in. Full bulk clone of 10–50 repos into OneDrive
would cost GBs + constant sync churn, so V1.0 ships the catalog + a safe
shallow-clone script. Clone 2–3 you will actually read.

## Clone (shallow, opt-in)

```powershell
python scripts/clone_repos.py --list
python scripts/clone_repos.py --clone autogen
python scripts/clone_repos.py --clone llms-from-scratch --clone llm-zoomcamp
# full set only if you mean it (large):
python scripts/clone_repos.py --clone-all
```

Clones go to `repos/clones/` (gitignored).

## What to steal from each (teaching lens)

- Agent loops: autogen, langgraph, openai-agents-python, babyagi, crewai, agno
- Memory/progress: mem0 vs `tutor/store.py` (local SM-2 JSON)
- Typed items/evals: pydantic-ai, openai-cookbook, gemini-cookbook
- Curriculum design: generative-ai-for-beginners, ai-for-beginners, llm-zoomcamp, llm-course, llms-from-scratch
- Corpus model: free-programming-books (index links, not full text)
- Optional UI/model: open-webui, khoj, transformers, python-genai

See `catalog.json` for URLs + one-line why for all 24.
