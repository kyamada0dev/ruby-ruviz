# Phase 8 benchmark: data-ingest cost, native buffer (Numo / Polars) vs Ruby Array.
#
# Isolates the "Ruby data -> Rust Vec<f64>" step by timing a single `line(x, y)`
# call (which extracts both series and stores them, but does NOT render). Render
# is source-independent, so a smaller separate measurement shows where total time
# goes.
#
# Run: bundle exec ruby benchmark/plot.rb   (optionally BENCH_MAX=10_000_000)

$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "numo/narray"
require "polars"

def mono = Process.clock_gettime(Process::CLOCK_MONOTONIC)

# Median of `reps` timings (seconds).
def bench(reps = 5)
  times = Array.new(reps) do
    t = mono
    yield
    mono - t
  end
  times.sort[reps / 2]
end

max = Integer(ENV.fetch("BENCH_MAX", 1_000_000))
sizes = [1_000, 10_000, 100_000, 1_000_000, 10_000_000].select { |n| n <= max }

puts "ruby-ruviz Phase 8 — data ingest: native buffer vs Ruby Array intermediate"
puts "(median of 5, one `line(x,y)` call = extract x + extract y -> Vec<f64>)"
puts
puts "  Numo        : line(numo)        native buffer read (the binding's path)"
puts "  Numo->to_a  : line(numo.to_a)   the anti-pattern §22 warns against"
puts "  Polars      : line(series)      cast(f64).to_numo -> native buffer"
puts
puts format("%12s | %11s %11s %11s %11s | %s",
            "N", "Array", "Numo", "Numo->to_a", "Polars", "to_a / Numo")
puts "-" * 92

sizes.each do |n|
  xs = Array.new(n) { |i| i.to_f }
  ys = Array.new(n) { |i| Math.sin(i * 0.001) }
  nx = Numo::DFloat.cast(xs)
  ny = Numo::DFloat.cast(ys)
  px = Polars::Series.new("x", xs)
  py = Polars::Series.new("y", ys)

  # warmup
  2.times do
    Ruviz.plot.line(xs, ys)
    Ruviz.plot.line(nx, ny)
    Ruviz.plot.line(px, py)
  end

  t_arr  = bench { Ruviz.plot.line(xs, ys) }
  t_numo = bench { Ruviz.plot.line(nx, ny) }
  t_toa  = bench { Ruviz.plot.line(nx.to_a, ny.to_a) } # Numo -> Ruby Array -> Rust
  t_pol  = bench { Ruviz.plot.line(px, py) }

  ms = ->(t) { format("%9.2fms", t * 1000) }
  puts format("%12d | %11s %11s %11s %11s | %5.1fx",
              n, ms.call(t_arr), ms.call(t_numo), ms.call(t_toa), ms.call(t_pol),
              t_toa / t_numo)

  # free before the next (larger) size
  xs = ys = nx = ny = px = py = nil
  GC.start
end

# Render cost is the same regardless of source — show it once for context.
n = sizes.last
xs = Array.new(n) { |i| i.to_f }
ys = xs.map { |v| Math.sin(v * 0.001) }
require "tmpdir"
Dir.mktmpdir do |dir|
  path = File.join(dir, "b.png")
  t = bench(3) { Ruviz.plot.size_px(800, 600).line(xs, ys).save(path) }
  puts
  puts format("render (save PNG, N=%d): %.1fms  — source-independent", n, t * 1000)
end
