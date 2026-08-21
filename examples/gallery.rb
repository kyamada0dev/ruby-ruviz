# Generates the README gallery thumbnails into docs/gallery/.
#   bundle exec ruby examples/gallery.rb
$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "fileutils"

DIR = File.expand_path("../docs/gallery", __dir__)
FileUtils.mkdir_p(DIR)
W = 360
H = 260

def out(name) = File.join(DIR, "#{name}.png")

# Deterministic "sample" so the gallery is reproducible (no RNG).
def sample(n = 400)
  (0...n).map { |i| Math.sin(i * 0.7) + 0.6 * Math.sin(i * 0.13) + 0.3 * Math.cos(i * 0.37) }
end

x = (0..80).map { |i| i / 8.0 }

Ruviz.plot.size_px(W, H).title("line")
     .line(x, x.map { |v| Math.sin(v) }, color: "#2563eb", width: 2.0)
     .line(x, x.map { |v| Math.cos(v) }, color: "#dc2626").grid(true).save(out("line"))

Ruviz.plot.size_px(W, H).title("scatter")
     .scatter(x, x.map { |v| Math.sin(v) }, color: "#7c3aed", marker: :circle, marker_size: 4.0)
     .grid(true).save(out("scatter"))

Ruviz.plot.size_px(W, H).title("bar")
     .bar(%w[a b c d e], [3, 7, 2, 5, 4], color: "#4682b4").grid(true).save(out("bar"))

Ruviz.plot.size_px(W, H).title("histogram")
     .histogram(sample, bins: 24, color: "#059669").save(out("histogram"))

Ruviz.plot.size_px(W, H).title("area")
     .area(x, x.map { |v| Math.exp(-0.2 * v) + 0.3 }, baseline: 0.0, color: "#38bdf8", alpha: 0.5)
     .grid(true).save(out("area"))

Ruviz.plot.size_px(W, H).title("boxplot")
     .boxplot(sample, color: "#059669").grid(true).save(out("boxplot"))

Ruviz.plot.size_px(W, H).title("kde")
     .kde(sample, color: "#2563eb").grid(true).save(out("kde"))

Ruviz.plot.size_px(W, H).title("ecdf")
     .ecdf(sample, color: "#b45309").grid(true).save(out("ecdf"))

Ruviz.plot.size_px(W, H).title("violin")
     .violin(sample, color: "#7c3aed", alpha: 0.6).grid(true).save(out("violin"))

grid = (0..15).map { |i| (0..15).map { |j| Math.sin(i * 0.4) * Math.cos(j * 0.4) } }
Ruviz.plot.size_px(W, H).title("heatmap").heatmap(grid).save(out("heatmap"))

xs = (0..30).map { |i| i / 3.0 }
ys = (0..24).map { |j| j / 3.0 }
z  = ys.flat_map { |yy| xs.map { |xx| Math.sin(xx) * Math.cos(yy) } }
Ruviz.plot.size_px(W, H).title("contour").contour(xs, ys, z, levels: 10, filled: true).save(out("contour"))

Ruviz.plot.size_px(W, H).title("pie")
     .pie([35, 25, 20, 15, 5], labels: %w[rent food transit save other]).save(out("pie"))

Ruviz.plot.size_px(W, H).title("radar")
     .radar(%w[spd pwr rng agi cost eff],
            "A" => [4, 3, 5, 2, 3, 4], "B" => [2, 5, 3, 4, 4, 3])
     .legend(:upper_right).save(out("radar"))

Ruviz.plot.size_px(W, H).theme(:dark).title("theme + annotations")
     .line(x, x.map { |v| Math.exp(-0.2 * v) * Math.cos(v) }, color: "#38bdf8", width: 2.0)
     .hline(0.0, color: "#f87171", style: :dashed)
     .annotate_text(4.0, 0.5, "decay", color: "#a3e635", size: 13.0)
     .grid(true).save(out("theme"))

puts "wrote #{Dir[File.join(DIR, '*.png')].size} thumbnails to #{DIR}"
