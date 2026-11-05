# Refine run: bugs and logic

Short log of what this run changed, and what is still open.

## Changed
* **Placed pillars and beams no longer collapse and flow away** (`stress.rs`). Overburden used to add up through air gaps, so a free-standing post or a beam bridging a gap carried the whole roof above the gap, cracked and crumbled. Overburden now counts only material resting contiguously on a cell. A run only takes tributary load from a void beside it if it reaches that void's ceiling. Natural cantilevers and ledges benefit the same way.
* **Broken wood and iron become their own debris** (`mat.rs`): wood beams and doors crumble into loose wood splinters, and iron beams into loose iron scrap, instead of sand or gravel. Digging the debris gives the wood or iron back.
* Scenario harness: `place` directive and the `I` (iron beam) map char. New scenarios: `pillar_holds`, `pillar_holds_roof` + `roof_needs_pillar` (control), `beam_spans_gap`, `beam_overload`.

## Open
