require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class AreaBoxplotTest < Minitest::Test
  def save_png(&blk)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      plot = Ruviz.plot.size_px(400, 300)
      blk.call(plot)
      plot.save(path)
      File.binread(path, 4)
    end
  end

  def png?(b) = b == "\x89PNG".b

  def test_area
    x = (0..30).map { |i| i / 3.0 }
    y = x.map { |v| Math.exp(-0.2 * v) }
    assert png?(save_png { |p| p.area(x, y, baseline: 0.0, color: "#38bdf8", alpha: 0.5) })
  end

  def test_area_default_baseline_and_numo
    x = Numo::DFloat[0, 1, 2, 3]
    y = Numo::DFloat[1, 2, 1, 3]
    plot = Ruviz.plot
    assert_same plot, plot.area(x, y, label: "a")
  end

  def test_area_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.area([1, 2, 3], [1, 2]) }
  end

  def test_boxplot
    data = Array.new(200) { |i| Math.sin(i * 0.3) * 2 }
    assert png?(save_png { |p| p.boxplot(data, label: "sample", color: "#059669") })
  end

  def test_boxplot_numo
    assert png?(save_png { |p| p.boxplot(Numo::DFloat.new(300).rand_norm) })
  end

  def test_boxplot_empty_raises
    assert_raises(ArgumentError) { Ruviz.plot.boxplot([]) }
  end

  def test_area_then_line
    assert png?(save_png do |p|
      x = [0, 1, 2, 3, 4]
      p.area(x, [1, 3, 2, 4, 1], baseline: 0.0, alpha: 0.4)
      p.line(x, [2, 2, 2, 2, 2], color: "red")
    end)
  end
end
