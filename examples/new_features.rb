$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "fileutils"

# Phase-2 additions: line styles, error bars, polar line, heatmap colorbar +
# colormap, and fast mode. Rendered crisp via .dpi.
DIR = File.expand_path("../docs/gallery", __dir__)
FileUtils.mkdir_p(DIR)
def out(n) = File.join(DIR, "#{n}.png")

x = (0..60).map { |i| i / 6.0 }

# Line styles
p = Ruviz.plot.size_px(360, 260).dpi(200).title("line styles").grid(true)
%i[solid dashed dotted dash_dot].each_with_index do |st, i|
  p = p.line(x, x.map { |v| Math.sin(v) + i * 0.6 }, label: st.to_s, width: 2.0, style: st)
end
p.legend(:upper_right).save(out("line_styles"))

# Error bars
xs = (1..6).map(&:to_f)
Ruviz.plot.size_px(360, 260).dpi(200).title("error bars").grid(true)
     .error_bars(xs, xs.map { |v| v * 1.5 }, y_err: xs.map { |v| 0.4 + 0.1 * v }, color: "#2563eb")
     .save(out("errorbar"))

# Polar line (rose curve)
theta = (0..180).map { |i| i * Math::PI / 90.0 }
Ruviz.plot.size_px(300, 300).dpi(200).title("polar")
     .polar_line(theta, theta.map { |t| 1 + 0.5 * Math.cos(3 * t) }, color: "#7c3aed", width: 2.0)
     .save(out("polar"))

# Heatmap with a named colormap and a colorbar
grid = (0...12).map { |r| (0...12).map { |c| Math.sin(r / 2.0) * Math.cos(c / 2.0) } }
Ruviz.plot.size_px(360, 300).dpi(200).title("heatmap + colorbar")
     .heatmap(grid, colormap: "viridis", colorbar: true, colorbar_label: "value")
     .save(out("heatmap_colorbar"))

# Fast mode for a large series
bx = (0...20_000).map { |i| i * 0.001 }
Ruviz.plot.size_px(360, 260).dpi(150).title("fast mode (20k pts)").fast(true)
     .line(bx, bx.map { |v| Math.sin(v) }, color: "#2563eb").save(out("fast"))

puts "wrote line_styles, errorbar, polar, heatmap_colorbar, fast to #{DIR}"
