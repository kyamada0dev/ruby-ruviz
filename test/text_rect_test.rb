require "minitest/autorun"
require "tmpdir"
require "ruviz"

class TextRectTest < Minitest::Test
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

  def test_annotate_text_plain
    assert png?(render { |p| p.annotate_text(5.0, 0.5, "peak") })
  end

  def test_annotate_text_styled
    assert png?(render { |p| p.annotate_text(5.0, 0.5, "peak", color: "red", size: 16.0) })
  end

  def test_annotate_text_bad_color_raises
    assert_raises(ArgumentError) { Ruviz.plot.annotate_text(0, 0, "x", color: "not-a-color") }
  end

  def test_rect_plain
    assert png?(render { |p| p.rect(2.0, -0.5, 3.0, 1.0) })
  end

  def test_rect_styled
    assert png?(render { |p| p.rect(2.0, -0.5, 3.0, 1.0, color: "#38bdf8", line_width: 2.0) })
  end

  def test_chaining
    plot = Ruviz.plot
    assert_same plot, plot.annotate_text(1, 1, "a").rect(0, 0, 1, 1)
  end
end
