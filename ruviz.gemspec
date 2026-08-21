require_relative "lib/ruviz/version"

Gem::Specification.new do |spec|
  spec.name          = "ruviz"
  spec.version       = Ruviz::VERSION
  spec.summary       = "Ruby bindings for the ruviz plotting library"
  spec.description   = "Thin, idiomatic Ruby bindings over the Rust `ruviz` " \
                       "plotting library, with first-class Numo::NArray and " \
                       "Polars data input."
  spec.homepage      = "https://github.com/kyamada0dev/ruby-ruviz"
  spec.license       = "MIT"

  spec.author        = "kyamada0dev"
  spec.email         = "dev.kenji.yamada@gmail.com"

  spec.files         = Dir["*.{md,txt}", "{ext,lib}/**/*", "Cargo.*"]
  spec.require_path  = "lib"
  spec.extensions    = ["ext/ruviz/extconf.rb"]

  spec.required_ruby_version = ">= 3.3"

  spec.add_dependency "rb_sys"
end
