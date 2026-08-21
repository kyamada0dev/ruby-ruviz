$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

x = (0..100).map { |i| i / 10.0 }
y = x.map { |v| Math.exp(-0.3 * v) }
y2 = x.map { |v| Math.exp(-0.6 * v) }

out = File.join(__dir__, "smoke.png")

Ruviz.plot
     .size_px(760, 420)
     .title("Decay Rates")
     .xlabel("time")
     .ylabel("intensity")
     .line(x, y, label: "fast decay", color: "#2563eb", width: 2.0)
     .line(x, y2, label: "faster decay", color: "red")
     .grid(true)
     .legend(:upper_right)
     .save(out)

puts "wrote #{out} (#{File.size(out)} bytes)" if File.exist?(out)
