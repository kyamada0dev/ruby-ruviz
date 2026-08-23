require "minitest/autorun"
require "tmpdir"
require "ruviz"

# 3D plots: scatter3d / line3d / surface / wireframe (needs the crate's `3d`
# feature, enabled in Cargo.toml).
class Plot3DTest < Minitest::Test
  def png?(b) = b == "\x89PNG".b

  def save(p3)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      p3.save(path)
      png?(File.binread(path, 4))
    end
  end

  def test_scatter3d
    p3 = Ruviz.scatter3d([0, 1, 2], [0.0, 1.0, 0.5], [0.5, 1.5, 1.0])
    assert_instance_of Ruviz::Plot3D, p3
    assert_same p3, p3.title("s").zlabel("z").marker(:circle).marker_size(6.0).color("blue")
    assert save(p3)
  end

  def test_line3d
    t = (0..40).map { |i| i * 0.2 }
    p3 = Ruviz.line3d(t.map { |v| Math.cos(v) }, t.map { |v| Math.sin(v) }, t)
    assert save(p3.line_width(2.0).color("red"))
  end

  def test_surface
    x = [-1.0, 0.0, 1.0]
    y = [-1.0, 0.0, 1.0]
    z = [[0.0, 1.0, 0.0], [1.0, 2.0, 1.0], [0.0, 1.0, 0.0]]
    assert save(Ruviz.surface(x, y, z).title("surf").zlabel("z"))
  end

  def test_wireframe
    x = [-1.0, 0.0, 1.0]
    y = [-1.0, 0.0, 1.0]
    z = [[0.0, 1.0, 0.0], [1.0, 2.0, 1.0], [0.0, 1.0, 0.0]]
    assert save(Ruviz.wireframe(x, y, z).line_width(1.5))
  end

  def test_xyz_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.scatter3d([0, 1], [0, 1], [0, 1, 2]) }
  end

  def test_surface_grid_shape_mismatch_raises
    # z must be (y.len x x.len)
    assert_raises(ArgumentError) { Ruviz.surface([0, 1, 2], [0, 1], [[0, 1]]) }
  end
end
