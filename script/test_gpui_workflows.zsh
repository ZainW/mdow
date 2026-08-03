#!/usr/bin/env zsh
set -euo pipefail

ROOT_DIR="${0:A:h:h}"
WORKFLOW="$ROOT_DIR/.github/workflows/gpui.yml"
RELEASE_WORKFLOW="$ROOT_DIR/.github/workflows/release.yml"
PACKAGE_JSON="$ROOT_DIR/package.json"

[[ -f "$WORKFLOW" ]] || {
  print -u2 -- "FAIL: missing GPUI workflow: $WORKFLOW"
  exit 1
}
[[ -f "$RELEASE_WORKFLOW" ]] || {
  print -u2 -- "FAIL: missing release workflow: $RELEASE_WORKFLOW"
  exit 1
}

ruby - "$WORKFLOW" "$RELEASE_WORKFLOW" "$PACKAGE_JSON" <<'RUBY'
require 'json'
require 'fileutils'
require 'tmpdir'
require 'yaml'

workflow_path, release_workflow_path, package_json_path = ARGV

def assert(condition, message)
  return if condition

  abort("FAIL: #{message}")
end

def archive_check_succeeds?(command, archives, environment = {})
  Dir.mktmpdir('mdow-gpui-workflow-test') do |root|
    archives.each do |archive|
      path = File.join(root, archive)
      FileUtils.mkdir_p(File.dirname(path))
      FileUtils.touch(path)
    end

    Dir.chdir(root) do
      system(
        environment,
        'bash', '-e', '-u', '-o', 'pipefail', '-c', command,
        out: File::NULL, err: File::NULL,
      )
    end
  end
end

def package_smoke_creates_expected_archives?(command, environment)
  Dir.mktmpdir('mdow-gpui-release-smoke-test') do |root|
    fake_bin = File.join(root, 'bin')
    FileUtils.mkdir_p(fake_bin)
    fake_pnpm = File.join(fake_bin, 'pnpm')
    File.write(
      fake_pnpm,
      <<~'BASH',
        #!/usr/bin/env bash
        set -euo pipefail
        [[ "$*" == "run package:gpui-mac-beta" ]]
        [[ "$VERSION" == "0.0.0-ci" ]]
        [[ "$CI" == "false" ]]
        mkdir -p dist/gpui-mac
        touch dist/gpui-mac/MdowNative-0.0.0-ci-arm64-mac-beta.zip
        touch dist/gpui-mac/MdowNative-mac-beta.zip
      BASH
    )
    FileUtils.chmod(0o755, fake_pnpm)

    smoke_environment = environment.transform_values(&:to_s).merge(
      'PATH' => "#{fake_bin}:#{ENV.fetch('PATH')}",
    )
    succeeded = Dir.chdir(root) do
      system(
        smoke_environment,
        'bash', '-e', '-u', '-o', 'pipefail', '-c', command,
        out: File::NULL, err: File::NULL,
      )
    end
    expected_archives = [
      'dist/gpui-mac/MdowNative-0.0.0-ci-arm64-mac-beta.zip',
      'dist/gpui-mac/MdowNative-mac-beta.zip',
    ]
    succeeded && expected_archives.all? { |archive| File.file?(File.join(root, archive)) }
  end
end

def cleanup_deletes_configured_keychain?(command, keychain_template)
  return false unless keychain_template.is_a?(String)

  Dir.mktmpdir('mdow-gpui-keychain-cleanup-test') do |root|
    runner_temp = File.join(root, 'runner-temp')
    fake_bin = File.join(root, 'bin')
    FileUtils.mkdir_p([runner_temp, fake_bin])
    security_log = File.join(root, 'security.log')
    fake_security = File.join(fake_bin, 'security')
    File.write(
      fake_security,
      <<~'BASH',
        #!/usr/bin/env bash
        set -euo pipefail
        printf '%s\n' "$*" > "$FAKE_SECURITY_LOG"
      BASH
    )
    FileUtils.chmod(0o755, fake_security)

    keychain_path = keychain_template.sub('${{ runner.temp }}', runner_temp)
    succeeded = system(
      {
        'FAKE_SECURITY_LOG' => security_log,
        'KEYCHAIN_PATH' => keychain_path,
        'PATH' => "#{fake_bin}:#{ENV.fetch('PATH')}",
        'RUNNER_TEMP' => runner_temp,
      },
      'bash', '-e', '-u', '-o', 'pipefail', '-c', command,
      out: File::NULL, err: File::NULL,
    )
    succeeded && File.file?(security_log) &&
      File.read(security_log).strip == "delete-keychain #{keychain_path}"
  end
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
archive_check = steps.find { |step| step['name'] == 'Verify expected GPUI package ZIPs' }
assert(!archive_check.nil?, 'verify must check both expected package ZIPs before upload')
assert(
  steps.index(archive_check) < steps.index(artifact),
  'verify must check both expected package ZIPs before uploading artifacts',
)
archive_check_command = archive_check.fetch('run')
versioned_archive = 'dist/gpui-mac/MdowNative-0.0.0-ci-arm64-mac-beta.zip'
alias_archive = 'dist/gpui-mac/MdowNative-mac-beta.zip'
{
  'no archives' => [[], false],
  'versioned archive only' => [[versioned_archive], false],
  'alias archive only' => [[alias_archive], false],
  'both expected archives' => [[versioned_archive, alias_archive], true],
}.each do |layout, (archives, expected_success)|
  actual_success = archive_check_succeeds?(archive_check_command, archives)
  assert(
    actual_success == expected_success,
    "package archive check must #{expected_success ? 'pass' : 'fail'} with #{layout}",
  )
