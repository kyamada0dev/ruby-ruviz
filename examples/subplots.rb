$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

# A grid of plots rendered into one figure (like matplotlib's subplots or a
# pandas facet). Build each panel with Ruviz.plot, then place them with
# #subplot(row, col, plot) or #subplot_at(flat_index, plot). Subplots render
# raster (PNG) output.

x = (0..100).map { |i| i / 10.0 }

line    = Ruviz.plot.title("line").line(x, x.map { |v| Math.sin(v) }, color: "#2563eb").grid(true)
scatter = Ruviz.plot.title("scatter").scatter(x, x.map { |v| Math.cos(v) }, color: "#059669", marker_size: 4.0)
bars    = Ruviz.plot.title("bar").bar(%w[a b c d], [3, 7, 2, 5], color: "#f59e0b")
hist    = Ruviz.plot.title("hist").histogram(x.map { |v| Math.sin(v) }, bins: 15, color: "#7c3aed")

out = File.join(__dir__, "subplots.png")

Ruviz.subplots(2, 2, 820, 620)
     .suptitle("Subplot Gallery")
     .subplot(0, 0, line)
     .subplot(0, 1, scatter)
     .subplot(1, 0, bars)
     .subplot(1, 1, hist)
     .save(out)

puts "wrote #{out} (#{File.size(out)} bytes)" if File.exist?(out)
