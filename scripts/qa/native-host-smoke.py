#!/usr/bin/env python3
"""Native Messaging -> daemon -> SQLite smoke, with disposable profile only."""
import json
import os
import pathlib
import sqlite3
import struct
import subprocess
import tempfile
import time

ROOT = pathlib.Path(__file__).resolve().parents[2]
HOST = ROOT / 'target/debug/watcher-native-host'
DAEMON = ROOT / 'target/debug/watcherd'
CLI = ROOT / 'target/debug/watcher'


def send(message, env):
    value = json.dumps(message).encode()
    framed = struct.pack('=I', len(value)) + value
    result = subprocess.run([str(HOST), '--dev'], input=framed, capture_output=True,
                            check=True, env=env, timeout=6)
    assert len(result.stdout) >= 4, result.stderr
    size = struct.unpack('=I', result.stdout[:4])[0]
    assert len(result.stdout[4:]) == size, 'native message framing broken'
    return json.loads(result.stdout[4:])


with tempfile.TemporaryDirectory(prefix='watcher-p3-') as home:
    env = dict(os.environ, WATCHER_DEV_HOME=home)
    now = int(time.time() * 1000)
    sample = dict(source_id='qa-profile-1', seq=1, start_ms=now - 5000,
                  end_ms=now - 1000, domain='example.org')
    request = dict(op='browser_batch', version=1, intervals=[sample])
    assert send(request, env)['message'] == 'daemon_unavailable'
    with open(os.devnull, 'w') as sink:
        daemon = subprocess.Popen([str(DAEMON), '--dev'], env=env, stdout=sink, stderr=sink)
        try:
            socket = pathlib.Path(home) / 'daemon.sock'
            for _ in range(80):
                if socket.exists():
                    break
                assert daemon.poll() is None, 'daemon failed to start'
                time.sleep(.1)
            assert socket.exists(), 'daemon socket did not appear'
            for _ in range(2):
                response = send(request, env)
                assert response == {'kind': 'browser_ack', 'version': 1, 'received': 1}, response
            report = send(dict(op='browser_report', version=1, from_ms=now-10_000,
                               to_ms=now+10_000), env)
            assert report['kind'] == 'browser_report', report
            assert report['domains'] == [{'domain': 'example.org', 'duration_ms': 4000}], report
            forbidden = send(dict(op='shutdown', version=1), env)
            assert forbidden['kind'] == 'error' and 'unavailable' in forbidden['message']
            conn = sqlite3.connect(pathlib.Path(home) / 'watcher.sqlite')
            assert conn.execute('PRAGMA user_version').fetchone()[0] == 3
            assert conn.execute('SELECT COUNT(*) FROM browser_intervals').fetchone()[0] == 1
            conn.close()
            print('PASS: offline response, native framing, durable import, retry idempotence, reporting, forbidden command')
        finally:
            subprocess.run([str(CLI), '--dev', 'shutdown'], env=env,
                           capture_output=True, timeout=5, check=False)
            try:
                daemon.wait(timeout=8)
            except subprocess.TimeoutExpired:
                daemon.terminate()
                daemon.wait(timeout=3)
