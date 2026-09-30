# Foreign languages

The fifth subject, with ElevenLabs as the voice. Evidence: `docs/tutor/research/languages.md`. All
teaching rules in `PEDAGOGY.md` apply. This file adds what is specific to second-language learning.

## Method

1. **Attempt first in the target language.** No translation or gloss before an attempt. The learner may
   ask for help through an on-screen gloss tool, which is logged as assistance.
2. **Brief explicit rule in English, then production.** Instruction works (Kang et al. 2018: g 1.06); the
   explicit/implicit difference is minor, so keep rules short and move to output fast.
3. **Spaced retrieval with FSRS.** Spacing is strong in L2 vocabulary (Kim & Webb 2022, 48 experiments).
   - Each word or phrase gets two cards: recognition (L2 to meaning) and production (meaning to L2, typed
     or spoken). A listening card (audio to meaning) is added for words met in audio input.
   - Retention target 0.9.
   - A spoken card completed with a hint or gloss earns no Good rating.
4. **Production formats by default:** L1-to-L2 translation, sentence cloze, dictation, rubric-scored
   free response. Every item is a full sentence with audio and provenance (source, licence, id), or
   carries an "unverified" flag. It gets a second-pass check before it enters FSRS.
5. **Corrective feedback order** (Lyster & Saito 2010: prompts beat recasts):
   - **Accuracy mode:** clarification request, then repetition, then a metalinguistic clue, then
     elicitation, then explicit correction. At most two corrections per turn.
   - **Fluency mode:** no interruptions. An end-of-session digest of 1-3 errors, each turned into
     production cards. Recasts are used only when a slip breaks comprehension.
6. **Perception-first pronunciation.** A minimal-pair trainer using high-variability phonetic training
   (HVPT: g 0.92 pre-post, 0.67 vs control; Uchihara et al. 2025): several voices, real words, immediate
   feedback, and held-out voices and words at test.
   - Production transfer is smaller (d 0.54), so it is trained and tested separately: listen, repeat,
     compare.
   - ElevenLabs voices stand in for natural talkers, which is untested. They are native-checked and
     labelled until a personal test (3 voices vs 1 on one contrast) runs.
7. **Comprehensible input lane.** Graded texts and audio stories whose known-word coverage is computed
   from the learner's own FSRS state. The target is 95-98% for reading and about 95% for audio. Each text
   ends with a short comprehension check. Reading is extra exposure, not the main source of new cards.
8. **Honest speech claims.**
   - The tutor sees recogniser text, not sound. It says "the recogniser heard X", never "your accent is
     good", and the raw transcript is shown beside any corrected form.
   - Scribe is a machine-listener intelligibility probe, never a pronunciation score.
   - Phoneme-level scoring (Azure Pronunciation Assessment, paid, prosody en-US only) is used only for
     scripted drills in supported locales, as phoneme colouring. There is no single 0-100 score.

## ElevenLabs use

| Use | Model | Notes |
|---|---|---|
| Pre-generated drill, sentence and story audio | Eleven v4 | 90+ languages; the only model documented for non-English IPA pronunciation-dictionary rules. $0.022 per 1k characters during the discount to Oct 12, list $0.08 |
| Long narrative with numbers | Multilingual v2 | 29 languages |
| Live role-play agent | Flash v2.5 or v3 Conversational | Only after testing which model an agent actually uses |
| Speech to text | Scribe v2 | $0.22/h, word timestamps. WER tiers: 34 languages at 5% or less (Spanish, French, German, Italian, Portuguese, Russian, Japanese, and others); Mandarin 5-10%; Arabic, Korean, Hebrew, Thai 10-20% |

- **Per-language config table:** `{lang, variety, tts_model, voice_ids[], speed, dictionary_id, stt_lang}`.
- **QA gate per language:** a 50-sentence native-checked gold set. An "audio unverified" badge shows until
  it passes. A TTS-to-Scribe round trip catches gross errors cheaply.
- **Role-play agent:**
  - One agent per target language, with the language pinned and the language-detection tool off (an
    accent can trigger an unwanted switch).
  - Scenario and level come in through dynamic variables. English help is an on-screen client tool, not a
    spoken switch.
  - Retention is set explicitly to short, and audio storage is off.
- **Audio cache:** generated audio is cached locally (in `data/`), so each sentence is paid for once.

## Measurement

- **Monthly no-AI production hold-out:** speak and write unaided from a fixed prompt, scored on a fixed
  rubric (accuracy, complexity, fluency in words per minute and pauses, intelligibility). This is the
  test for the crutch effect.
- **Delayed retests:** at 7 and 30 days, on new sentences and new voices.
- **Level:** per-skill CEFR-style bands labelled "Tutor estimate, not an official level". Never inferred
  from card accuracy or chat alone.
- **Placement (about 10 minutes):** yes/no vocabulary with pseudowords by frequency band, a short
  read-and-answer block, and a 3-5 minute voice interview.

## Languages

- **First candidates** (top STT tier and deepest open data): Spanish, French, German, Italian.
- **Available but more hours:** Russian, Japanese.
- **Weaker speech tooling:** Mandarin, Arabic, Korean, Hebrew.
- **Reading-only classical languages:** Latin, Ancient Greek, Biblical Hebrew, with no pronunciation
  scoring.
- The owner picks.

## Sources

- **Grounded, licence-tracked corpora per language:** Tatoeba (filtered by rating), Wiktionary via
  Kaikki, Universal Dependencies, Wikisource and Gutenberg for reading, plus the owner's notes.
- **Metadata only:** frequency lists.
- **Not used:** copyrighted textbooks, subtitles or commercial graded readers.
- **Graded text:** open A2-C1 graded readers were not found, so graded text is generated and
  coverage-validated.

## Not building

- Streaks, XP or leagues.
- Certified or chat-inferred CEFR scores; fluency percentages; hours-to-fluency countdowns.
- Pronunciation scores from transcripts.
- Always-on language detection.
- Open free chat without attempt-first.
- Shadowing as a headline feature.
- Local phoneme models (no GPU).

## Open tests before build

1. Scribe and the agent's ASR on the owner's accented beginner speech: does it silently fix errors?
2. Which TTS model an agent uses per language; whether dictionary rules apply inside agents; the real
   per-minute agent cost.
3. The native-checked gold set per language, and an Azure pilot with deliberate mispronunciations.
