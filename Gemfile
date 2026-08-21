source "https://rubygems.org"

gemspec

gem "rake"
gem "rake-compiler"
gem "minitest"

# Optional data-input backends exercised by the tests/benchmarks.
group :development, :test do
  gem "numo-narray-alt"
  # Forked polars-df: its optimized Series#to_numo makes the Polars -> ruviz
  # ingest path fast (the released gem's to_numo is the bottleneck). See
  # benchmark/plot.rb.
  gem "polars-df",
      git: "https://github.com/kyamada0dev/ruby-polars.git",
      branch: "feat/numo-fast-conversion"
end
