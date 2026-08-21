require "minitest/autorun"
require "ruviz"

class RuvizTest < Minitest::Test
  def test_module_defined
    assert defined?(Ruviz)
  end

  def test_native_extension_loaded
    # Phase 1 smoke test: the Rust extension is loaded and callable.
    assert_match(/native extension loaded/, Ruviz._hello)
  end

  def test_version
    assert_equal "0.1.0", Ruviz::VERSION
  end
end
