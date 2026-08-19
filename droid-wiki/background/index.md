# Background

This section gathers the rationale behind how Capto is built and the debt that is deliberately carried along the way. It is the place to understand why a choice was made, in case a future change wants to revisit it.

- [Design decisions](design-decisions.md) documents the deliberate architectural choices: why Capto stays local-only, why there is one machine-wide session owned by the desktop, why encoding goes through a pinned bundled FFmpeg, and how the rest of the pipeline is shaped around those commitments. Each decision lists the evidence in the code or docs so it can be checked against the current tree.
- [Cleanup opportunities](../cleanup-opportunities.md) tracks the concrete hotspots that are candidates for future work: which files carry the most complexity, where dead paths are prevented, and how dependency freshness is handled. It also records what the CI gates already block at merge time.

For how these decisions connect to the recording pipeline shape, see [Architecture](../overview/architecture.md). For the observable size and complexity facts behind the cleanup list, see [By the numbers](../by-the-numbers.md).