end
assert(artifact.dig('with', 'name') == 'mdow-native-ci-arm64', 'artifact name must be mdow-native-ci-arm64')
assert(artifact.dig('with', 'path') == 'dist/gpui-mac/*.zip', 'artifact path must be dist/gpui-mac/*.zip')
assert(artifact.dig('with', 'if-no-files-found') == 'error', 'artifact must fail when ZIP files are absent')

release_workflow = YAML.load_file(release_workflow_path)
assert(release_workflow.is_a?(Hash), 'release workflow must parse to a mapping')
release_jobs = release_workflow.fetch('jobs')
assert(release_jobs.key?('gpui-mac-beta'), 'release workflow must define gpui-mac-beta')
assert(!release_jobs.key?('native-mac-beta'), 'release workflow must not define native-mac-beta')

electron_job = release_jobs.fetch('release')
expected_electron_matrix = [
  { 'os' => 'macos-latest', 'platform' => 'mac' },
  { 'os' => 'ubuntu-latest', 'platform' => 'linux' },
  { 'os' => 'windows-latest', 'platform' => 'windows' },
]
assert(
  electron_job.dig('strategy', 'matrix', 'include') == expected_electron_matrix,
  'Electron release matrix must remain macOS, Linux, and Windows',
)
electron_steps = electron_job.fetch('steps')
electron_certificate = electron_steps.find { |step| step['name'] == 'Import Apple signing certificate' }
assert(!electron_certificate.nil?, 'Electron release must keep its signing-certificate import')
assert(
  electron_certificate['if'] == "matrix.platform == 'mac'",
  'Electron certificate import must remain scoped to macOS',
)
electron_publish = electron_steps.find { |step| step['name'] == 'Build and publish' }
assert(
  electron_publish && electron_publish['run'] == 'pnpm run --filter desktop publish',
  'Electron release must keep the desktop publish command',
)

gpui_job = release_jobs.fetch('gpui-mac-beta')
assert(gpui_job['runs-on'] == 'macos-15', 'gpui-mac-beta must run on macos-15')
gpui_steps = gpui_job.fetch('steps')
assert(gpui_steps.is_a?(Array), 'gpui-mac-beta steps must be a list')

gpui_checkout = gpui_steps.find { |step| step['uses'] == 'actions/checkout@v5' }
assert(!gpui_checkout.nil?, 'gpui-mac-beta must use actions/checkout@v5')

gpui_full_xcode = gpui_steps.find { |step| step['name'] == 'Select full Xcode' }
assert(
  gpui_full_xcode &&
    gpui_full_xcode['run'] == 'sudo xcode-select -s /Applications/Xcode.app/Contents/Developer',
  'gpui-mac-beta must explicitly select the full Xcode toolchain',
)

gpui_metal = gpui_steps.find { |step| step['name'] == 'Ensure Metal toolchain is available' }
assert(
  gpui_metal && gpui_metal['run'] == 'xcrun metal -v || xcodebuild -downloadComponent MetalToolchain',
  'gpui-mac-beta must verify or install the Metal toolchain',
)

gpui_rust = gpui_steps.find { |step| step['uses'] == 'dtolnay/rust-toolchain@1.93.0' }
assert(!gpui_rust.nil?, 'gpui-mac-beta must install Rust 1.93.0')
gpui_components = gpui_rust.dig('with', 'components').to_s.split(/[\s,]+/)
%w[rustfmt clippy].each do |component|
  assert(gpui_components.include?(component), "release Rust setup must install #{component}")
end

gpui_cache = gpui_steps.find { |step| step['uses'] == 'actions/cache@v4' }
assert(!gpui_cache.nil?, 'gpui-mac-beta must cache Cargo data')
gpui_cache_paths = gpui_cache.dig('with', 'path').to_s.lines.map(&:strip)
['~/.cargo/registry', '~/.cargo/git', 'apps/gpui/target'].each do |path|
  assert(gpui_cache_paths.include?(path), "release Cargo cache must include #{path}")
