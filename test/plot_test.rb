require "minitest/autorun"
require "tmpdir"
require "ruviz"

class PlotTest < Minitest::Test
  def setup
    @x = (0..50).map { |i| i / 5.0 }
    @y = @x.map { |v| Math.exp(-v) }
  end

  def test_plot_returns_plot
    assert_instance_of Ruviz::Plot, Ruviz.plot
  end

  def test_builder_methods_chain
    plot = Ruviz.plot
    assert_same plot, plot.size_px(400, 300)
    assert_same plot, plot.title("t")
    assert_same plot, plot.xlabel("x").ylabel("y")
    assert_same plot, plot.line(@x, @y)
    assert_same plot, plot.grid(true).legend(:best).yscale(:log)
  end

  def test_save_png
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      Ruviz.plot.size_px(320, 240).line(@x, @y, label: "e", color: "blue").save(path)
      assert File.exist?(path)
      # PNG magic number
      assert_equal "\x89PNG".b, File.binread(path, 4)
    end
  end

  def test_save_svg
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.svg")
      Ruviz.plot.line(@x, @y).save(path)
      assert_includes File.read(path), "<svg"
    end
  end

  def test_line_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.line([1, 2, 3], [1, 2]) }
  end

  def test_save_without_series_raises
    Dir.mktmpdir do |dir|
      assert_raises(ArgumentError) { Ruviz.plot.save(File.join(dir, "empty.png")) }
    end
  end

  def test_invalid_color_raises
    assert_raises(ArgumentError) { Ruviz.plot.line(@x, @y, color: "not-a-color") }
  end

  def test_invalid_scale_raises
    assert_raises(ArgumentError) { Ruviz.plot.yscale(:bogus) }
  end

  def test_invalid_legend_raises
    assert_raises(ArgumentError) { Ruviz.plot.legend(:nowhere) }
  end

  def test_integer_data_accepted
    Dir.mktmpdir do |dir|
      path = File.join(dir, "int.png")
      Ruviz.plot.line([0, 1, 2, 3], [0, 1, 4, 9]).save(path)
      assert File.exist?(path)
    end
  end
end
