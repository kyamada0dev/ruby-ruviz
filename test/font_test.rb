require "minitest/autorun"
require "tmpdir"
require "ruviz"

class FontTest < Minitest::Test
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

  def test_font_family_named
    assert png?(render { |p| p.font_family("JetBrainsMono Nerd Font Mono") })
  end

  def test_font_family_generic
    assert png?(render { |p| p.font_family(:monospace) })
  end

  def test_unknown_font_falls_back
    # An unresolved family should render (system fallback), not raise.
    assert png?(render { |p| p.font_family("No Such Font 12345") })
  end

  def test_font_size
    assert png?(render { |p| p.font_size(18) })
  end

  def test_font_size_non_positive_raises
    assert_raises(ArgumentError) { Ruviz.plot.font_size(0) }
    assert_raises(ArgumentError) { Ruviz.plot.font_size(-3) }
  end

  def test_title_size_and_legend_size
    assert png?(render { |p| p.title("t").title_size(22).legend_font_size(10) })
  end

  def test_scale_typography
    assert png?(render { |p| p.scale_typography(1.4) })
  end

  def test_typography_non_positive_raises
    assert_raises(ArgumentError) { Ruviz.plot.title_size(0) }
    assert_raises(ArgumentError) { Ruviz.plot.legend_font_size(-1) }
    assert_raises(ArgumentError) { Ruviz.plot.scale_typography(0) }
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.font_family(:serif).font_size(12)
             .title_size(20).legend_font_size(11).scale_typography(1.1)
  end
end