end
assert(
  gpui_cache.dig('with', 'key').to_s.include?("hashFiles('apps/gpui/Cargo.lock')"),
  'release Cargo cache key must use apps/gpui/Cargo.lock',
)

release_gate_commands = [
  'cargo fmt --manifest-path apps/gpui/Cargo.toml -- --check',
  'cargo test --locked --manifest-path apps/gpui/Cargo.toml',
  'cargo clippy --locked --manifest-path apps/gpui/Cargo.toml --all-targets -- -D warnings',
  'cargo build --release --locked --manifest-path apps/gpui/Cargo.toml',
  'pnpm run test:package:gpui-mac-beta',
  'VERSION=0.0.0-ci pnpm run package:gpui-mac-beta',
]
release_gate_positions = release_gate_commands.map do |command|
  position = gpui_steps.index { |step| step['run'] == command }
  assert(!position.nil?, "gpui-mac-beta must run #{command}")
  position
end
assert(
  release_gate_positions == release_gate_positions.sort,
  'gpui-mac-beta must run the complete GPUI verification and package preflight in order',
)
release_pnpm = gpui_steps.find { |step| step['uses'] == 'pnpm/action-setup@v6' }
assert(!release_pnpm.nil?, 'gpui-mac-beta package preflights must set up pnpm')
release_node = gpui_steps.find { |step| step['uses'] == 'actions/setup-node@v6' }
assert(!release_node.nil?, 'gpui-mac-beta package preflights must set up Node')
assert(
  gpui_steps.index(release_pnpm) < release_gate_positions.fetch(-2) &&
    gpui_steps.index(release_node) < release_gate_positions.fetch(-2),
  'gpui-mac-beta must set up pnpm and Node before package preflights',
)
release_smoke = gpui_steps.fetch(release_gate_positions.last)
assert(
  release_smoke.dig('env', 'CI') == 'false',
  'gpui-mac-beta package smoke must explicitly use ad-hoc CI=false signing',
)
assert(
  package_smoke_creates_expected_archives?(release_smoke.fetch('run'), release_smoke.fetch('env')),
  'gpui-mac-beta package smoke must run with CI=false and create both non-release ZIPs',
)

gpui_certificate = gpui_steps.find { |step| step['name'] == 'Import Apple signing certificate' }
assert(!gpui_certificate.nil?, 'gpui-mac-beta must import the Apple signing certificate')
gpui_keychain_path = gpui_certificate.dig('env', 'KEYCHAIN_PATH')
assert(
  gpui_keychain_path == '${{ runner.temp }}/gpui-mac-signing.keychain-db',
  'gpui-mac-beta must define its keychain path before certificate setup starts',
)
gpui_certificate_command = gpui_certificate.fetch('run')
assert(
  gpui_certificate_command.include?('security create-keychain -p "$KEYCHAIN_PASSWORD" "$KEYCHAIN_PATH"'),
  'gpui-mac-beta certificate setup must use the step-scoped temporary keychain',
)
assert(
  gpui_certificate_command.include?('echo "KEYCHAIN_PATH=$KEYCHAIN_PATH" >> "$GITHUB_ENV"'),
  'gpui-mac-beta must export KEYCHAIN_PATH for the packager',
)

gpui_package = gpui_steps.find { |step| step['name'] == 'Build signed and notarized Mdow Native beta' }
assert(!gpui_package.nil?, 'gpui-mac-beta must package the signed and notarized beta')
assert(
  gpui_package['run'].to_s.strip ==
    'VERSION="${GITHUB_REF_NAME#v}" bash script/package_gpui_mac_beta.sh',
  'gpui-mac-beta must derive VERSION from the tag and run the GPUI packager',
)
assert(gpui_package.dig('env', 'CI') == 'true', 'gpui-mac-beta packaging must export CI=true')
assert(
  gpui_package.dig('env', 'GITHUB_RUN_NUMBER') == '${{ github.run_number }}',
  'gpui-mac-beta packaging must pass the GitHub run number',
)
{
  'APPLE_ID' => '${{ secrets.APPLE_ID }}',
  'APPLE_APP_SPECIFIC_PASSWORD' => '${{ secrets.APPLE_APP_SPECIFIC_PASSWORD }}',
  'APPLE_TEAM_ID' => '${{ secrets.APPLE_TEAM_ID }}',
}.each do |name, value|
  assert(gpui_package.dig('env', name) == value, "gpui-mac-beta packaging must pass #{name}")
