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
    # Phase 3 supports Ruby Array; Numo::NArray and Polars Series get native
    # buffer paths in later phases. Until then, objects that respond to
    # +to_a+ are converted here as a convenience (not for large data).
    def coerce_data(values)
      return values if values.is_a?(::Array)
      return values.to_a if values.respond_to?(:to_a)

      raise ArgumentError, "unsupported data type: #{values.class}"
    end
  end

  # Start a new plot.
  #
  # @return [Ruviz::Plot]
  def self.plot
    Plot.new
  end
end
