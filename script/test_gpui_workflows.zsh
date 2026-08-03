#!/usr/bin/env zsh
set -euo pipefail

ROOT_DIR="${0:A:h:h}"
WORKFLOW="$ROOT_DIR/.github/workflows/gpui.yml"
PACKAGE_JSON="$ROOT_DIR/package.json"

[[ -f "$WORKFLOW" ]] || {
  print -u2 -- "FAIL: missing GPUI workflow: $WORKFLOW"
  exit 1
}

ruby - "$WORKFLOW" "$PACKAGE_JSON" <<'RUBY'
require 'json'
require 'yaml'

workflow_path, package_json_path = ARGV

def assert(condition, message)
  return if condition

  abort("FAIL: #{message}")
end

workflow = YAML.load_file(workflow_path)
assert(workflow.is_a?(Hash), 'workflow must parse to a mapping')

triggers = workflow.fetch('on')
required_paths = [
  'apps/gpui/**',
  'script/package_gpui_mac_beta.sh',
  'script/native_mac_bundle.sh',
  'script/test_package_gpui_mac_beta.zsh',
  '.github/workflows/gpui.yml',
]

%w[pull_request push].each do |event|
  event_config = triggers.fetch(event)
  assert(event_config.is_a?(Hash), "#{event} must have a configuration mapping")
  paths = event_config.fetch('paths')
  assert(paths.is_a?(Array), "#{event} paths must be a list")
  required_paths.each do |path|
    assert(paths.include?(path), "#{event} must watch #{path}")
  end
end

verify_job = workflow.fetch('jobs').fetch('verify')
assert(verify_job['runs-on'] == 'macos-15', 'verify must run on macos-15')
steps = verify_job.fetch('steps')
assert(steps.is_a?(Array), 'verify steps must be a list')

checkout = steps.find { |step| step['uses'] == 'actions/checkout@v5' }
assert(!checkout.nil?, 'verify must use actions/checkout@v5')

full_xcode = steps.find { |step| step['name'] == 'Select full Xcode' }
assert(
  full_xcode && full_xcode['run'] == 'sudo xcode-select -s /Applications/Xcode.app/Contents/Developer',
  'verify must explicitly select the full Xcode toolchain',
)

metal = steps.find { |step| step['name'] == 'Ensure Metal toolchain is available' }
assert(
  metal && metal['run'] == 'xcrun metal -v || xcodebuild -downloadComponent MetalToolchain',
  'verify must verify or install the Metal toolchain',
)

rust = steps.find { |step| step['uses'] == 'dtolnay/rust-toolchain@1.93.0' }
assert(!rust.nil?, 'verify must install Rust 1.93.0')
components = rust.dig('with', 'components').to_s.split(/[\s,]+/)
%w[rustfmt clippy].each do |component|
  assert(components.include?(component), "Rust setup must install #{component}")
end

cache = steps.find { |step| step['uses'] == 'actions/cache@v4' }
assert(!cache.nil?, 'verify must cache Cargo data')
cache_paths = cache.dig('with', 'path').to_s.lines.map(&:strip)
['~/.cargo/registry', '~/.cargo/git', 'apps/gpui/target'].each do |path|
  assert(cache_paths.include?(path), "Cargo cache must include #{path}")
end
cache_key = cache.dig('with', 'key').to_s
assert(cache_key.include?("hashFiles('apps/gpui/Cargo.lock')"), 'Cargo cache key must use apps/gpui/Cargo.lock')

required_commands = [
  'cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check',
  'cargo test --locked --manifest-path apps/gpui/Cargo.toml',
  'cargo clippy --locked --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings',
  'cargo build --release --locked --manifest-path apps/gpui/Cargo.toml',
  'pnpm run test:package:gpui-mac-beta',
  'VERSION=0.0.0-ci pnpm run package:gpui-mac-beta',
]
command_positions = required_commands.map do |command|
  position = steps.index { |step| step['run'] == command }
  assert(!position.nil?, "verify must run #{command}")
  position
end
assert(
  command_positions == command_positions.sort,
  'verify must run formatting, tests, lint, build, package test, and package in order',
)
assert(
  !steps.any? { |step| step['run'].to_s.match?(/(?:^|\s)CI=true(?:\s|$)/) },
  'verify must preserve ad-hoc signing for the package smoke test',
)
package_step = steps.fetch(command_positions.last)
assert(
  package_step.dig('env', 'CI') == 'false',
  'the package smoke test must override GitHub Actions CI=true for ad-hoc signing',
)

artifact = steps.find { |step| step['uses'] == 'actions/upload-artifact@v4' }
assert(!artifact.nil?, 'verify must upload a package artifact')
assert(artifact.dig('with', 'name') == 'mdow-native-ci-arm64', 'artifact name must be mdow-native-ci-arm64')
assert(artifact.dig('with', 'path') == 'dist/gpui-mac/*.zip', 'artifact path must be dist/gpui-mac/*.zip')
assert(artifact.dig('with', 'if-no-files-found') == 'error', 'artifact must fail when ZIP files are absent')

scripts = JSON.parse(File.read(package_json_path)).fetch('scripts')
assert(
  scripts['test:gpui-workflows'] == 'zsh script/test_gpui_workflows.zsh',
  'package.json must expose test:gpui-workflows',
)

puts 'PASS: GPUI workflow contract'
RUBY
