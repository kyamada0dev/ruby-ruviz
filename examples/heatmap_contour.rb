$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "numo/narray"

# heatmap from a 2-D Numo array (built row-major, then cast to Numo::DFloat)
rows = (0..23).map { |i| (0..23).map { |j| Math.sin(i * 0.3) * Math.cos(j * 0.3) } }
m = Numo::DFloat.cast(rows)
Ruviz.plot.size_px(520, 420).title("Heatmap")
     .heatmap(m).save("/tmp/rv_heatmap.png")

# contour of z = sin(x) * cos(y)
xs = (0..40).map { |i| i / 4.0 }
ys = (0..30).map { |j| j / 4.0 }
z  = ys.flat_map { |y| xs.map { |x| Math.sin(x) * Math.cos(y) } }
Ruviz.plot.size_px(520, 420).title("Contour").xlabel("x").ylabel("y")
     .contour(xs, ys, z, levels: 12, filled: true).save("/tmp/rv_contour.png")

puts "ok"
