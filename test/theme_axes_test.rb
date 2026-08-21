require "minitest/autorun"
require "tmpdir"
require "ruviz"

class ThemeAxesTest < Minitest::Test
  def setup
    @x = (0..20).map { |i| i / 2.0 }
    @y = @x.map { |v| Math.sin(v) }
  end

  def render(&blk)
    Dir.mktmpdir do |dir|
      path = File.join(dir, "p.png")
      plot = Ruviz.plot.size_px(400, 300)
      blk.call(plot)
      plot.line(@x, @y)
      plot.save(path)
      File.binread(path, 4)
    end
  end

  def png?(b) = b == "\x89PNG".b

  def test_themes
    %i[light dark publication minimal seaborn presentation].each do |t|
      assert png?(render { |p| p.theme(t) }), "theme #{t} failed"
    end
  end

  def test_invalid_theme_raises
    assert_raises(ArgumentError) { Ruviz.plot.theme(:neon) }
  end

  def test_xlim_ylim
    assert png?(render { |p| p.xlim(0, 5).ylim(-1, 1) })
  end

  def test_xlim_rejects_bad_range
    assert_raises(ArgumentError) { Ruviz.plot.xlim(5, 5) }
    assert_raises(ArgumentError) { Ruviz.plot.ylim(3, 1) }
  end

  def test_hline_vline_default_and_styled
    assert png?(render { |p| p.hline(0.0).vline(2.5) })
    assert png?(render { |p| p.hline(0.5, color: "red", width: 2.0, style: :solid).vline(3.0, color: "#059669") })
  end

  def test_invalid_linestyle_raises
    assert_raises(ArgumentError) { Ruviz.plot.hline(1.0, style: :zigzag) }
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.theme(:dark).xlim(0, 1).ylim(0, 1).hline(0.5).vline(0.5)
  end
end
