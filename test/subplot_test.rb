require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class SubplotTest < Minitest::Test
  def png?(b) = b == "\x89PNG".b

  def line_plot(title)
    x = (0..20).map { |i| i / 5.0 }
    Ruviz.plot.title(title).line(x, x.map { |v| Math.sin(v) }, color: "#2563eb")
  end

  def test_subplots_grid_saves_png
    Dir.mktmpdir do |dir|
      path = File.join(dir, "f.png")
      Ruviz.subplots(1, 2, 640, 320)
           .suptitle("two panels")
           .subplot(0, 0, line_plot("a"))
           .subplot(0, 1, line_plot("b"))
           .save(path)
      assert png?(File.binread(path, 4))
    end
  end

  def test_subplot_at_flat_index
    Dir.mktmpdir do |dir|
      path = File.join(dir, "f.png")
      fig = Ruviz.subplots(2, 2, 480, 480)
      4.times { |i| fig = fig.subplot_at(i, line_plot("p#{i}")) }
      fig.save(path)
      assert png?(File.binread(path, 4))
    end
  end

  def test_methods_chain_returns_self
    fig = Ruviz.subplots(1, 1, 200, 200)
    assert_same fig, fig.suptitle("t")
    assert_same fig, fig.subplot(0, 0, line_plot("x"))
  end

  def test_out_of_range_cell_raises
    fig = Ruviz.subplots(1, 2, 200, 200)
    assert_raises(ArgumentError) { fig.subplot(0, 5, line_plot("x")) }
  end

  def test_non_plot_argument_raises
    fig = Ruviz.subplots(1, 1, 200, 200)
    assert_raises(ArgumentError) { fig.subplot(0, 0, "not a plot") }
  end

  def test_save_without_subplots_raises
    Dir.mktmpdir do |dir|
      assert_raises(ArgumentError) { Ruviz.subplots(1, 1, 200, 200).save(File.join(dir, "f.png")) }
    end
  end
end
