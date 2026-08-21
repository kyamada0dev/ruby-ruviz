# frozen_string_literal: true

require_relative "ruviz/version"

# Native extension. Support a per-Ruby-version subdir (used when precompiled),
# falling back to the plain build produced by `rake compile` during development.
begin
  require "ruviz/#{RUBY_VERSION.to_f}/ruviz"
rescue LoadError
  require "ruviz/ruviz"
end

# Ruby bindings for the Rust `ruviz` plotting library.
#
# The Ruby API is a thin, idiomatic facade over the Rust builder; ruviz itself
# performs all layout, scaling and rendering.
module Ruviz
end
