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

    # Text annotation anchored at data coordinates (x, y).
    #
    # @param size [Numeric, nil] font size
    def annotate_text(x, y, text, color: nil, size: nil)
      @handle.annotate_text(Float(x), Float(y), text.to_s, color&.to_s, size && Float(size))
      self
    end

    # Rectangle annotation with lower-left corner at (x, y), in data units.
    def rect(x, y, width, height, color: nil, line_width: nil)
      @handle.rect(
        Float(x), Float(y), Float(width), Float(height),
        color&.to_s, line_width && Float(line_width)
      )
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

    # Kernel density estimate of a 1-D sample.
    def kde(data, label: nil, color: nil, alpha: nil)
      @handle.kde(coerce_data(data), label&.to_s, color&.to_s, alpha && Float(alpha))
      self
    end

    # Empirical cumulative distribution of a 1-D sample.
    def ecdf(data, label: nil, color: nil, alpha: nil)
      @handle.ecdf(coerce_data(data), label&.to_s, color&.to_s, alpha && Float(alpha))
      self
    end

    # Violin plot of a 1-D sample.
    def violin(data, label: nil, color: nil, alpha: nil)
      @handle.violin(coerce_data(data), label&.to_s, color&.to_s, alpha && Float(alpha))
      self
    end

    # Heatmap of a 2-D matrix.
    #
    # @param data [Array<Array<Numeric>>, Numo::NArray, Polars::DataFrame]
    def heatmap(data)
      @handle.heatmap(coerce_matrix(data))
      self
    end

    # Contour plot. +x+ (nx) and +y+ (ny) are 1-D axes; +z+ is a flat, row-major
    # grid of length nx*ny (or any 2-D data, which is flattened).
    #
    # @param levels [Integer, nil] number of contour levels
    # @param filled [Boolean, nil] filled contours
    def contour(x, y, z, levels: nil, filled: nil)
      @handle.contour(
        coerce_data(x),
        coerce_data(y),
        coerce_data(flatten_2d(z)),
        levels && Integer(levels),
        filled.nil? ? nil : (filled ? true : false)
      )
      self
    end

    # Pie (or donut) chart from a 1-D set of slice values.
    #
    # @param labels [Array, nil] slice labels (must match values length)
    # @param donut [Float, nil] hole ratio in [0, 1) for a donut chart
    def pie(values, labels: nil, donut: nil)
      @handle.pie(
        coerce_data(values),
        labels && Array(labels).map(&:to_s),
        donut && Float(donut)
      )
      self
    end

    # Radar (spider) chart.
    #
    # @param labels [Array] the axis categories
    # @param series [Hash, Array, Numo::NArray] one or more series. Accepts a
    #   Hash of name => values, an Array of {name:, values:} hashes, an Array of
    #   value-arrays (unnamed), or a single value-array.
    def radar(labels, series)
      names, values_list = normalize_radar_series(series)
      @handle.radar(Array(labels).map(&:to_s), names, values_list)
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

    # A 2-D matrix argument: a Ruby Array of Arrays or a Numo::NArray goes to
    # Rust as-is; a Polars DataFrame is converted natively via to_numo.
    def coerce_matrix(values)
      return values if values.is_a?(::Array) || numo_narray?(values)
      return values.to_numo if values.respond_to?(:to_numo)

      raise ArgumentError, "expected a 2-D Array or Numo::NArray (got #{values.class})"
    end

    # Flatten a 2-D grid (Numo 2-D or Array-of-Arrays) to a row-major 1-D form;
    # already-1-D data passes through.
    def flatten_2d(z)
      return z.ndim >= 2 ? z.flatten : z if numo_narray?(z)
      return z.flatten if z.is_a?(::Array) && z.first.is_a?(::Array)

      z
    end

    def numo_narray?(values)
      defined?(Numo::NArray) && values.is_a?(Numo::NArray)
    end

    # Normalize the many accepted `radar` series shapes into parallel arrays of
    # names (String or nil) and float-value arrays. Radar data is per-axis and
    # small, so converting Numo to a Ruby Array here is fine.
    def normalize_radar_series(series)
      case series
      when Hash
        [series.keys.map(&:to_s), series.values.map { |v| to_float_array(v) }]
      when ::Array
        if series.first.is_a?(Hash)
          [series.map { |h| (h[:name] || h["name"])&.to_s },
           series.map { |h| to_float_array(h[:values] || h["values"]) }]
        elsif series.first.is_a?(::Array) || numo_narray?(series.first)
          [::Array.new(series.length), series.map { |v| to_float_array(v) }]
        else
          [[nil], [to_float_array(series)]] # a single flat array of numbers
        end
      else
        [[nil], [to_float_array(series)]] # e.g. a single 1-D Numo array
      end
    end

    def to_float_array(values)
      arr = numo_narray?(values) ? values.to_a : Array(values)
      arr.map { |x| Float(x) }
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
