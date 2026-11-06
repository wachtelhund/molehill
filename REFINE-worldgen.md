# Refine run: worldgen and biomes (p6)

Short log of what this run changed, and what is still open.

## Changed
* **biomes-v1** (`refine/biomes-v1`): regions became 2D. `Gen::biome_of(x, wy)` bends each column's leaning by
  depth (wobbling ±15 m): Loamlands and Karst up high, Granite crags, **Fungal hollows** (new) as pockets in the
  12-64 m layers, Ashlands below 50-80 m. Each region sets its strata offset, cave openness, fluid pockets
  (karst floods, ashlands burn, crags dry), host rock (karst limestone, crag gravel/sandstone, loam clay,
  ashlands basalt), cave growth (fungus banks, moss, mycelium on side walls in the fungal hollows; seep moss in
  the karst; none in the ashlands) and natural back walls (new `Wall::Mycel`; salt/damp, cracks, roots, basalt).
  Renderer: parallax strata tinted per cell region; natural ground in front gets a soft region wash.
  Tests: `sim/tests/biomes.rs`; `--example biomediag`.
* Constraints kept: start zone softening untouched, ore bands depth-only, near grotto and NEAR_FIND untouched,
  no edits to site layouts.

## Open
* Fungal hollows have no hazard of their own yet (idea: fungus respires, so a sealed hollow's air goes stale via `air.rs`).
* Region-specific fauna/flora beyond existing materials (no new materials yet); grotto chance could lean by region.
* Region name in the HUD / journal on first entry (discovery-style).
