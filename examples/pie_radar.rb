$LOAD_PATH.unshift File.expand_path("../lib", __dir__)
require "ruviz"

Ruviz.plot.size_px(480, 420).title("Pie")
     .pie([35, 25, 20, 15, 5],
          labels: %w[rent food transport savings other])
     .save("/tmp/rv_pie.png")

Ruviz.plot.size_px(480, 420).title("Radar")
     .radar(%w[speed power range agility cost efficiency],
            "Model A" => [4, 3, 5, 2, 3, 4],
            "Model B" => [2, 5, 3, 4, 4, 3])
     .legend(:upper_right)
     .save("/tmp/rv_radar.png")

puts "ok"
