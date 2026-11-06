# Refine run: worldgen (p6)

Short log of what this run changed, and what is still open.

## Changed
* **less-loose-upper** (`refine/less-loose-upper`): settled ground round the burrow (`gen::SETTLED`, `settled_at`,
  `gen::LENS`). Within 40 m of the burrow, easing off by 70 m, sand/gravel lenses need up to +0.14 more lens
  noise, none is soaked, and sand within 1 m of a natural cave is cemented to sandstone. Upper layers (effective
  depth < 10 m) get +0.012 everywhere. Deeper (10-35 m) and further-out lenses are unchanged: the hazard pockets stay.
  Tests: `sim/tests/settled_ground.rs`. Tool: `--example loosesurvey`.

### Numbers (`--example loosesurvey -- 16`, seeds 0-15, game window 1024×512 tiles, within 30 m of the burrow)
| | before | after |
|---|---|---|
| loose share of ground (sand + gravel + fungus + knocked loose) | 7.50 % | 0.68 % |
| sand | 6.65 % | 0.30 % |
| gravel | 0.62 % | 0.13 % |
| fungus (lamp oil, kept) | 0.23 % | 0.25 % |
| soaked ground (`F_WET`; what's left is the near grotto's moss and roots) | 7.66 % | 0.20 % |
| dig bot: generator-loose ground displaced per m of tunnel | 9.96 | 0.99 |
| dig bot: loose fill left in the tunnels per m | 20.7 | 20.1 |
| dig bot: cells crumbled (stress cave-ins) per m | 126 | 143 |

The bot's tunnel refill and cave-ins are dominated by roof rubble (sandstone and limestone roofs, coal and ore
seams; `WHY=1` breaks it down), not by the loose ground the generator places. That half is p2's
(collapse/slide thresholds); the crumbled numbers are passed on as their baseline, not changed here.

## Open / for others
* **p2:** the parallel CA (rayon) isn't deterministic on the first run after a rebuild. Seen on
  `digging_straight_down_never_kills` seed 6 (a limestone clump falls into a flooded shaft onto the mole on some runs),
  and on `tree_regrow` / `water_levels` on the biomes branch. Repeat runs of the same binary pass.
* Biomes v1 (PR #10) is parked behind this; it will be rebased onto it.
