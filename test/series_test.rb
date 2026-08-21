require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class SeriesTest < Minitest::Test
  def save_png(&blk)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      plot = Ruviz.plot.size_px(400, 300)
      blk.call(plot)
      plot.save(path)
      File.binread(path, 4)
    end
  end

  def png?(bytes) = bytes == "\x89PNG".b

  def test_scatter
    x = (0..20).map { |i| i / 2.0 }
    y = x.map { |v| Math.sin(v) }
    assert png?(save_png { |p| p.scatter(x, y, label: "s", color: "red", marker: :diamond, marker_size: 6.0) })
  end

  def test_scatter_chains_and_numo
    x = Numo::DFloat[0, 1, 2, 3]
    y = Numo::DFloat[0, 1, 4, 9]
    plot = Ruviz.plot
    assert_same plot, plot.scatter(x, y, marker: :circle)
  end

  def test_bar
    assert png?(save_png { |p| p.bar(%w[a b c d], [3, 1, 4, 1], color: "#4682b4") })
  end

  def test_bar_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.bar(%w[a b], [1, 2, 3]) }
  end

  def test_histogram
    data = Array.new(200) { |i| Math.sin(i * 0.3) }
    assert png?(save_png { |p| p.histogram(data, bins: 20, color: "green") })
  end

  def test_histogram_auto_bins
    assert png?(save_png { |p| p.histogram(Numo::DFloat.new(100).rand_norm) })
  end

  def test_histogram_zero_bins_raises
    assert_raises(ArgumentError) { Ruviz.plot.histogram([1, 2, 3], bins: 0) }
  end

  def test_invalid_marker_raises
    assert_raises(ArgumentError) { Ruviz.plot.scatter([1, 2], [1, 2], marker: :nope) }
  end

  def test_mixed_series
    # line + scatter + bar in one figure
    assert png?(save_png do |p|
      p.line([0, 1, 2, 3], [0, 1, 2, 3], label: "line")
      p.scatter([0, 1, 2, 3], [3, 2, 1, 0], label: "pts", marker: :square)
    end)
  end
end
