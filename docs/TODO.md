# ruby-ruviz — unimplemented API backlog

Gaps between the underlying `ruviz` crate (v0.11) and this Ruby binding.
Priority reflects usefulness for everyday data-viz (pandas-style workflows).

## Implemented today

- Series: `line` `scatter` `bar` `histogram` `area` `boxplot` `kde` `ecdf`
  `violin` `heatmap` `contour` `pie` `radar`
- Figure/axes: `size_px` `title` `xlabel` `ylabel` `xscale` `yscale` `xlim`
  `ylim` `grid` `legend` `theme` `font_family` `font_size` `title_size`
  `legend_font_size` `scale_typography`
- Annotations: `hline` `vline` `annotate_text` `rect`
- Output: `save` (PNG/SVG/PDF by extension)
- Figures: `subplots` (`suptitle` / `subplot` / `subplot_at`, raster output)

## A. Series types — not yet bound (low cost: mirror the existing pattern)

- [ ] `step` — step line
- [ ] `stem` — stem / impulse
- [ ] `grouped_bar` — grouped bars (only plain `bar` today)
- [ ] `stacked_bar` — stacked bars
- [ ] `stacked_area` — stacked area
- [x] `error_bars` / `error_bars_xy` — error bars (symmetric x/y)
- [ ] `strip` — strip plot (categorical scatter)
- [ ] `swarm` — swarm / beeswarm plot
- [ ] `boxen` — letter-value (boxen) plot
- [ ] `hexbin` — hexbin density
- [ ] `rug` — rug marks
- [ ] `quiver` — vector field
- [x] `polar_line` — polar plot
- [ ] `dendrogram` — hierarchical clustering

## B. Composite figures (figure-level, like `subplots`)

- [ ] `jointplot` — scatter + marginal distributions
- [ ] `pairplot` — pairwise scatter matrix
- [ ] `regplot` — regression line
- [ ] `residplot` — residual plot

## C. Styling / configuration gaps (high value)

- [x] heatmap colormap + colorbar: `heatmap(colormap:, colorbar:, colorbar_label:)`.
      Value-colored `scatter` (per-point cmap) still unbound.
- [ ] per-point mapping: `color_source`, `marker_size_source` (bubble / colored
      scatter)
- [ ] `fill_between` / `fill_between_styled`, `axhspan` / `axvspan` (shaded bands)
- [ ] tick control: `xtick_rotation`, `minor_ticks`, `tick_direction_*`,
      `tick_sides`, `tight_layout`
- [ ] arrows and styled annotations: `arrow` / `arrow_styled`, `hline_styled`,
      `vline_styled`, `text_styled`, `rect_styled`
- [ ] `scientific_notation`

## D. Output options

- [x] DPI control: `dpi` (crisper text/lines; layout unchanged). `save_with_dpi`
      / `save_with_size` still unbound.
- [ ] in-memory bytes: `render_png_bytes`, `render_to_svg` (embed without a file)
- [ ] SVG/PDF for `subplots` (currently the crate renders subplot figures as
      raster only — upstream limitation)

## E. Advanced (larger scope / needs a decision)

- [x] 3D: `scatter3d`, `line3d`, `surface`, `wireframe` — the crate's `3d` Cargo
      feature is enabled and these standalone builders are bound. Camera/elev/azim
      control and 3D animation still unbound.
- [ ] interactive sessions (`InteractivePlotSession`)
- [ ] animation (`recorder` / `figure_size`)
- [ ] reactive signals, GPU backend selection

---

Suggested first batch: **A** (step, stem, grouped_bar, stacked_bar,
stacked_area, error_bars, strip, swarm, hexbin) + **C** (cmap+colorbar,
xtick_rotation, fill_between). These follow the existing series/annotation
binding pattern and cover the most common pandas/seaborn plots.
