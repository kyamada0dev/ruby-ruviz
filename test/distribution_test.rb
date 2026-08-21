require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class DistributionTest < Minitest::Test
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

  def sample
    Array.new(500) { |i| Math.sin(i * 0.37) + Math.cos(i * 0.11) }
  end

  def test_kde
    assert png?(save_png { |p| p.kde(sample, label: "density", color: "#2563eb") })
  end

  def test_ecdf
    assert png?(save_png { |p| p.ecdf(sample, color: "#059669") })
  end

  def test_violin
    assert png?(save_png { |p| p.violin(sample, label: "v", color: "#7c3aed", alpha: 0.6) })
  end

  def test_numo_input
    assert png?(save_png { |p| p.kde(Numo::DFloat.new(400).rand_norm) })
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.kde([1.0, 2.0, 3.0])
    assert_same plot, plot.ecdf([1.0, 2.0, 3.0])
    assert_same plot, plot.violin([1.0, 2.0, 3.0])
  end

  def test_empty_raises
    assert_raises(ArgumentError) { Ruviz.plot.kde([]) }
    assert_raises(ArgumentError) { Ruviz.plot.ecdf([]) }
    assert_raises(ArgumentError) { Ruviz.plot.violin([]) }
  end
end
