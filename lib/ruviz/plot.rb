# frozen_string_literal: true

module Ruviz
  # Raised for ruviz rendering / IO failures.
  class Error < StandardError; end

  # Fluent, chainable facade over the native +Ruviz::PlotHandle+.
  #
  # Every configuration method mutates the underlying native state and returns
  # +self+, so calls can be chained; +save+ renders via ruviz and writes a file.
  #
  #   Ruviz.plot
  #     .size_px(760, 420)
  #     .title("Decay")
  #     .line(x, y, label: "fast", color: "#2563eb", width: 2.0)
  #     .yscale(:log)
  #     .grid(true)
  #     .legend(:upper_right)
  #     .save("decay.png")
  class Plot
    def initialize
      @handle = PlotHandle.new
    end

    def size_px(width, height)
      @handle.size_px(Integer(width), Integer(height))
      self
    end

    def title(text)
      @handle.title(text.to_s)
      self
    end

    def xlabel(text)
      @handle.xlabel(text.to_s)
      self
    end

    def ylabel(text)
      @handle.ylabel(text.to_s)
      self
    end

    # @param scale [Symbol, String] :linear, :log or :symlog
    # @param linthresh [Float, nil] threshold for :symlog
    def xscale(scale, linthresh: nil)
      @handle.xscale(scale.to_s, linthresh && Float(linthresh))
      self
    end

    def yscale(scale, linthresh: nil)
      @handle.yscale(scale.to_s, linthresh && Float(linthresh))
      self
    end

    def grid(enabled = true)
      @handle.grid(enabled ? true : false)
      self
    end

    # @param position [Symbol, String] e.g. :best, :upper_right, :outside_right
    def legend(position = :best)
      @handle.legend(position.to_s)
      self
    end

    # @param name [Symbol, String] light, dark, publication, minimal, seaborn, presentation
    def theme(name)
      @handle.theme(name.to_s)
      self
    end

    def xlim(min, max)
      @handle.xlim(Float(min), Float(max))
      self
    end

    def ylim(min, max)
      @handle.ylim(Float(min), Float(max))
      self
    end

    # Horizontal reference line at y. With no style it is a dashed gray line;
    # pass any of color/width/style to customize.
    #
    # @param style [Symbol, String] :solid, :dashed, :dotted, :dash_dot, :dash_dot_dot
    def hline(y, color: nil, width: nil, style: nil)
      @handle.hline(Float(y), color&.to_s, width && Float(width), style&.to_s)
      self
    end

    # Vertical reference line at x (see {#hline}).
    def vline(x, color: nil, width: nil, style: nil)
      @handle.vline(Float(x), color&.to_s, width && Float(width), style&.to_s)
      self
    end

    # Add a line series.
    #
    # @param x [Array<Numeric>, Numo::NArray, Polars::Series] x values
    # @param y [Array<Numeric>, Numo::NArray, Polars::Series] y values
    def line(x, y, label: nil, color: nil, width: nil)
      @handle.line(
        coerce_data(x),
        coerce_data(y),
        label&.to_s,
        color&.to_s,
        width && Float(width)
      )
      self
    end

    # Add a scatter series.
    #
    # @param marker [Symbol, String] e.g. :circle, :square, :triangle_down, :diamond_open
    def scatter(x, y, label: nil, color: nil, marker: nil, marker_size: nil, alpha: nil)
      @handle.scatter(
        coerce_data(x),
        coerce_data(y),
        label&.to_s,
        color&.to_s,
        marker&.to_s,
        marker_size && Float(marker_size),
        alpha && Float(alpha)
      )
      self
    end

    # Add a bar series.
    #
    # @param categories [Array] category labels (stringified)
    # @param values [Array<Numeric>, Numo::NArray, Polars::Series]
    def bar(categories, values, label: nil, color: nil, alpha: nil)
      @handle.bar(
        Array(categories).map(&:to_s),
        coerce_data(values),
        label&.to_s,
        color&.to_s,
        alpha && Float(alpha)
      )
      self
    end

    # Add a histogram series.
    #
    # @param bins [Integer, nil] number of bins (auto if nil)
    def histogram(data, bins: nil, label: nil, color: nil, alpha: nil)
      @handle.histogram(
        coerce_data(data),
        bins && Integer(bins),
        label&.to_s,
        color&.to_s,
        alpha && Float(alpha)
      )
      self
    end

    # Add a filled area series between the curve and +baseline+.
    def area(x, y, baseline: 0.0, label: nil, color: nil, width: nil, alpha: nil)
      @handle.area(
        coerce_data(x),
        coerce_data(y),
        Float(baseline),
        label&.to_s,
        color&.to_s,
        width && Float(width),
        alpha && Float(alpha)
      )
      self
    end

    # Add a box-and-whisker series from a 1-D sample.
    def boxplot(data, label: nil, color: nil, alpha: nil)
      @handle.boxplot(
        coerce_data(data),
        label&.to_s,
        color&.to_s,
        alpha && Float(alpha)
      )
      self
    end

    # Render and write the plot. Format is chosen by the file extension
    # (.png, .svg, .pdf); defaults to PNG.
    #
    # @param path [String]
    # @return [self]
    def save(path)
      @handle.save(path.to_s)
      self
    end

    private

    # Normalize a data argument into something the native `line` accepts.
    #
    # - Ruby Array and Numo::NArray are passed straight to Rust (Numo is read
    #   through its native buffer there — no Ruby Array is materialized).
    # - Polars Series are converted to a Numo array via the Polars binding's
    #   own +to_numo+ (native, not +to_a+), then take the same buffer path.
    # - Anything else responding to +to_a+ is accepted as a convenience.
    def coerce_data(values)
      return values if values.is_a?(::Array)
      return values if numo_narray?(values)
      return polars_series_to_numo(values) if polars_series?(values)
      return values.to_a if values.respond_to?(:to_a)

      raise ArgumentError, "unsupported data type: #{values.class}"
    end

    def numo_narray?(values)
      defined?(Numo::NArray) && values.is_a?(Numo::NArray)
    end

    def polars_series?(values)
      defined?(Polars::Series) && values.is_a?(Polars::Series)
    end

    # Native Polars -> Numo (float64), avoiding any Ruby Array intermediate.
    # Only cast when needed: an already-float64 Series skips a redundant copy.
    def polars_series_to_numo(series)
      series = series.cast(Polars::Float64) unless series.dtype == Polars::Float64
      series.to_numo
    end
  end

  # Start a new plot.
  #
  # @return [Ruviz::Plot]
  def self.plot
    Plot.new
  end
end
