"""Build the vault's category map from an OpenAlex taxonomy snapshot plus our crosswalks.

Usage: python build_categories.py <taxonomy_dir>
Reads  <taxonomy_dir>/openalex/{fields,subfields,topics}.json and <taxonomy_dir>/arxiv/categories.json.
Writes <taxonomy_dir>/categories.json (machine map used by libraryd and intake) and <taxonomy_dir>/CATEGORIES.md.

Design: categories are metadata, never paths. Every OpenAlex topic gets exactly one primary vault category and any
number of secondary ones; psychology and cognitive science topics also get study-map areas. Books and preprints
reach the same map through arXiv, LCC and DDC crosswalks. Every lookup that names an OpenAlex subfield is checked
against the snapshot, so a typo fails the build instead of silently dropping documents.
"""

import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

VERSION = "2026-10-01.1"

# ---------------------------------------------------------------------------------------------------------------
# Vault categories, in the owner's priority order. Targets are proposals for the owner to adjust.
CATEGORIES = [
    dict(slug="psych-cogsci", name="Psychology and cognitive science", priority=1,
         scope="Mind, brain and behaviour: every psychology subfield, cognitive and behavioural neuroscience, "
               "human factors, plus cognition-related work in philosophy, linguistics, AI, economics, education "
               "and psychiatry through the cognitive-science overlay.",
         pilot=(3150, 350), full=(52000, 2800),
         synonyms=["psychology", "psych", "cognitive science", "cogsci", "cognition", "neuroscience",
                   "cognitive neuroscience", "behavioral science", "behavioural science"]),
    dict(slug="engineering", name="Engineering", priority=2,
         scope="All engineering disciplines, chemical engineering, materials science and energy.",
         pilot=(630, 70), full=(17000, 1300),
         synonyms=["engineering", "eng", "materials", "energy", "electrical", "mechanical", "civil"]),
    dict(slug="mathematics", name="Mathematics", priority=2,
         scope="Pure and applied mathematics, statistics and probability, theoretical computer science, and "
               "operations research.",
         pilot=(560, 140), full=(15000, 1600),
         synonyms=["mathematics", "math", "maths", "statistics", "stats", "probability"]),
    dict(slug="medicine", name="Medicine", priority=3,
         scope="Clinical medicine, psychiatry, neurology, pharmacology, nursing, dentistry, health professions "
               "and public health.",
         pilot=(570, 30), full=(18000, 900),
         synonyms=["medicine", "medical", "clinical", "health", "psychiatry", "neurology", "pharmacology"]),
    dict(slug="cs-ai", name="Computer science and AI", priority=4,
         scope="All computer science, including artificial intelligence and machine learning.",
         pilot=(620, 80), full=(18000, 1300),
         synonyms=["computer science", "cs", "ai", "artificial intelligence", "machine learning", "ml",
                   "programming", "software"]),
    dict(slug="natural-sciences", name="Natural sciences", priority=5,
         scope="Physics and astronomy, chemistry, biology and the life sciences, earth and environmental "
               "science.",
         pilot=(450, 50), full=(18000, 1100),
         synonyms=["natural sciences", "science", "physics", "chemistry", "biology", "earth science",
                   "astronomy", "ecology"]),
    dict(slug="humanities", name="Humanities", priority=6,
         scope="Philosophy, history, languages and linguistics, literature, religion, the arts and classics.",
         pilot=(220, 80), full=(8000, 900),
         synonyms=["humanities", "philosophy", "history", "linguistics", "literature", "religion", "arts",
                   "classics"]),
    dict(slug="social-other", name="Social sciences and other", priority=7,
         scope="Everything else (education, economics, business, law, sociology, politics). Not a gathering "
               "target, but nothing delivered is ever left without a category.",
         pilot=(0, 0), full=(4000, 100),
         synonyms=["social sciences", "education", "economics", "business", "law", "sociology", "politics",
                   "other"]),
]
SLUGS = [c["slug"] for c in CATEGORIES]

# ---------------------------------------------------------------------------------------------------------------
# OpenAlex field -> default primary category; (field, subfield) overrides; subfield-level secondary categories.
FIELD_PRIMARY = {
    "Psychology": "psych-cogsci",
    "Neuroscience": "natural-sciences",
    "Engineering": "engineering", "Chemical Engineering": "engineering", "Materials Science": "engineering",
    "Energy": "engineering",
    "Mathematics": "mathematics",
    "Medicine": "medicine", "Nursing": "medicine", "Health Professions": "medicine", "Dentistry": "medicine",
    "Pharmacology, Toxicology and Pharmaceutics": "medicine", "Veterinary": "medicine",
    "Computer Science": "cs-ai",
    "Physics and Astronomy": "natural-sciences", "Chemistry": "natural-sciences",
    "Earth and Planetary Sciences": "natural-sciences", "Agricultural and Biological Sciences": "natural-sciences",
    "Biochemistry, Genetics and Molecular Biology": "natural-sciences", "Environmental Science": "natural-sciences",
    "Immunology and Microbiology": "natural-sciences",
    "Arts and Humanities": "humanities",
    "Social Sciences": "social-other", "Economics, Econometrics and Finance": "social-other",
    "Business, Management and Accounting": "social-other", "Decision Sciences": "social-other",
}
SUBFIELD_PRIMARY = {
    ("Neuroscience", "Cognitive Neuroscience"): "psych-cogsci",
    ("Neuroscience", "Behavioral Neuroscience"): "psych-cogsci",
    ("Neuroscience", "Sensory Systems"): "psych-cogsci",
    ("Neuroscience", "Developmental Neuroscience"): "psych-cogsci",
    ("Neuroscience", "Biological Psychiatry"): "medicine",
    ("Neuroscience", "Neurology"): "medicine",
    ("Social Sciences", "Human Factors and Ergonomics"): "psych-cogsci",
    ("Social Sciences", "Linguistics and Language"): "humanities",
    ("Social Sciences", "Archeology"): "humanities",
    ("Social Sciences", "Cultural Studies"): "humanities",
    ("Social Sciences", "Health"): "medicine",
    ("Decision Sciences", "Statistics, Probability and Uncertainty"): "mathematics",
    ("Decision Sciences", "Management Science and Operations Research"): "mathematics",
    ("Environmental Science", "Environmental Engineering"): "engineering",
    ("Environmental Science", "Water Science and Technology"): "engineering",
    ("Environmental Science", "Waste Management and Disposal"): "engineering",
    ("Biochemistry, Genetics and Molecular Biology", "Cancer Research"): "medicine",
    ("Biochemistry, Genetics and Molecular Biology", "Molecular Medicine"): "medicine",
    ("Biochemistry, Genetics and Molecular Biology", "Clinical Biochemistry"): "medicine",
}
SUBFIELD_SECONDARY = {
    ("Medicine", "Psychiatry and Mental health"): ["psych-cogsci"],
    ("Neuroscience", "Biological Psychiatry"): ["psych-cogsci"],
    ("Social Sciences", "Human Factors and Ergonomics"): ["engineering"],
    ("Social Sciences", "Life-span and Life-course Studies"): ["psych-cogsci"],
    ("Mathematics", "Theoretical Computer Science"): ["cs-ai"],
    ("Computer Science", "Computational Theory and Mathematics"): ["mathematics"],
    ("Computer Science", "Signal Processing"): ["engineering"],
    ("Computer Science", "Hardware and Architecture"): ["engineering"],
    ("Engineering", "Biomedical Engineering"): ["medicine"],
    ("Engineering", "Control and Systems Engineering"): ["cs-ai"],
    ("Health Professions", "Speech and Hearing"): ["psych-cogsci"],
    ("Arts and Humanities", "History and Philosophy of Science"): ["natural-sciences"],
    ("Physics and Astronomy", "Statistical and Nonlinear Physics"): ["mathematics"],
    ("Mathematics", "Mathematical Physics"): ["natural-sciences"],
}

