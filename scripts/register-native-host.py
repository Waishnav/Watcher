#!/usr/bin/env python3
"""Opt-in developer registration of Watcher's Native Messaging host.

Usage: python3 scripts/register-native-host.py /absolute/path/to/watcher-native-host
       [chrome-extension-id]

This is *per-user* registration, never machine-wide. Uninstall by removing
only the generated dev.watcher.bridge.json files.
"""
import json
import os
import pathlib
import re
import sys

HOST = 'dev.watcher.bridge'
FIREFOX_ID = 'watcher@watcher.local'


def main():
    if len(sys.argv) not in (2, 3):
        sys.exit('Usage: register-native-host.py /absolute/native-host [chrome-extension-id]')
    executable = pathlib.Path(sys.argv[1]).expanduser().resolve(strict=True)
    if not executable.is_file() or not os.access(executable, os.X_OK):
        sys.exit('Executable native host is required')
    chrome_id = sys.argv[2] if len(sys.argv) == 3 else None
    if chrome_id is not None and not re.fullmatch(r'[a-p]{32}', chrome_id):
        sys.exit('Chrome extension ID must be 32 characters from a through p')
    home = pathlib.Path.home()
    if sys.platform == 'linux':
        roots = [home / '.mozilla/native-messaging-hosts']
        if chrome_id:
            roots.extend([home / '.config/google-chrome/NativeMessagingHosts',
                          home / '.config/chromium/NativeMessagingHosts'])
    elif sys.platform == 'darwin':
        roots = [home / 'Library/Application Support/Mozilla/NativeMessagingHosts']
        if chrome_id:
            roots.append(home / 'Library/Application Support/Google/Chrome/NativeMessagingHosts')
    else:
        sys.exit('For Windows, register the host manifest in HKCU NativeMessagingHosts. Windows packaging is pending.')
    for root in roots:
        root.mkdir(parents=True, exist_ok=True, mode=0o700)
        firefox = 'mozilla' in str(root).lower()
        payload = {'name': HOST, 'description': 'Watcher local-only browser bridge',
                   'type': 'stdio', 'path': str(executable)}
        if firefox:
            payload['allowed_extensions'] = [FIREFOX_ID]
        else:
            payload['allowed_origins'] = [f'chrome-extension://{chrome_id}/']
        path = root / f'{HOST}.json'
        # Do not override an existing manifest pointing somewhere else.
        if path.exists():
            previous = json.loads(path.read_text())
            if previous.get('path') != str(executable):
                sys.exit(f'Refusing to overwrite native host registration: {path}')
        with path.open('w', encoding='utf-8') as output:
            json.dump(payload, output, indent=2)
            output.write('\n')
        path.chmod(0o600)
        print(f'Registered {path}')
    print('Restart the browser to discover the native host. No daemon was started.')


if __name__ == '__main__':
    main()
