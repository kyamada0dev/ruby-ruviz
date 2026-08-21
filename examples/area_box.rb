$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "numo/narray"

x = (0..80).map { |i| i / 8.0 }
y = x.map { |v| Math.exp(-0.15 * v) * (1 + 0.4 * Math.sin(v)) }
Ruviz.plot.size_px(560, 340).title("Area").xlabel("t").ylabel("value")
     .area(x, y, baseline: 0.0, color: "#38bdf8", alpha: 0.5, label: "signal")
     .grid(true).legend(:upper_right).save("/tmp/rv_area.png")

Ruviz.plot.size_px(560, 340).title("Boxplot")
     .boxplot(Numo::DFloat.new(1000).rand_norm, color: "#059669", label: "normal")
     .grid(true).save("/tmp/rv_box.png")

puts "ok"