# Cognitive-science overlay: topics outside psych-cogsci become secondary members when their name or keywords
# match this lexicon and their subfield is one where cognitive work actually appears.
COGSCI_SUBFIELDS = {
    "Artificial Intelligence", "Human-Computer Interaction", "Language and Linguistics", "Linguistics and Language",
    "Philosophy", "Education", "Economics and Econometrics", "Psychiatry and Mental health", "Neurology",
    "Biological Psychiatry", "Ecology, Evolution, Behavior and Systematics", "Animal Science and Zoology",
    "Communication", "Anthropology", "Speech and Hearing", "Rehabilitation", "Cellular and Molecular Neuroscience",
    "Physiology", "Computer Vision and Pattern Recognition", "Pediatrics, Perinatology and Child Health",
    "Geriatrics and Gerontology", "Organizational Behavior and Human Resource Management", "Marketing",
    "History and Philosophy of Science", "Developmental Biology", "Endocrine and Autonomic Systems",
}
COGSCI_TERMS = [
    r"\bcogniti", r"\bpercept", r"\battention", r"\bmemory\b", r"\bmemories", r"consciousness", r"\bemotion",
    r"\baffect(ive)?\b", r"decision[- ]making", r"\bjudg(e)?ment", r"\breasoning\b", r"\bheuristic",
    r"language acquisition", r"psycholinguist", r"\bbilingual", r"\breading\b", r"\bdyslexi", r"\bmind\b",
    r"\bmental\b", r"metacogniti", r"executive function", r"working memory", r"categori[sz]ation",
    r"theory of mind", r"\bembodi", r"\blearning and memory\b", r"human learning", r"\bmotivation",
    r"behavio(u)?ral economics", r"\bnudg", r"(?<!artificial )\bintelligence\b", r"\bautism", r"\bschizophreni",
    r"\bdepress", r"\banxiety", r"\bdementia", r"\balzheimer", r"\baphasi", r"\bsleep\b", r"\bpersonality",
    r"\bsocial cognition", r"\bfear\b", r"\breward\b", r"\bhabit", r"\banimal cognition", r"\bcomparative cognition",
    r"neural basis", r"\bbrain\b", r"\bneurocogniti", r"cognitive model", r"\bhuman behavio", r"\bperceptual",
    r"\bvisual (system|processing|cortex)", r"\bspeech perception", r"\bfree will", r"\bphilosophy of mind",
    r"\bintentional", r"\bmoral psychology", r"\bpsycholog",
]
COGSCI_EXCLUDE = [r"\bmemory (allocation|management|systems?|hierarchy|devices?|cells?|chips?)",
                  r"\bcache\b", r"\bflash memory", r"\bmemristor", r"\bshape memory", r"\bmachine reading",
                  r"cognitive (radio|computing and networks)", r"\btumou?r", r"\blymphoma", r"\bsentiment"]
# Education and AI topic names use "learning" for both human and machine learning; for these two subfields the
# overlay also accepts human-learning names, and the machine-learning ones are excluded.
COGSCI_EXTRA = {
    "Education": [r"\blearning\b", r"\bliteracy", r"\bchild development", r"\bcritical thinking",
                  r"\bassessment and feedback", r"\bpedagog"],
    "Artificial Intelligence": [r"intelligent tutoring", r"\bcognitive science", r"\blogic, reasoning"],
}
COGSCI_EXTRA_EXCLUDE = [r"machine learning", r"deep learning", r"\bfew-shot", r"domain adaptation", r"federated"]

