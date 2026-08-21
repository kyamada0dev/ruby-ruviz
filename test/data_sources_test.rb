require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"
require "polars"

class DataSourcesTest < Minitest::Test
  def png_bytes(x, y)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      Ruviz.plot.size_px(320, 240).line(x, y).save(path)
      File.binread(path)
    end
  end

  def test_numo_dfloat
    x = Numo::DFloat[0, 1, 2, 3, 4]
    y = Numo::DFloat[0, 1, 4, 9, 16]
    bytes = png_bytes(x, y)
    assert_equal "\x89PNG".b, bytes[0, 4]
  end

  def test_numo_int64_is_cast_to_float
    x = Numo::Int64[0, 1, 2, 3]
    y = Numo::Int64[0, 1, 8, 27]
    assert_equal "\x89PNG".b, png_bytes(x, y)[0, 4]
  end

  def test_polars_series
    x = Polars::Series.new("x", [0.0, 1.0, 2.0, 3.0])
    y = Polars::Series.new("y", [0.0, 2.0, 4.0, 6.0])
    assert_equal "\x89PNG".b, png_bytes(x, y)[0, 4]
  end

  def test_polars_integer_series
    x = Polars::Series.new("x", [0, 1, 2, 3])
    y = Polars::Series.new("y", [3, 2, 1, 0])
    assert_equal "\x89PNG".b, png_bytes(x, y)[0, 4]
  end

  # The native Numo buffer path must produce the same image as the equivalent
  # Ruby Array — i.e. it reads the same numbers, just without boxing them.
  def test_numo_matches_array
    xs = (0..200).map { |i| i / 20.0 }
    ys = xs.map { |v| Math.sin(v) }
    from_array = png_bytes(xs, ys)
    from_numo = png_bytes(Numo::DFloat.cast(xs), Numo::DFloat.cast(ys))
    assert_equal from_array, from_numo
  end

  # Polars -> Numo (native) path must match the Array path too.
  def test_polars_matches_array
    xs = (0..100).map { |i| i.to_f }
    ys = xs.map { |v| v * v }
    from_array = png_bytes(xs, ys)
    from_polars = png_bytes(Polars::Series.new("x", xs), Polars::Series.new("y", ys))
    assert_equal from_array, from_polars
  end
end
