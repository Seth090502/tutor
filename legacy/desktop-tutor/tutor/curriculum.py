"""Curriculum: what to learn and in what order. First-principles prerequisite chains."""

DOMAINS = {
    "math": {
        "label": "Math",
        "levels": ["foundations", "algebra", "geometry", "precalculus"],
        "topics": {
            "arithmetic-laws": {
                "label": "Arithmetic laws (comm/assoc/distrib)",
                "prereq": [],
                "level": "foundations",
            },
            "fractions": {
                "label": "Fractions, ratios, percents",
                "prereq": ["arithmetic-laws"],
                "level": "foundations",
            },
            "linear-equations": {
                "label": "Linear equations",
                "prereq": ["arithmetic-laws"],
                "level": "algebra",
            },
            "quadratics": {
                "label": "Quadratics (factoring, formula)",
                "prereq": ["linear-equations"],
                "level": "algebra",
            },
            "functions": {
                "label": "Functions (domain, range, inverse)",
                "prereq": ["linear-equations"],
                "level": "algebra",
            },
            "exponents-logs": {
                "label": "Exponents and logarithms",
                "prereq": ["functions"],
                "level": "precalculus",
            },
            "geometry-basics": {
                "label": "Angles, triangles, Pythagoras",
                "prereq": ["fractions"],
                "level": "geometry",
            },
        },
    },
    "english": {
        "label": "English",
        "levels": ["foundations", "craft", "analysis"],
        "topics": {
            "grammar-core": {
                "label": "Sentence core (subjects, verbs, agreement)",
                "prereq": [],
                "level": "foundations",
            },
            "punctuation": {
                "label": "Punctuation (comma, apostrophe, colon)",
                "prereq": ["grammar-core"],
                "level": "foundations",
            },
            "thesis": {
                "label": "Thesis + paragraph structure",
                "prereq": ["grammar-core"],
                "level": "craft",
            },
            "evidence": {"label": "Evidence + citation", "prereq": ["thesis"], "level": "craft"},
            "rhetoric": {
                "label": "Rhetoric (ethos/pathos/logos, fallacies)",
                "prereq": ["evidence"],
                "level": "analysis",
            },
        },
    },
    "science": {
        "label": "Science",
        "levels": ["foundations", "mechanisms", "systems"],
        "topics": {
            "scientific-method": {
                "label": "Scientific method + units",
                "prereq": [],
                "level": "foundations",
            },
            "cells": {
                "label": "Cells (organelles, diffusion)",
                "prereq": ["scientific-method"],
                "level": "mechanisms",
            },
            "photosynthesis": {
                "label": "Photosynthesis + respiration",
                "prereq": ["cells"],
                "level": "mechanisms",
            },
            "genetics": {
                "label": "Genetics (Mendel, Punnett)",
                "prereq": ["cells"],
                "level": "mechanisms",
            },
            "newton": {
                "label": "Newton's laws + energy",
                "prereq": ["scientific-method"],
                "level": "systems",
            },
        },
    },
    "chemistry": {
        "label": "Chemistry",
        "levels": ["foundations", "reactions", "quantitative"],
        "topics": {
            "atoms": {"label": "Atoms, periodic table, ions", "prereq": [], "level": "foundations"},
            "bonding": {
                "label": "Bonding (ionic/covalent/metallic)",
                "prereq": ["atoms"],
                "level": "foundations",
            },
            "reactions": {
                "label": "Reaction types + balancing",
                "prereq": ["bonding"],
                "level": "reactions",
            },
            "stoichiometry": {
                "label": "Moles + stoichiometry",
                "prereq": ["reactions"],
                "level": "quantitative",
            },
            "acids-bases": {
                "label": "Acids, bases, pH",
                "prereq": ["reactions"],
                "level": "reactions",
            },
        },
    },
}

# First-principles explainers: definition -> mechanism -> check.
EXPLAINERS = {
    "quadratics": (
        "A quadratic is ax^2+bx+c=0 (a!=0). From first principles: x^2 grows "
        "nonlinearly, so there can be 0/1/2 real roots. Complete the square:\n"
        "  ax^2+bx+c=0 -> (x+b/2a)^2 = (b^2-4ac)/4a^2.\n"
        "Hence x = [-b +- sqrt(b^2-4ac)]/2a. The discriminant D=b^2-4ac decides: "
        "D>0 two roots, D=0 one, D<0 none real.\n"
        "Socratic checks: (1) Why can't a=0? (2) If D=0, what does the graph do? "
        "(3) Give a D<0 example and explain why no real x works."
    ),
    "photosynthesis": (
        "From first principles: life needs free energy. Photosynthesis stores light "
        "energy as chemical bonds: 6CO2+6H2O+light -> C6H12O6+6O2 (chlorophyll). "
        "Respiration reverses it to release ATP. Light reactions (thylakoid) make "
        "ATP/NADPH; Calvin cycle (stroma) fixes CO2.\n"
        "Checks: (1) Where do the O2 atoms come from? (2) Why do both steps need "
        "each other? (3) What happens if CO2=0 but light is high?"
    ),
    "stoichiometry": (
        "From first principles: atoms are conserved. A mole (6.022e23) lets us count "
        "atoms by weighing grams. Balance the equation first, then convert: "
        "grams -> moles -> mole-ratio -> moles -> grams.\n"
        "Checks: (1) Why balance before converting? (2) What is the limiting reactant "
        "in 2H2+O2->2H2O if you have 3 mol H2 + 1 mol O2? (3) Where does excess mass go?"
    ),
}


def get_domains():
    return sorted(DOMAINS.keys())


def topics_for(domain):
    return DOMAINS.get(domain, {}).get("topics", {})


def next_topic(domain, mastered):
    """First unmastered topic whose prereqs are mastered (simple topological next)."""
    mastered = set(mastered or [])
    for tid, t in topics_for(domain).items():
        if tid in mastered:
            continue
        if all(p in mastered for p in t.get("prereq", [])):
            return tid
    return None
