$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"
require "numo/narray"

s = Numo::DFloat.new(3000).rand_norm

Ruviz.plot.size_px(560, 340).title("KDE").xlabel("x").ylabel("density")
     .kde(s, color: "#2563eb", label: "normal").grid(true).save("/tmp/rv_kde.png")

Ruviz.plot.size_px(560, 340).title("ECDF").xlabel("x").ylabel("F(x)")
     .ecdf(s, color: "#059669").grid(true).save("/tmp/rv_ecdf.png")

Ruviz.plot.size_px(560, 340).title("Violin")
     .violin(s, color: "#7c3aed", alpha: 0.6, label: "normal").grid(true).save("/tmp/rv_violin.png")

puts "ok"
