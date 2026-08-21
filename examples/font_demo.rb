$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

x = (0..80).map { |i| i / 8.0 }
y = x.map { |v| Math.sin(v) * Math.exp(-0.1 * v) }

Ruviz.plot
     .size_px(640, 400)
     .font_family("JetBrainsMono Nerd Font Mono")
     .font_size(15.0)
     .title("JetBrainsMono Nerd Font 0123 => <= != ->")
     .xlabel("time [s]").ylabel("amplitude")
     .line(x, y, label: "damped sin", color: "#2563eb", width: 2.0)
     .grid(true).legend(:upper_right)
     .save("/tmp/rv_font.png")

# default font for comparison
Ruviz.plot.size_px(640, 400)
     .title("Default font 0123 => <= != ->")
     .xlabel("time [s]").ylabel("amplitude")
     .line(x, y, color: "#dc2626", width: 2.0)
     .grid(true).save("/tmp/rv_font_default.png")

puts "ok"
