# ruby-ruviz

Ruby bindings for the Rust [`ruviz`](https://github.com/Ameyanagi/ruviz) plotting
library, built with [Magnus](https://github.com/matsadler/magnus) + `rb-sys`.

The binding is deliberately **thin**: ruviz performs all layout, scaling and
rendering; Ruby just offers an idiomatic fluent API and hands numeric data to
Rust with as little copying as possible.

```ruby
Ruviz.plot
  .size_px(760, 420)
  .title("Decay Rates")
  .xlabel("time")
  .ylabel("intensity")
  .line(x, y, label: "fast decay")
  .yscale(:log)
  .grid(true)
  .legend(:upper_right)
  .save("decay.png")
```

## Data input

Numeric data can come from (in order of preference, to avoid materializing Ruby
objects for large datasets):

1. `Numo::NArray`
2. Polars `Series` / DataFrame column
3. Ruby `Array`

## Status

Early development. See `ruviz-ruby-binding-devspec.md` for the design and the
phased implementation plan.

- **Phase 1 — extension skeleton**: `require "ruviz"`, native extension loads. ✅
- Phase 2 — `Ruviz.plot` / `Ruviz::Plot` (TypedData) + `save`.
- Phase 3+ — Array / Numo / Polars data, line plot, axes, scales, legend, PNG/SVG/PDF.

## Development

```sh
bundle install
bundle exec rake compile
bundle exec rake test
```

## License

MIT