# Study map for psychology and cognitive science: areas a PhD-level reader navigates by. Multi-label; the primary
# area is the one with the most matches (ties go to the earlier area).
PSYCH_AREAS = [
    ("perception", "Perception and psychophysics",
     [r"\bpercept", r"\bvisual", r"\bvision\b", r"auditory", r"\bhearing\b", r"multisensory", r"\bsensory",
      r"psychophysic", r"\btactile", r"\bolfact", r"\bmotion perception", r"\bface (perception|recognition)",
      r"\bspeech perception", r"\bpain\b"]),
    ("attention", "Attention, consciousness and cognitive control",
     [r"\battention", r"consciousness", r"\bawareness", r"executive function", r"cognitive control",
      r"\binhibition", r"\bmind[- ]wandering", r"\bvigilance"]),
    ("memory", "Memory and learning",
     [r"\bmemory\b", r"\bmemories", r"\bforgetting", r"\brecall\b", r"\brecognition memory", r"\bencoding",
      r"retrieval practice", r"\bspacing\b", r"\bamnesi", r"\bimplicit learning", r"\bskill acquisition",
      r"\bconditioning", r"\breinforcement\b", r"\bassociative learning", r"\bhabituation"]),
    ("language", "Language and psycholinguistics",
     [r"\blanguage\b", r"psycholinguist", r"\breading\b", r"\bspeech\b", r"\bbilingual", r"\bword\b",
      r"\bsentence", r"\bdyslexi", r"\baphasi", r"\bsign language", r"\bliteracy"]),
    ("thinking", "Reasoning, judgement and decision making",
     [r"\breasoning\b", r"decision[- ]making", r"\bjudg(e)?ment", r"\bheuristic", r"problem[- ]solving",
      r"\bcreativity", r"\bintelligence\b", r"\bbelief", r"\brisk\b", r"\bchoice\b", r"\bcategori[sz]ation",
      r"\bconcept", r"\bnumerical cognition", r"\banalog(y|ical)"]),
    ("emotion-motivation", "Emotion and motivation",
     [r"\bemotion", r"\baffect(ive)?\b", r"\bmood\b", r"\bmotivation", r"\breward\b", r"\bstress\b", r"\bfear\b",
      r"\bwell[- ]being", r"\bhappiness", r"\bself[- ]regulation", r"\bgoal"]),
    ("developmental", "Development across the lifespan",
     [r"\bchild", r"\binfan", r"\badolescen", r"\bdevelopment(al)?\b", r"\baging\b", r"\bageing\b",
      r"\bolder adults", r"\blife[- ]?span", r"\bparenting", r"\battachment\b", r"\bearly childhood"]),
    ("social", "Social and cultural psychology",
     [r"\bsocial\b", r"\battitude", r"\bprejudice", r"\bstereotyp", r"\bgroup", r"\bculture", r"\bcultural",
      r"\bidentity", r"\bpersuasion", r"\bcooperation", r"\bmoral", r"\bintergroup", r"\bclose relationships"]),
    ("personality", "Personality and individual differences",
     [r"\bpersonality", r"\btraits?\b", r"\bindividual differences", r"\bnarcissis", r"\bself[- ]esteem",
      r"\btemperament"]),
    ("clinical", "Clinical psychology and psychopathology",
     [r"\banxiety", r"\bdepress", r"\bdisorder", r"\btherap", r"psychopatholog", r"\bautism", r"\badhd\b",
      r"attention[- ]deficit", r"\bschizophreni", r"\bptsd\b", r"post[- ]?traumatic", r"\baddiction",
      r"\bsubstance use", r"\bsuicid", r"\bpsychosis", r"\beating disorder", r"\bmental health", r"\bbipolar",
      r"\bobsessive", r"\bdementia", r"\btrauma"]),
    ("biological", "Biological psychology and cognitive neuroscience",
     [r"\bbrain\b", r"\bneural\b", r"\bneuro", r"\bcortex", r"\bcortical", r"\bfmri\b", r"\beeg\b",
      r"\bneuroimaging", r"\bhormon", r"\bsleep\b", r"\bdopamin", r"\bhippocamp", r"\bamygdala",
      r"\bsynap", r"\boscillation", r"\bgenetic"]),
    ("methods", "Methods, psychometrics and statistics",
     [r"\bpsychometric", r"\bmeasurement", r"\bscale (development|validation)", r"\bvalidity",
      r"\breliability", r"\bstatistic", r"\bmeta[- ]analys", r"\breplication", r"\bfactor analysis",
      r"\bitem response", r"\bstructural equation", r"\bexperimental design", r"\bopen science"]),
    ("applied", "Applied, educational, organisational and health psychology",
     [r"\beducation", r"\bstudents?\b", r"\bteach", r"\bschool", r"\bworkplace", r"\borganizational",
      r"\bemployee", r"\bleadership", r"\bjob\b", r"\bsport", r"\bhealth behavio", r"\bhuman factors",
      r"\bergonomic", r"\bconsumer", r"\btraffic", r"\bdriving\b", r"\bcounsel"]),
    ("computational", "Computational and formal models of cognition",
     [r"\bcomputational model", r"\bcognitive model", r"\bbayesian", r"\bneural network", r"\bconnectionis",
      r"\breinforcement learning", r"\bpredictive (processing|coding)", r"\bcognitive architecture",
      r"\bartificial intelligence", r"\bdeep learning"]),
    ("foundations", "History, philosophy and foundations of psychology",
     [r"\bphilosoph", r"\bhistory of psychology", r"\bfree will", r"\bconsciousness", r"\bmind[- ]body",
      r"\bepistemolog", r"\bintentional", r"\bphenomenolog", r"\bevolutionary psychology", r"\bevolution"]),
]

