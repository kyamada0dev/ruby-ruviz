require "bundler/gem_tasks"
require "rake/testtask"
require "rake/extensiontask"

Rake::TestTask.new do |t|
  t.test_files = FileList["test/**/*_test.rb"]
end

task default: :test

gemspec = Bundler.load_gemspec("ruviz.gemspec")
Rake::ExtensionTask.new("ruviz", gemspec) do |ext|
  ext.lib_dir = "lib/ruviz"
end

task :remove_ext do
  Dir["lib/ruviz/ruviz.{bundle,so}"].each { |path| File.unlink(path) }
end

Rake::Task["build"].enhance [:remove_ext]
