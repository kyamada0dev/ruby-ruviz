require "minitest/autorun"
require "tmpdir"
require "ruviz"

# Phase-2 additions: line styles, error bars, polar line, heatmap colorbar/cmap,
# and fast mode.
class FeaturesTest < Minitest::Test
  def png?(b) = b == "\x89PNG".b

  def save(plot)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      plot.save(path)
      return png?(File.binread(path, 4))
    end
  end

  def test_line_style_chains_and_renders
    x = (0..20).map { |i| i / 5.0 }
    plot = Ruviz.plot.size_px(320, 240)
    assert_same plot, plot.line(x, x.map { |v| Math.sin(v) }, style: :dashed, color: "red")
    assert save(plot)
  end

  def test_line_rejects_unknown_style
    assert_raises(ArgumentError) { Ruviz.plot.line([0, 1], [0, 1], style: :zigzag) }
  end

  def test_error_bars
    x = [1.0, 2.0, 3.0, 4.0]
    y = [2.0, 4.0, 6.0, 8.0]
    err = [0.3, 0.4, 0.5, 0.6]
    plot = Ruviz.plot.size_px(320, 240)
    assert_same plot, plot.error_bars(x, y, y_err: err, x_err: [0.1, 0.1, 0.1, 0.1], color: "blue")
    assert save(plot)
  end

  def test_polar_line
    theta = (0..90).map { |i| i * Math::PI / 45.0 }
    r = theta.map { |t| 1 + 0.5 * Math.cos(3 * t) }
    plot = Ruviz.plot.size_px(320, 320)
    assert_same plot, plot.polar_line(theta, r, color: "purple", width: 2.0)
    assert save(plot)
  end

  def test_heatmap_colorbar_and_colormap
    grid = (0...6).map { |r| (0...6).map { |c| Math.sin(r) * Math.cos(c) } }
    plot = Ruviz.plot.size_px(320, 280)
    assert_same plot, plot.heatmap(grid, colormap: "viridis", colorbar: true, colorbar_label: "v")
    assert save(plot)
  end

  def test_fast_mode
    x = (0...5000).map { |i| i * 0.01 }
    plot = Ruviz.plot.size_px(320, 240).fast(true)
    assert_same plot, plot.fast
    plot.line(x, x.map { |v| Math.sin(v) })
    assert save(plot)
  end
end
