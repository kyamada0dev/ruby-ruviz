require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class HeatmapContourTest < Minitest::Test
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

  def test_heatmap_array_of_arrays
    m = (0..5).map { |i| (0..5).map { |j| Math.sin(i * 0.5) * Math.cos(j * 0.5) } }
    assert png?(save_png { |p| p.heatmap(m) })
  end

  def test_heatmap_numo_2d
    m = Numo::DFloat.new(8, 8).seq / 10.0
    assert png?(save_png { |p| p.heatmap(m) })
  end

  def test_heatmap_ragged_raises
    assert_raises(ArgumentError) { Ruviz.plot.heatmap([[1, 2, 3], [4, 5]]) }
  end

  def test_heatmap_empty_raises
    assert_raises(ArgumentError) { Ruviz.plot.heatmap([]) }
  end

  def test_contour_flat_z
    xs = (0..15).map { |i| i / 3.0 }
    ys = (0..12).map { |j| j / 3.0 }
    z = ys.flat_map { |y| xs.map { |x| Math.sin(x) * Math.cos(y) } } # ny*nx row-major
    assert png?(save_png { |p| p.contour(xs, ys, z, levels: 10) })
  end

  def test_contour_2d_z_is_flattened
    xs = [0.0, 1.0, 2.0]
    ys = [0.0, 1.0]
    z2d = [[0.0, 1.0, 2.0], [3.0, 4.0, 5.0]] # ny=2 rows, nx=3 cols
    assert png?(save_png { |p| p.contour(xs, ys, z2d, filled: true) })
  end

  def test_contour_z_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.contour([0, 1, 2], [0, 1], [1, 2, 3]) }
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.heatmap([[1, 2], [3, 4]])
  end
end
