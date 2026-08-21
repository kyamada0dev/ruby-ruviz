require "minitest/autorun"
require "tmpdir"
require "ruviz"
require "numo/narray"

class PieRadarTest < Minitest::Test
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

  def test_pie
    assert png?(save_png { |p| p.pie([30, 20, 40, 10], labels: %w[a b c d]) })
  end

  def test_donut
    assert png?(save_png { |p| p.pie([1, 2, 3], donut: 0.5) })
  end

  def test_pie_label_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.pie([1, 2, 3], labels: %w[a b]) }
  end

  def test_pie_bad_donut_raises
    assert_raises(ArgumentError) { Ruviz.plot.pie([1, 2], donut: 1.5) }
  end

  def test_radar_hash_series
    labels = %w[speed power range agility cost]
    assert png?(save_png do |p|
      p.radar(labels, "A" => [4, 3, 5, 2, 3], "B" => [2, 5, 3, 4, 4])
    end)
  end

  def test_radar_single_array
    assert png?(save_png { |p| p.radar(%w[a b c d], [1, 2, 3, 4]) })
  end

  def test_radar_array_of_hashes_and_numo
    labels = %w[a b c]
    assert png?(save_png do |p|
      p.radar(labels, [{ name: "x", values: Numo::DFloat[1, 2, 3] }, { name: "y", values: [3, 2, 1] }])
    end)
  end

  def test_radar_length_mismatch_raises
    assert_raises(ArgumentError) { Ruviz.plot.radar(%w[a b c], [1, 2]) }
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.pie([1, 2, 3])
  end
end
