"""Bootstrap confidence intervals over questions (fixed seed, 2,000 resamples)."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
import random

N_BOOT = 2000
BOOT_SEED = 20260930


def quantile(sorted_values: Sequence[float], q: float) -> float:
    """Linear-interpolation quantile of an already sorted sequence."""
    pos = q * (len(sorted_values) - 1)
    lo = int(pos)
    hi = min(lo + 1, len(sorted_values) - 1)
    return sorted_values[lo] + (sorted_values[hi] - sorted_values[lo]) * (pos - lo)


def bootstrap_ci(
    columns: Mapping[str, Sequence[float]],
    *,
    n_boot: int = N_BOOT,
    seed: int = BOOT_SEED,
) -> dict[str, tuple[float, float]]:
    """95% percentile CI of each column's mean, resampling question indices with replacement.

    All columns share each resample, so passing per-question score differences between two
    runs as columns gives the paired bootstrap.
    """
    names = list(columns)
    if not names:
        return {}
    n = len(columns[names[0]])
    if n == 0 or any(len(columns[name]) != n for name in names):
        raise ValueError("columns must be non-empty and the same length")
    rng = random.Random(seed)
    population = range(n)
    means: dict[str, list[float]] = {name: [] for name in names}
    for _ in range(n_boot):
        idx = rng.choices(population, k=n)
        for name in names:
            means[name].append(sum(map(columns[name].__getitem__, idx)) / n)
    return {
        name: (quantile(sorted(vals), 0.025), quantile(sorted(vals), 0.975))
        for name, vals in means.items()
    }
