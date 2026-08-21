$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

x = (0..120).map { |i| i / 12.0 }
y = x.map { |v| Math.sin(v) }

Ruviz.plot
     .size_px(560, 340)
     .title("Annotations").xlabel("t").ylabel("y")
     .line(x, y, color: "#2563eb")
     .rect(3.0, -0.4, 3.0, 0.8, color: "#fde68a", line_width: 1.5)
     .annotate_text(4.5, 0.65, "region", color: "#b45309", size: 15.0)
     .hline(0.0, style: :dashed)
     .grid(true)
     .save("/tmp/rv_annot.png")

puts "ok"