# ---------------------------------------------------------------------------------------------------------------
# Crosswalks into OpenAlex subfields. Values are (field, subfield) or a bare category slug when no subfield fits.
ARXIV = {
    "astro-ph.*": ("Physics and Astronomy", "Astronomy and Astrophysics"),
    "cond-mat.*": ("Physics and Astronomy", "Condensed Matter Physics"),
    "cond-mat.stat-mech": ("Physics and Astronomy", "Statistical and Nonlinear Physics"),
    "cond-mat.dis-nn": ("Physics and Astronomy", "Statistical and Nonlinear Physics"),
    "cond-mat.mtrl-sci": ("Materials Science", "General Materials Science"),
    "cs.AI": ("Computer Science", "Artificial Intelligence"), "cs.LG": ("Computer Science", "Artificial Intelligence"),
    "cs.MA": ("Computer Science", "Artificial Intelligence"), "cs.NE": ("Computer Science", "Artificial Intelligence"),
    "cs.CL": ("Computer Science", "Artificial Intelligence"),
    "cs.CV": ("Computer Science", "Computer Vision and Pattern Recognition"),
    "cs.HC": ("Computer Science", "Human-Computer Interaction"),
    "cs.IR": ("Computer Science", "Information Systems"), "cs.DB": ("Computer Science", "Information Systems"),
    "cs.DL": ("Computer Science", "Information Systems"), "cs.SI": ("Computer Science", "Information Systems"),
    "cs.CR": ("Computer Science", "Information Systems"),
    "cs.NI": ("Computer Science", "Computer Networks and Communications"),
    "cs.DC": ("Computer Science", "Computer Networks and Communications"),
    "cs.SE": ("Computer Science", "Software"), "cs.PL": ("Computer Science", "Software"),
    "cs.OS": ("Computer Science", "Software"), "cs.MS": ("Computer Science", "Software"),
    "cs.AR": ("Computer Science", "Hardware and Architecture"), "cs.ET": ("Computer Science", "Hardware and Architecture"),
    "cs.PF": ("Computer Science", "Hardware and Architecture"),
    "cs.GR": ("Computer Science", "Computer Graphics and Computer-Aided Design"),
    "cs.CG": ("Computer Science", "Computer Graphics and Computer-Aided Design"),
    "cs.CC": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.DS": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.FL": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.LO": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.DM": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.GT": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.IT": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.SC": ("Computer Science", "Computational Theory and Mathematics"),
    "cs.SY": ("Engineering", "Control and Systems Engineering"),
    "cs.RO": ("Engineering", "Control and Systems Engineering"),
    "cs.SD": ("Computer Science", "Signal Processing"),
    "cs.NA": ("Mathematics", "Numerical Analysis"),
    "cs.MM": ("Computer Science", "Computer Science Applications"),
    "cs.CY": ("Computer Science", "Computer Science Applications"),
    "cs.CE": ("Computer Science", "Computer Science Applications"),
    "cs.GL": ("Computer Science", "Computer Science Applications"),
    "cs.OH": ("Computer Science", "Computer Science Applications"),
    "econ.*": ("Economics, Econometrics and Finance", "Economics and Econometrics"),
    "eess.AS": ("Computer Science", "Signal Processing"), "eess.SP": ("Computer Science", "Signal Processing"),
    "eess.IV": ("Computer Science", "Computer Vision and Pattern Recognition"),
    "eess.SY": ("Engineering", "Control and Systems Engineering"),
    "gr-qc": ("Physics and Astronomy", "Nuclear and High Energy Physics"),
    "hep-*": ("Physics and Astronomy", "Nuclear and High Energy Physics"),
    "nucl-*": ("Physics and Astronomy", "Nuclear and High Energy Physics"),
    "math.AC": ("Mathematics", "Algebra and Number Theory"), "math.AG": ("Mathematics", "Algebra and Number Theory"),
    "math.NT": ("Mathematics", "Algebra and Number Theory"), "math.RA": ("Mathematics", "Algebra and Number Theory"),
    "math.GR": ("Mathematics", "Algebra and Number Theory"), "math.RT": ("Mathematics", "Algebra and Number Theory"),
    "math.QA": ("Mathematics", "Algebra and Number Theory"), "math.KT": ("Mathematics", "Algebra and Number Theory"),
    "math.CT": ("Mathematics", "Algebra and Number Theory"),
    "math.AT": ("Mathematics", "Geometry and Topology"), "math.GT": ("Mathematics", "Geometry and Topology"),
    "math.DG": ("Mathematics", "Geometry and Topology"), "math.GN": ("Mathematics", "Geometry and Topology"),
    "math.MG": ("Mathematics", "Geometry and Topology"), "math.SG": ("Mathematics", "Geometry and Topology"),
    # OpenAlex has no analysis subfield; its analysis topics sit under Applied Mathematics.
    "math.AP": ("Mathematics", "Applied Mathematics"), "math.CA": ("Mathematics", "Applied Mathematics"),
    "math.CV": ("Mathematics", "Applied Mathematics"), "math.FA": ("Mathematics", "Applied Mathematics"),
    "math.OA": ("Mathematics", "Applied Mathematics"), "math.SP": ("Mathematics", "Applied Mathematics"),
    "math.DS": ("Mathematics", "Applied Mathematics"), "math.OC": ("Mathematics", "Applied Mathematics"),
    "math.CO": ("Mathematics", "Discrete Mathematics and Combinatorics"),
    "math.LO": ("Mathematics", "Theoretical Computer Science"),
    "math.IT": ("Mathematics", "Theoretical Computer Science"),
    "math.PR": ("Mathematics", "Statistics and Probability"), "math.ST": ("Mathematics", "Statistics and Probability"),
    "math.NA": ("Mathematics", "Numerical Analysis"),
    "math.MP": ("Mathematics", "Mathematical Physics"), "math-ph": ("Mathematics", "Mathematical Physics"),
    "math.HO": "mathematics", "math.GM": "mathematics",
    "nlin.*": ("Physics and Astronomy", "Statistical and Nonlinear Physics"),
    "physics.acc-ph": ("Physics and Astronomy", "Nuclear and High Energy Physics"),
    "physics.plasm-ph": ("Physics and Astronomy", "Nuclear and High Energy Physics"),
    "physics.ao-ph": ("Earth and Planetary Sciences", "Atmospheric Science"),
    "physics.app-ph": ("Physics and Astronomy", "Condensed Matter Physics"),
    "physics.atm-clus": ("Physics and Astronomy", "Atomic and Molecular Physics, and Optics"),
    "physics.atom-ph": ("Physics and Astronomy", "Atomic and Molecular Physics, and Optics"),
    "physics.optics": ("Physics and Astronomy", "Atomic and Molecular Physics, and Optics"),
    "physics.bio-ph": ("Biochemistry, Genetics and Molecular Biology", "Biophysics"),
    "physics.chem-ph": ("Chemistry", "Physical and Theoretical Chemistry"),
    "physics.flu-dyn": ("Engineering", "Computational Mechanics"),
    "physics.geo-ph": ("Earth and Planetary Sciences", "Geophysics"),
    "physics.hist-ph": ("Arts and Humanities", "History and Philosophy of Science"),
    "physics.ins-det": ("Physics and Astronomy", "Instrumentation"),
    "physics.data-an": ("Physics and Astronomy", "Instrumentation"),
    "physics.med-ph": ("Medicine", "Radiology, Nuclear Medicine and Imaging"),
    "physics.space-ph": ("Earth and Planetary Sciences", "Space and Planetary Science"),
    "physics.soc-ph": ("Physics and Astronomy", "Statistical and Nonlinear Physics"),
    "physics.ed-ph": ("Social Sciences", "Education"),
    "physics.class-ph": "natural-sciences", "physics.comp-ph": "natural-sciences",
    "physics.gen-ph": "natural-sciences", "physics.pop-ph": "natural-sciences",
    "q-bio.BM": ("Biochemistry, Genetics and Molecular Biology", "Biochemistry"),
    "q-bio.CB": ("Biochemistry, Genetics and Molecular Biology", "Cell Biology"),
    "q-bio.SC": ("Biochemistry, Genetics and Molecular Biology", "Cell Biology"),
    "q-bio.GN": ("Biochemistry, Genetics and Molecular Biology", "Genetics"),
    "q-bio.MN": ("Biochemistry, Genetics and Molecular Biology", "Molecular Biology"),
    "q-bio.NC": ("Neuroscience", "Cognitive Neuroscience"),
    "q-bio.PE": ("Agricultural and Biological Sciences", "Ecology, Evolution, Behavior and Systematics"),
    "q-bio.TO": ("Biochemistry, Genetics and Molecular Biology", "Physiology"),
    "q-bio.OT": "natural-sciences", "q-bio.QM": "natural-sciences",
    "q-fin.EC": ("Economics, Econometrics and Finance", "Economics and Econometrics"),
    "q-fin.*": ("Economics, Econometrics and Finance", "Finance"),
    "quant-ph": ("Physics and Astronomy", "Atomic and Molecular Physics, and Optics"),
    "stat.ML": ("Computer Science", "Artificial Intelligence"),
    "stat.*": ("Mathematics", "Statistics and Probability"),
}
ARXIV_SECONDARY = {"cs.CL": ["humanities", "psych-cogsci"], "stat.ML": ["mathematics"], "cs.RO": ["cs-ai"],
                   "cs.LG": ["mathematics"], "physics.soc-ph": ["social-other"], "math.HO": ["humanities"],
                   "cs.CY": ["social-other"], "cs.HC": ["psych-cogsci"]}