end
assert(
  !gpui_package.fetch('env').key?('NATIVE_MAC_CODESIGN_IDENTITY') &&
    !gpui_package.fetch('env').key?('CSC_NAME'),
  'gpui-mac-beta must discover its Developer ID identity from KEYCHAIN_PATH',
)
assert(
  gpui_package.dig('env', 'KEYCHAIN_PATH') == gpui_keychain_path,
  'gpui-mac-beta packaging must use the same explicit keychain as certificate setup',
)
assert(
  release_gate_positions.last < gpui_steps.index(gpui_certificate) &&
    gpui_steps.index(gpui_certificate) < gpui_steps.index(gpui_package),
  'gpui-mac-beta must gate before importing the certificate and packaging',
)

release_archive_check = gpui_steps.find { |step| step['name'] == 'Verify expected GPUI package ZIPs' }
assert(!release_archive_check.nil?, 'gpui-mac-beta must check both expected ZIPs before upload')
release_archive_command = release_archive_check.fetch('run')
release_versioned_archive = 'dist/gpui-mac/MdowNative-1.2.3-arm64-mac-beta.zip'
release_alias_archive = 'dist/gpui-mac/MdowNative-mac-beta.zip'
{
  'no archives' => [[], false],
  'versioned archive only' => [[release_versioned_archive], false],
  'alias archive only' => [[release_alias_archive], false],
  'both expected archives' => [[release_versioned_archive, release_alias_archive], true],
}.each do |layout, (archives, expected_success)|
  actual_success = archive_check_succeeds?(
    release_archive_command,
    archives,
    { 'GITHUB_REF_NAME' => 'v1.2.3' },
  )
  assert(
    actual_success == expected_success,
    "release archive check must #{expected_success ? 'pass' : 'fail'} with #{layout}",
  )
end

gpui_artifact = gpui_steps.find { |step| step['uses'] == 'actions/upload-artifact@v4' }
assert(!gpui_artifact.nil?, 'gpui-mac-beta must upload a workflow artifact')
assert(
  gpui_artifact.dig('with', 'path') == 'dist/gpui-mac/*.zip',
  'gpui-mac-beta workflow artifact must upload dist/gpui-mac/*.zip',
)
assert(
  gpui_artifact.dig('with', 'if-no-files-found') == 'error',
  'gpui-mac-beta workflow artifact must fail when ZIP files are absent',
)

draft_release = gpui_steps.find { |step| step['name'] == 'Ensure draft GitHub release exists' }
assert(!draft_release.nil?, 'gpui-mac-beta must keep draft-release creation')
github_release_upload = gpui_steps.find { |step| step['name'] == 'Upload GPUI Mac beta' }
assert(!github_release_upload.nil?, 'gpui-mac-beta must upload ZIPs to the GitHub release')
assert(
  github_release_upload['run'].to_s.include?(
    'gh release upload "$TAG" dist/gpui-mac/MdowNative-*.zip --clobber',
  ),
  'gpui-mac-beta must upload MdowNative GPUI ZIPs to the GitHub release',
)
assert(
  gpui_steps.index(release_archive_check) < gpui_steps.index(gpui_artifact) &&
    gpui_steps.index(release_archive_check) < gpui_steps.index(github_release_upload),
  'gpui-mac-beta must verify both ZIPs before either upload',
)
gpui_cleanup = gpui_steps.find { |step| step['name'] == 'Clean up keychain' }
assert(!gpui_cleanup.nil?, 'gpui-mac-beta must keep keychain cleanup')
assert(gpui_cleanup['if'] == '${{ always() }}', 'gpui-mac-beta keychain cleanup must always run')
assert(
  gpui_cleanup.dig('env', 'KEYCHAIN_PATH') == gpui_keychain_path,
  'gpui-mac-beta cleanup must receive the same explicit keychain as certificate setup',
)
assert(
  cleanup_deletes_configured_keychain?(
    gpui_cleanup.fetch('run'),
    gpui_cleanup.dig('env', 'KEYCHAIN_PATH'),
  ),
  'gpui-mac-beta cleanup must resolve the step-scoped keychain without a completed import',
)

publish_needs = release_jobs.fetch('publish').fetch('needs')
assert(
  publish_needs.is_a?(Array) && publish_needs.sort == %w[gpui-mac-beta release],
  'publish must need both Electron release and gpui-mac-beta',
)

release_commands = release_jobs.values.flat_map do |job|
  job.fetch('steps', []).filter_map { |step| step['run'] }
end
forbidden_release_references = /Swift|package:native-mac-beta|dist\/native-mac/
assert(
  release_commands.none? { |command| command.match?(forbidden_release_references) },
  'release commands must not reference Swift or the removed native-mac package',
)

scripts = JSON.parse(File.read(package_json_path)).fetch('scripts')
assert(
  scripts['test:gpui-workflows'] == 'zsh script/test_gpui_workflows.zsh',
  'package.json must expose test:gpui-workflows',
)

puts 'PASS: GPUI workflow contracts'
RUBY
