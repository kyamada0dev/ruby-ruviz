$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "numo/narray"

Ruviz.plot.size_px(500, 320).title("Bar").xlabel("category").ylabel("value")
     .bar(%w[alpha beta gamma delta], [3, 7, 2, 5], color: "#4682b4")
     .grid(true).save("/tmp/rv_bar.png")

xs = (0..60).map { |i| i / 6.0 }
ys = xs.map { |v| Math.sin(v) }
Ruviz.plot.size_px(500, 320).title("Scatter").xlabel("x").ylabel("sin x")
     .scatter(xs, ys, color: "#dc2626", marker: :circle, marker_size: 5.0)
     .grid(true).save("/tmp/rv_scatter.png")

Ruviz.plot.size_px(500, 320).title("Histogram")
     .histogram(Numo::DFloat.new(2000).rand_norm, bins: 30, color: "#059669")
     .save("/tmp/rv_hist.png")

puts "ok"
