You are reviewing an ambitious personal-learning product and designing the next phase of its build.
Attached is BUNDLE.md: the full design record of "Tutor". It contains the specs (PLAN, PEDAGOGY, DESIGN,
NBACK, LANGUAGES), the interface contracts C1-C10, twelve verified research syntheses (learning science,
Dual N-Back, pedagogy of top-performing school systems, AI tutoring trials, discipline-based pedagogy,
corpus licensing, subject tools, foreign languages, cognition, PhD curricula, technical RAG and
scheduling, corpus exploitation), the fork's commit log and a map of every file written.

<context>
What Tutor is today:
- A Tauri 2 desktop app (Rust shell, React 19 + TypeScript + Tailwind 4) over a forked DeepTutor
  backend (FastAPI, Python) and a Rust retrieval engine (tantivy BM25, LanceDB, pdfium).
- It is built for one adult learning to PhD level in math, English, science (physics, biology, earth and
  space, neuroscience and cognitive science), chemistry, and five languages (Spanish, French, German,
  Japanese, Mandarin).
- An ElevenLabs voice tutor with client tools teaches attempt-first:
  - a server-side hint ladder and confidence ratings;
  - FSRS for atoms and a Beta-posterior mastery model for skills;
  - delayed unassisted measurement.
- It also has a Dual N-Back module with honest claims, an eval harness with train/test splits and
  paired-bootstrap keep/revert, and a designed UI (subject worlds, component and chart kits, motion
  rules).

Hard constraints to respect:
- New development goes into Rust or React where feasible. The DeepTutor Python backend is extended only
  at narrow seams.
- The development PC's GPU is off-limits. A second desktop is planned to hold the library and run local
  embedding and OCR models.
- The owner's library stays indexed locally. Retrieved passages may go to the lesson LLMs.
- Claims must stay evidence-based: no IQ, "brain training" or guaranteed-outcome claims.
- Secrets are never in code. Commits are local; nothing is pushed.
</context>

<your_task>
Work in two parts, and think as long as you need before answering.

PART 1: a deep, skeptical review. Read the whole bundle. For every area below, judge current depth
against the best in the world, cite the bundle section that supports each judgement, and name the
highest-leverage improvements that are feasible for a small team working with AI agents. The areas:
- teaching method and learning science;
- the learner model and scheduling;
- measurement and evals;
- the voice tutor;
- corpus exploitation and retrieval quality;
- question generation and verification;
- each subject track, including the languages;
- Dual N-Back;
- UX, visual design and motion;
- architecture (Rust/React/Tauri/Python);
- reliability, testing, security, privacy and cost.

Separate what is verified from what is assumed, and flag contradictions between documents.

Then assess the business honestly. Could a product grown from this become a large business if it were
positioned and marketed well? Reason from evidence, not enthusiasm:
- Who pays and why (segments such as self-directed advanced learners, graduate and professional-exam
  candidates, language learners, and institutions).
- What is defensible, the competition, and unit economics (LLM, voice-minute, OCR and embedding costs per
  active learner).
- Distribution.
- Licensing and legal limits:
  - the Apache-2.0 fork obligations;
  - the fact that a user's own library cannot ship with the product;
  - OpenCode's internal-use terms;
  - provider data terms;
  - claims law for cognitive or educational outcomes.
- Accessibility, and what the evidence says the product can truthfully promise.

Do not conclude "yes" to be encouraging. Keep iterating on the product design, deepening it area by area,
until you can make a defensible case that it could support a large business. Or, if you cannot, state
precisely what would have to become true (milestones, metrics, evidence, and kill criteria) for that
case to hold.

PART 2: write the build prompt. Write one self-contained prompt for Claude Opus 5.5 running in Claude Code
on this repository, to carry out the improvements from Part 1. Follow Anthropic's published prompting
guidance for Claude:
- State the goal, context and motivation explicitly, and say why each rule exists.
- Use XML tags to separate context, constraints, tasks and output format.
- Say what to do rather than only what to avoid.
- Ask explicitly for thorough, high-quality, beyond-the-basics work where depth matters, and for minimal,
  surgical changes where it does not.
- Define verifiable done-checks (tests, builds, evals, screenshots) for every task, and tell it to verify
  before claiming completion.
- For long-horizon work, tell it to keep state in files (the plan and contracts), work in phases, and
  commit at each verified step.
- Tell it to use parallel subagents for independent work, with Opus 5.5 for hard engineering and design
  decisions and Sonnet 5.5 for bounded implementation, research and tests, each with a self-contained
  brief.
- Do not use assistant prefill or instructions that assume a specific thinking budget.

The build prompt must:
- Prioritise by impact on real learning outcomes first, product quality second and commercial readiness
  third.
- Specify each improvement at contract level (data shapes, interfaces, algorithms, UI states), with its
  evidence and its eval gate.
- Prefer Rust and React for new components.
- Stay within every constraint above.
- Include a short business-readiness workstream only where it changes what gets built: telemetry that
  respects privacy, onboarding, packaging and updates, and the licence audit.
</your_task>

<output_format>
1. Review: one section per area, each with current depth (1-5), key gaps, and the three highest-leverage
   improvements with their evidence.
2. Business case: the verdict, the reasoning, unit economics as ranges with stated assumptions,
   the risks, and the milestones and kill criteria.
3. Contradictions and unverified assumptions found in the bundle.
4. The Opus 5.5 build prompt in a single fenced block, ready to paste. It should be long enough to be
   complete: phases, contracts, done-checks and eval gates.
</output_format>