# Library of Congress Classification: (class letters, low, high, target, secondary). Numbers are class numbers
# within the subclass; None means the whole subclass. The most specific match wins.
LCC = [
    ("A", None, None, "social-other", []),
    ("B", None, None, ("Arts and Humanities", "Philosophy"), []),
    ("BC", None, None, ("Arts and Humanities", "Philosophy"), ["mathematics"]),
    ("BD", None, None, ("Arts and Humanities", "Philosophy"), []),
    ("BF", None, None, ("Psychology", "General Psychology"), []),
    ("BF", 1001, 2055, ("Arts and Humanities", "Religious studies"), []),  # parapsychology and occult sciences
    ("BH", None, None, ("Arts and Humanities", "Philosophy"), []),
    ("BJ", None, None, ("Arts and Humanities", "Philosophy"), []),
    ("BL", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BM", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BP", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BQ", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BR", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BS", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BT", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BV", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("BX", None, None, ("Arts and Humanities", "Religious studies"), []),
    ("C", None, None, ("Arts and Humanities", "History"), []),
    ("D", None, None, ("Arts and Humanities", "History"), []),
    ("E", None, None, ("Arts and Humanities", "History"), []),
    ("F", None, None, ("Arts and Humanities", "History"), []),
    ("G", None, None, ("Social Sciences", "Geography, Planning and Development"), []),
    ("GN", None, None, ("Social Sciences", "Anthropology"), []),
    ("GV", None, None, "social-other", []),
    ("H", None, None, "social-other", []),
    ("HA", None, None, ("Decision Sciences", "Statistics, Probability and Uncertainty"), []),
    ("HB", None, None, ("Economics, Econometrics and Finance", "Economics and Econometrics"), []),
    ("HM", None, None, ("Social Sciences", "Sociology and Political Science"), []),
    ("J", None, None, ("Social Sciences", "Political Science and International Relations"), []),
    ("K", None, None, ("Social Sciences", "Law"), []),
    ("L", None, None, ("Social Sciences", "Education"), []),
    ("LB", 1051, 1091, ("Psychology", "Developmental and Educational Psychology"), []),  # educational psychology
    ("M", None, None, ("Arts and Humanities", "Music"), []),
    ("ML", 3800, 3923, ("Arts and Humanities", "Music"), ["psych-cogsci"]),  # psychology and physics of music
    ("N", None, None, ("Arts and Humanities", "Visual Arts and Performing Arts"), []),
    ("P", None, None, ("Arts and Humanities", "Language and Linguistics"), []),
    ("P", 37, 37.9, ("Psychology", "Experimental and Cognitive Psychology"), ["humanities"]),  # psycholinguistics
    ("PA", None, None, ("Arts and Humanities", "Classics"), []),
    ("PE", None, None, ("Arts and Humanities", "Language and Linguistics"), []),
    ("PN", None, None, ("Arts and Humanities", "Literature and Literary Theory"), []),
    ("PR", None, None, ("Arts and Humanities", "Literature and Literary Theory"), []),
    ("PS", None, None, ("Arts and Humanities", "Literature and Literary Theory"), []),
    ("PQ", None, None, ("Arts and Humanities", "Literature and Literary Theory"), []),
    ("PT", None, None, ("Arts and Humanities", "Literature and Literary Theory"), []),
    ("PL", None, None, ("Arts and Humanities", "Language and Linguistics"), []),
    ("Q", None, None, "natural-sciences", []),
    ("Q", 334, 390, ("Computer Science", "Artificial Intelligence"), ["psych-cogsci"]),  # AI, cybernetics
    ("QA", None, None, "mathematics", []),
    ("QA", 75, 76.95, "cs-ai", []),
    ("QA", 76.87, 76.87, ("Computer Science", "Artificial Intelligence"), []),  # neural networks
    ("QA", 150, 272, ("Mathematics", "Algebra and Number Theory"), []),
    ("QA", 273, 280, ("Mathematics", "Statistics and Probability"), []),
    ("QA", 297, 299.4, ("Mathematics", "Numerical Analysis"), []),
    ("QA", 300, 433, ("Mathematics", "Applied Mathematics"), []),
    ("QA", 440, 699, ("Mathematics", "Geometry and Topology"), []),
    ("QA", 801, 939, ("Mathematics", "Mathematical Physics"), ["natural-sciences"]),
    ("QB", None, None, ("Physics and Astronomy", "Astronomy and Astrophysics"), []),
    ("QC", None, None, "natural-sciences", []),
    ("QD", None, None, "natural-sciences", []),
    ("QE", None, None, ("Earth and Planetary Sciences", "Geology"), []),
    ("QH", None, None, "natural-sciences", []),
    ("QH", 426, 470, ("Biochemistry, Genetics and Molecular Biology", "Genetics"), []),
    ("QH", 573, 671, ("Biochemistry, Genetics and Molecular Biology", "Cell Biology"), []),
    ("QK", None, None, ("Agricultural and Biological Sciences", "Plant Science"), []),
    ("QL", None, None, ("Agricultural and Biological Sciences", "Animal Science and Zoology"), []),
    ("QL", 750, 795, ("Agricultural and Biological Sciences", "Ecology, Evolution, Behavior and Systematics"),
     ["psych-cogsci"]),  # animal behaviour
    ("QM", None, None, ("Medicine", "Anatomy"), []),
    ("QP", None, None, ("Biochemistry, Genetics and Molecular Biology", "Physiology"), []),
    ("QP", 351, 495, ("Neuroscience", "Cognitive Neuroscience"), []),  # neurophysiology, neuropsychology
    ("QR", None, None, ("Immunology and Microbiology", "Microbiology"), []),
    ("R", None, None, "medicine", []),
    ("RC", 321, 431, ("Medicine", "Neurology"), ["psych-cogsci"]),
    ("RC", 435, 571, ("Medicine", "Psychiatry and Mental health"), ["psych-cogsci"]),
    ("RM", None, None, ("Medicine", "Pharmacology"), []),
    ("RT", None, None, "medicine", []),
    ("S", None, None, ("Agricultural and Biological Sciences", "Agronomy and Crop Science"), []),
    ("T", None, None, "engineering", []),
    ("T", 55.4, 60.8, ("Engineering", "Industrial and Manufacturing Engineering"), []),
    ("T", 58.5, 58.64, "cs-ai", []),  # information technology
    ("TA", None, None, ("Engineering", "Civil and Structural Engineering"), []),
    ("TJ", None, None, ("Engineering", "Mechanical Engineering"), []),
    ("TK", None, None, ("Engineering", "Electrical and Electronic Engineering"), []),
    ("TK", 7885, 7895, ("Computer Science", "Hardware and Architecture"), []),
    ("TL", None, None, ("Engineering", "Aerospace Engineering"), []),
    ("TP", None, None, "engineering", []),
    ("TS", None, None, ("Engineering", "Industrial and Manufacturing Engineering"), []),
    ("TX", None, None, "social-other", []),
    ("U", None, None, "social-other", []),
    ("V", None, None, "social-other", []),
    ("Z", None, None, ("Social Sciences", "Library and Information Sciences"), []),
]

# Dewey Decimal Classification: (low, high, target, secondary) over the decimal class number.
DDC = [
    (0, 99.999, "social-other", []), (4, 6.999, "cs-ai", []),
    (100, 149.999, ("Arts and Humanities", "Philosophy"), []), (128, 128.999, ("Arts and Humanities", "Philosophy"),
                                                                 ["psych-cogsci"]),  # philosophy of mind
    (150, 159.999, ("Psychology", "General Psychology"), []),
    (160, 199.999, ("Arts and Humanities", "Philosophy"), []), (200, 299.999, ("Arts and Humanities", "Religious studies"), []),
    (300, 399.999, "social-other", []), (330, 339.999, ("Economics, Econometrics and Finance", "Economics and Econometrics"), []),
    (370, 379.999, ("Social Sciences", "Education"), []),
    (370.15, 370.159, ("Psychology", "Developmental and Educational Psychology"), []),
    (400, 499.999, ("Arts and Humanities", "Language and Linguistics"), []),
    (401.9, 401.9999, ("Psychology", "Experimental and Cognitive Psychology"), ["humanities"]),
    (500, 509.999, "natural-sciences", []), (510, 519.999, "mathematics", []),
    (519.2, 519.599, ("Mathematics", "Statistics and Probability"), []),
    (520, 529.999, ("Physics and Astronomy", "Astronomy and Astrophysics"), []), (530, 599.999, "natural-sciences", []),
    (591.5, 591.599, ("Agricultural and Biological Sciences", "Ecology, Evolution, Behavior and Systematics"),
     ["psych-cogsci"]),
    (600, 609.999, "engineering", []), (610, 619.999, "medicine", []),
    (612.8, 612.899, ("Neuroscience", "Cognitive Neuroscience"), ["medicine"]),
    (616.8, 616.899, ("Medicine", "Psychiatry and Mental health"), ["psych-cogsci"]),
    (620, 629.999, "engineering", []), (630, 639.999, "natural-sciences", []), (640, 659.999, "social-other", []),
    (660, 699.999, "engineering", []), (700, 789.999, "humanities", []), (790, 799.999, "social-other", []),
    (800, 899.999, ("Arts and Humanities", "Literature and Literary Theory"), []),
    (900, 999.999, ("Arts and Humanities", "History"), []), (910, 919.999, "social-other", []),
]

# Category hints from manifests and inbox folder names (oa-forge domains seen so far: math, inst, textbook,
# cognition). Words that name a document type rather than a field give no hint.
NON_FIELD_HINTS = ["textbook", "inst", "paper", "papers", "book", "books", "misc", "inbox", "loose"]


# ---------------------------------------------------------------------------------------------------------------
def load(path):
    return json.loads(path.read_text(encoding="utf-8"))["records"]


def compile_terms(terms):
    return [re.compile(t, re.I) for t in terms]


def topic_text(t):
    return " ".join([t["display_name"], " ".join(t.get("keywords") or [])])


def main():
    root = Path(sys.argv[1])
    subfields = load(root / "openalex" / "subfields.json")
    topics = load(root / "openalex" / "topics.json")
    arxiv = load(root / "arxiv" / "categories.json")

    sf_key = {(s["field"]["display_name"], s["display_name"]): s for s in subfields}
    sf_by_id = {s["id"]: s for s in subfields}

    def resolve(target):
        """(field, subfield) -> subfield record; slug -> None. Fails loudly on an unknown name."""
        if isinstance(target, str):
            assert target in SLUGS, f"unknown category {target}"
            return None
        assert tuple(target) in sf_key, f"unknown OpenAlex subfield {target}"
        return sf_key[tuple(target)]

    def subfield_categories(field, sub):
        primary = SUBFIELD_PRIMARY.get((field, sub)) or FIELD_PRIMARY[field]
        secondary = [c for c in SUBFIELD_SECONDARY.get((field, sub), []) if c != primary]
        return primary, secondary

    for key in list(SUBFIELD_PRIMARY) + list(SUBFIELD_SECONDARY):
        assert key in sf_key, f"unknown subfield key {key}"
    for s in subfields:
        assert s["field"]["display_name"] in FIELD_PRIMARY, f"field without a category: {s['field']}"

    cog = compile_terms(COGSCI_TERMS)
    cog_ex = compile_terms(COGSCI_EXCLUDE)
    cog_extra = {k: compile_terms(v) for k, v in COGSCI_EXTRA.items()}
    cog_extra_ex = compile_terms(COGSCI_EXTRA_EXCLUDE)
    areas = [(slug, name, compile_terms(terms)) for slug, name, terms in PSYCH_AREAS]

    topic_map, area_examples = {}, defaultdict(list)
    for t in topics:
        field, sub = t["field"]["display_name"], t["subfield"]["display_name"]
        primary, secondary = subfield_categories(field, sub)
        text = topic_text(t)
        name = t["display_name"]
        overlay = False
        # The overlay matches topic names only: OpenAlex keyword lists are too broad (they pulled in currency
        # recognition and quantum computing when matched).
        if primary != "psych-cogsci" and sub in COGSCI_SUBFIELDS and not any(r.search(name) for r in cog_ex):
            hit = any(r.search(name) for r in cog) or (
                any(r.search(name) for r in cog_extra.get(sub, []))
                and not any(r.search(name) for r in cog_extra_ex))
            if hit:
                secondary = secondary + ["psych-cogsci"]
                overlay = True
        entry = {"name": t["display_name"], "field": field, "subfield": sub, "primary": primary,
                 "secondary": sorted(set(secondary)), "works_count": t.get("works_count", 0)}
        if overlay:
            entry["cogsci_overlay"] = True
        if primary == "psych-cogsci" or "psych-cogsci" in entry["secondary"]:
            # The topic name counts three times as much as its (broad) keyword list.
            scored = [(3 * sum(1 for r in rx if r.search(name)) + sum(1 for r in rx if r.search(text)), i, slug)
                      for i, (slug, _, rx) in enumerate(areas)]
            hits = [(score, i, slug) for score, i, slug in scored if score > 0]
            hits.sort(key=lambda x: (-x[0], x[1]))
            entry["areas"] = [slug for _, _, slug in hits] or ["general"]
            area_examples[entry["areas"][0]].append((t.get("works_count", 0), t["display_name"]))
        topic_map[t["id"].rsplit("/", 1)[-1]] = entry

    def target_json(target, secondary):
        sf = resolve(target)
        if sf is None:
            return {"category": target, "secondary": secondary}
        primary, sec = subfield_categories(sf["field"]["display_name"], sf["display_name"])
        return {"category": primary, "secondary": sorted(set(sec + secondary) - {primary}),
                "openalex_subfield": sf["id"].rsplit("/", 1)[-1], "subfield": sf["display_name"]}

    arxiv_map, unmapped = {}, []
    for c in arxiv:
        code = c["code"]
        target = ARXIV.get(code)
        if target is None:
            prefix = code.split(".")[0] if "." in code else re.sub(r"-\w+$", "", code)
            target = ARXIV.get(code.split(".")[0] + ".*") or ARXIV.get(prefix + "-*") or ARXIV.get(prefix + ".*")
        if target is None:
            if c["active"]:
                unmapped.append(code)
            continue
        arxiv_map[code] = dict(target_json(target, ARXIV_SECONDARY.get(code, [])), name=c["name"],
                               active=c["active"])
    assert not unmapped, f"active arXiv categories without a mapping: {unmapped}"

    lcc = [dict(target_json(t, sec), cls=cls, low=lo, high=hi) for cls, lo, hi, t, sec in LCC]
    ddc = [dict(target_json(t, sec), low=lo, high=hi) for lo, hi, t, sec in DDC]

    # Per-category rollups for the human-readable map.
    sub_rows = defaultdict(list)
    for s in subfields:
        field, sub = s["field"]["display_name"], s["display_name"]
        primary, secondary = subfield_categories(field, sub)
        n = sum(1 for e in topic_map.values() if e["field"] == field and e["subfield"] == sub)
        sub_rows[primary].append(("primary", field, sub, n))
        for c in secondary:
            sub_rows[c].append(("secondary", field, sub, n))
    overlay_by_sub = Counter(e["subfield"] for e in topic_map.values() if e.get("cogsci_overlay"))
    cat_topics = Counter(e["primary"] for e in topic_map.values())
    cat_members = Counter(c for e in topic_map.values() for c in {e["primary"], *e["secondary"]})

    out = {
        "format": 1, "version": VERSION,
        "sources": {"openalex": json.loads((root / "openalex" / "topics.json").read_text(encoding="utf-8"))["fetched"],
                    "arxiv": "github.com/arXiv/arxiv-base arxiv/taxonomy/definitions.py"},
        "categories": [dict(slug=c["slug"], name=c["name"], priority=c["priority"], scope=c["scope"],
                            targets={"pilot": {"papers": c["pilot"][0], "books": c["pilot"][1]},
                                     "full": {"papers": c["full"][0], "books": c["full"][1]}},
                            hints=c["synonyms"]) for c in CATEGORIES],
        "non_field_hints": NON_FIELD_HINTS,
        "psych_areas": [{"slug": s, "name": n} for s, n, _ in PSYCH_AREAS] + [{"slug": "general", "name": "General"}],
        "topics": topic_map,
        "crosswalks": {"arxiv": arxiv_map, "lcc": lcc, "ddc": ddc},
        "precedence": ["manifest_field", "openalex_primary_topic", "arxiv_primary_category", "lcc", "ddc",
                       "inbox_folder_hint", "embedding_nearest_topic"],
    }
    (root / "categories.json").write_text(json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8",
                                          newline="\n")
    write_markdown(root, out, sub_rows, cat_topics, cat_members, overlay_by_sub, area_examples, arxiv_map)
    print(f"topics {len(topic_map)}; primary per category {dict(cat_topics)}")
    print(f"members per category {dict(cat_members)}")
    print(f"cogsci overlay topics {sum(overlay_by_sub.values())}: {dict(overlay_by_sub.most_common())}")
    print("psych areas (primary):", {k: len(v) for k, v in sorted(area_examples.items())})


def write_markdown(root, out, sub_rows, cat_topics, cat_members, overlay_by_sub, area_examples, arxiv_map):
    L = []
    L.append(f"# Vault category map (version {out['version']})\n")
    L.append("Generated by `tools/taxonomy/build_categories.py` from the OpenAlex topic taxonomy "
             f"(snapshot {out['sources']['openalex']}, CC0) and our arXiv, LCC and DDC crosswalks. "
             "Do not edit by hand; change the generator and rebuild.\n")
    L.append("## How categories work\n")
    L.append("- **Categories are metadata, never paths.** Text lives at `text/{h2}/{doc_id}.md` whatever its "
             "category, so reclassifying a document never changes an address.")
    L.append("- **Backbone:** OpenAlex's four-level taxonomy (domain, field, subfield, topic). OpenAlex assigns a "
             "primary topic to every paper it knows, so most papers are classified by the metadata lookup "
             "itself. The 4,516 topics are each mapped to one primary vault category and any number of "
             "secondary ones.")
    L.append("- **Every document has exactly one primary category and any number of secondary ones.** Filters "
             "and grep scopes match primary or secondary membership; coverage counts use the primary.")
    L.append("- **Cognitive-science overlay:** cognition-related topics outside psychology (philosophy of mind, "
             "psycholinguistics, cognitive AI, behavioural economics, psychiatry, animal cognition) are also "
             "members of psychology and cognitive science.")
    L.append("- **Study map:** psychology and cognitive-science topics also carry study areas (perception, memory, "
             "clinical, methods, and so on), so the tutor can scope a search the way a graduate curriculum is "
             "organised.")
    L.append("- **Books and preprints** reach the same map through crosswalks: LCC and DDC class numbers (from "
             "the book's own Cataloging-in-Publication block, or from metadata), and arXiv categories.")
    L.append("- **Precedence when sources disagree:** " + ", then ".join(out["precedence"]) + ". Every assignment "
             "records its source and confidence; low-confidence ones are flagged in the intake report.")
    L.append("- **Unknowns are never dropped:** a document with no usable signal is classified by its nearest "
             "OpenAlex topic in embedding space (topic name, description and keywords), flagged as such.\n")

    L.append("## Summary\n")
    L.append("| Priority | Category | Topics (primary) | Topics (all members) | Pilot papers / books | Full-scale papers / books |")
    L.append("|---|---|---|---|---|---|")
    for c in out["categories"]:
        t = c["targets"]
        L.append(f"| {c['priority']} | **{c['name']}** (`{c['slug']}`) | {cat_topics[c['slug']]} | "
                 f"{cat_members[c['slug']]} | {t['pilot']['papers']:,} / {t['pilot']['books']:,} | "
                 f"{t['full']['papers']:,} / {t['full']['books']:,} |")
    L.append("\nTargets are proposals sized to the owner's priorities and the 150,000-paper, 10,000-book goal; "
             "adjust them freely. They steer gathering and coverage reports, never what intake accepts.\n")

    for c in out["categories"]:
        slug = c["slug"]
        L.append(f"## {c['priority']}. {c['name']} (`{slug}`)\n")
        L.append(c["scope"] + "\n")
        rows = sorted(sub_rows[slug], key=lambda r: (r[0] != "primary", r[1], r[2]))
        L.append("| Role | OpenAlex field | Subfield | Topics |")
        L.append("|---|---|---|---|")
        for role, field, sub, n in rows:
            L.append(f"| {role} | {field} | {sub} | {n} |")
        if slug == "psych-cogsci":
            L.append(f"\n**Cognitive-science overlay** adds {sum(overlay_by_sub.values())} topics from other fields "
                     "as secondary members: " + "; ".join(f"{k} ({v})" for k, v in overlay_by_sub.most_common())
                     + ".\n")
            L.append("**Study map** (primary area per topic; the largest topics by OpenAlex works count are shown "
                     "as examples):\n")
            names = {a["slug"]: a["name"] for a in out["psych_areas"]}
            L.append("| Area | Topics | Examples |")
            L.append("|---|---|---|")
            for a in out["psych_areas"]:
                ex = sorted(area_examples.get(a["slug"], []), reverse=True)
                L.append(f"| {names[a['slug']]} (`{a['slug']}`) | {len(ex)} | "
                         + "; ".join(n for _, n in ex[:4]) + " |")
        codes = sorted(k for k, v in arxiv_map.items() if v["category"] == slug and v["active"])
        if codes:
            L.append(f"\n**arXiv categories:** {', '.join(codes)}.")
        lcc = sorted({x["cls"] + (f"{x['low']}-{x['high']}" if x["low"] is not None else "")
                      for x in out["crosswalks"]["lcc"] if x["category"] == slug})
        if lcc:
            L.append(f"**LCC classes:** {', '.join(lcc)}.")
        ddc = sorted({f"{x['low']:g}-{x['high']:g}" for x in out["crosswalks"]["ddc"] if x["category"] == slug},
                     key=lambda s: float(s.split("-")[0]))
        if ddc:
            L.append(f"**DDC ranges:** {', '.join(ddc)}.")
        L.append(f"**Folder and manifest hints:** {', '.join(c['hints'])}.\n")
    (root / "CATEGORIES.md").write_text("\n".join(L) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
