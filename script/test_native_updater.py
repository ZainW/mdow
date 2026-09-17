#!/usr/bin/env python3
"""Exercise the real Sparkle bridge against signed, isolated local updates."""
import base64
import functools
import http.server
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parent.parent
VENDOR = ROOT / 'apps/gpui/vendor/Sparkle'


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs).stdout.strip()


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass


with tempfile.TemporaryDirectory(prefix='mdow-updater-test-') as directory:
    work = Path(directory)
    run('bash', str(ROOT / 'script/fetch_sparkle.sh'))
    # Disposable test key; never reads or exports the production signing key.
    run('openssl', 'genpkey', '-algorithm', 'ed25519', '-out', str(work / 'key.pem'))
    private_der = subprocess.check_output(['openssl', 'pkey', '-in', str(work / 'key.pem'), '-outform', 'DER'])
    public_der = subprocess.check_output(['openssl', 'pkey', '-in', str(work / 'key.pem'), '-pubout', '-outform', 'DER'])
    (work / 'seed').write_bytes(base64.b64encode(private_der[-32:]))
    (work / 'seed').chmod(0o600)
    public = base64.b64encode(public_der[-32:]).decode()
    run('clang', '-fobjc-arc', '-fmodules', '-fmodules-cache-path=' + str(work / 'modules'),
        '-F' + str(VENDOR), '-framework', 'Sparkle', '-framework', 'AppKit',
        '-Wl,-rpath,@executable_path/../Frameworks',
        str(ROOT / 'apps/gpui/native/updater_test_host.m'),
        str(ROOT / 'apps/gpui/native/sparkle_bridge.m'), '-o', str(work / 'host'))
    server = http.server.ThreadingHTTPServer(('127.0.0.1', 0),
        functools.partial(QuietHandler, directory=directory))
    threading.Thread(target=server.serve_forever, daemon=True).start()
    base_url = f'http://127.0.0.1:{server.server_port}'
    marker = work / 'installed'

    def make_app(parent, version):
        app = parent / 'Mdow Updater Test.app'
        contents = app / 'Contents'
        (contents / 'MacOS').mkdir(parents=True)
        shutil.copy2(work / 'host', contents / 'MacOS/Host')
        run('bash', '-c', 'source "$1"; copy_sparkle_framework "$2" "$3"; sign_sparkle_framework "$3/Frameworks/Sparkle.framework" -',
            'test', str(ROOT / 'script/native_mac_bundle.sh'), str(VENDOR / 'Sparkle.framework'), str(contents))
        info = dict(CFBundleIdentifier='com.zain.mdow.updater-test', CFBundleName='Mdow Updater Test',
            CFBundleExecutable='Host', CFBundlePackageType='APPL', CFBundleVersion=str(version),
            CFBundleShortVersionString=f'0.0.{version}', SUFeedURL=base_url + '/appcast.xml',
            SUPublicEDKey=public, SUEnableAutomaticChecks=False, SUAutomaticallyUpdate=False,
            NSAppTransportSecurity={'NSAllowsLocalNetworking': True}, MdowTestMarker=str(marker))
        (contents / 'Info.plist').write_bytes(plistlib.dumps(info))
        run('codesign', '--force', '--sign', '-', str(app))
        return app

    update = make_app(work / 'update', 2)
    archive = work / 'update.zip'
    run('ditto', '-c', '-k', '--keepParent', str(update), str(archive))
    signature = run(str(VENDOR / 'bin/sign_update'), '--ed-key-file', str(work / 'seed'), '-p', str(archive))

    def feed(version=2, signature_value=signature):
        (work / 'appcast.xml').write_text(f'''<?xml version="1.0"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"><channel>
<title>Isolated updater test</title><item><title>Test {version}</title>
<enclosure url="{base_url}/update.zip" sparkle:version="{version}" sparkle:shortVersionString="0.0.{version}"
length="{archive.stat().st_size}" type="application/octet-stream" sparkle:edSignature="{signature_value}" />
</item></channel></rss>''')

    try:
        for mode in ['current', 'failure', 'download', 'install']:
            app = make_app(work / mode, 1)
            feed(1 if mode == 'current' else 2,
                 base64.b64encode(bytes(64)).decode() if mode == 'failure' else signature)
            env = dict(os.environ, MDOW_TEST_MODE=mode)
            result = subprocess.run([str(app / 'Contents/MacOS/Host')], env=env,
                                    capture_output=True, text=True, timeout=55)
            if result.returncode:
                raise RuntimeError(f'{mode} failed ({result.returncode}):\n{result.stdout}\n{result.stderr}')
            if mode == 'install':
                deadline = time.monotonic() + 30
                while not marker.exists() and time.monotonic() < deadline:
                    time.sleep(0.1)
                assert marker.read_text() == 'installed-and-relaunched'
                info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
                assert info['CFBundleVersion'] == '2', info
            print(f'PASS: native updater {mode}', flush=True)
    finally:
        server.shutdown()
