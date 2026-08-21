$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

x = (0..120).map { |i| i / 12.0 }
y = x.map { |v| Math.exp(-0.2 * v) * Math.cos(v) }

Ruviz.plot
     .size_px(640, 400)
     .theme(:dark)
     .title("Damped Oscillation")
     .xlabel("time").ylabel("amplitude")
     .line(x, y, label: "signal", color: "#38bdf8", width: 2.0)
     .hline(0.0, color: "#f87171", width: 1.5, style: :dashed)
     .vline(5.0, color: "#a3e635", style: :dotted)
     .xlim(0, 10).ylim(-1, 1)
     .grid(true)
     .legend(:upper_right)
     .save("/tmp/rv_theme.png")

puts "ok"
