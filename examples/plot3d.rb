$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "fileutils"

# 3D plots: scatter3d / line3d / surface / wireframe. scatter3d and line3d take
# three 1-D vectors; surface and wireframe take 1-D x (nx) and y (ny) axes plus
# a (ny x nx) 2-D z grid. Output is raster (PNG).
DIR = File.expand_path("../docs/gallery", __dir__)
FileUtils.mkdir_p(DIR)
def out(n) = File.join(DIR, "#{n}.png")

# 3D scatter
Ruviz.scatter3d([0, 1, 2, 3, 4], [0.2, 1.4, 0.8, 2.7, 2.1], [0.5, 1.8, 1.1, 3.2, 2.6])
     .title("3D scatter").xlabel("x").ylabel("y").zlabel("z")
     .marker(:circle).marker_size(8.0).color("#2563eb")
     .save(out("scatter3d"))

# 3D line (helix)
t = (0...200).map { |i| i * 0.08 }
Ruviz.line3d(t.map { |v| Math.cos(v) }, t.map { |v| Math.sin(v) }, t.map { |v| v * 0.08 })
     .title("3D helix").xlabel("x").ylabel("y").zlabel("z").line_width(2.0).color("#dc2626")
     .save(out("line3d"))

# 3D surface and wireframe over the same grid
xs = (-3..3).map { |i| i.to_f }
ys = (-3..3).map { |i| i.to_f }
grid = ys.map { |y| xs.map { |x| Math.exp(-(x * x + y * y) / 8.0) } }

Ruviz.surface(xs, ys, grid).title("3D surface").xlabel("x").ylabel("y").zlabel("z")
     .save(out("surface3d"))

Ruviz.wireframe(xs, ys, grid).title("3D wireframe").xlabel("x").ylabel("y").zlabel("z")
     .line_width(1.2).color("#334155").save(out("wireframe3d"))

puts "wrote scatter3d, line3d, surface3d, wireframe3d to #{DIR}"
