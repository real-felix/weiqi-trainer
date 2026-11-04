#!/usr/bin/env python3
"""Build and manage this checkout's local app, without a terminal staying open."""
import argparse
from contextlib import contextmanager
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import plistlib
import shutil
import signal
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.error
import urllib.request
import uuid


class Manager:
    def __init__(self, root, env=None):
        self.root = Path(root).resolve()
        self.env = dict(os.environ if env is None else env)
        self.env['PATH'] = os.pathsep.join([
            str(Path.home() / '.cargo/bin'), '/opt/homebrew/bin', '/usr/local/bin',
            self.env.get('PATH', ''), '/usr/bin', '/bin', '/usr/sbin', '/sbin',
        ])
        self.port = int(self.env.get('WEIQI_PORT', '43187'))
        if not 1 <= self.port <= 65535:
            raise ValueError('WEIQI_PORT must be between 1 and 65535')
        default_data = Path.home() / ('Library/Application Support/Weiqi Trainer'
                                    if sys.platform == 'darwin' else '.local/share/weiqi-trainer')
        self.data = Path(self.env.get('WEIQI_DATA_DIR', str(default_data))).expanduser().resolve()
        self.runtime = self.root / '.local/app'
        self.releases = self.runtime / 'releases'
        self.current = self.runtime / 'current'
        self.log = self.runtime / 'server.log'
        self.url = f'http://127.0.0.1:{self.port}'

    @contextmanager
    def lock(self):
        self.runtime.mkdir(parents=True, exist_ok=True)
        with (self.runtime / 'control.lock').open('a') as handle:
            fcntl.flock(handle, fcntl.LOCK_EX)
            yield

    def command(self, args, **kwargs):
        return subprocess.run(args, cwd=self.root, env=self.env, check=True, **kwargs)

    def owned_pid(self):
        result = subprocess.run(
            ['lsof', '-nP', '-t', f'-iTCP:{self.port}', '-sTCP:LISTEN'],
            env=self.env, text=True, capture_output=True, check=False,
        )
        if result.returncode == 1 and not result.stdout.strip() and not result.stderr.strip():
            return None
        if result.returncode != 0:
            raise RuntimeError(f'Cannot inspect port {self.port}: {result.stderr.strip()}')
        pids = set(result.stdout.split())
        if len(pids) != 1:
            raise RuntimeError(f'Port {self.port} has multiple listeners; leaving them untouched.')
        pid = int(pids.pop())
        command = self.command(['ps', '-p', str(pid), '-o', 'command='],
                               capture_output=True, text=True).stdout.strip()
        executable = Path(command)
        legacy = command in (str(self.root / 'target/release/weiqi-trainer'),
                             'target/release/weiqi-trainer')
        managed = (executable.name == 'weiqi-trainer' and
                   executable.parent.parent == self.releases)
        cwd = self.command(['lsof', '-a', '-p', str(pid), '-d', 'cwd', '-Fn'],
                           capture_output=True, text=True).stdout.splitlines()
        if not (legacy or managed) or f'n{self.root}' not in cwd:
            raise RuntimeError(f'Port {self.port} belongs to another program or checkout; '
                               'leaving it untouched. Choose another WEIQI_PORT.')
        return pid

    def state(self):
        with urllib.request.urlopen(self.url + '/api/state', timeout=2) as response:
            state = json.load(response)
        if Path(state['data_dir']).resolve() != self.data:
            raise RuntimeError('The running app uses another data directory; '
                               'set WEIQI_DATA_DIR to match before managing it.')
        return state

    def build(self):
        for tool in ['npm', 'cargo']:
            if not shutil.which(tool, path=self.env['PATH']):
                raise RuntimeError(f'{tool} was not found. Install the build tools described in README.md.')
        if not (self.root / 'node_modules').is_dir():
            raise RuntimeError('Install frontend dependencies first: npm ci')
        self.releases.mkdir(parents=True, exist_ok=True)
        stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ') + '-' + uuid.uuid4().hex[:8]
        release = self.releases / stamp
        # Build away from the running UI. A failed build leaves its release intact.
        with tempfile.TemporaryDirectory(prefix='.build-', dir=self.releases) as temporary:
            stage = Path(temporary)
            self.command(['npm', 'run', 'build', '--', '--outDir', str(stage / 'dist')])
            self.command(['cargo', 'build', '--offline', '--locked', '--release',
                          '--target-dir', str(self.root / 'target')])
            shutil.copy2(self.root / 'target/release/weiqi-trainer', stage / 'weiqi-trainer')
            stage.rename(release)
        return release

    def activate(self, release):
        pending = self.runtime / 'next'
        pending.unlink(missing_ok=True)
        pending.symlink_to(release)
        pending.replace(self.current)

    def backup(self):
        database = self.data / 'trainer.sqlite3'
        if not database.exists():
            return None
        folder = self.data / 'backups'
        folder.mkdir(parents=True, exist_ok=True)
        stamp = datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%S%fZ')
        backup = folder / f'before-update-{stamp}.sqlite3'
        with sqlite3.connect(database.as_uri() + '?mode=ro', uri=True) as source:
            with sqlite3.connect(backup) as destination:
                source.backup(destination)
                if destination.execute('PRAGMA integrity_check').fetchone()[0] != 'ok':
                    raise RuntimeError('Database backup failed its integrity check; update cancelled.')
        print(f'Database backup: {backup}', flush=True)
        return backup

    def start(self):
        pid = self.owned_pid()
        if pid:
            self.state()
            return pid
        if not (self.current / 'weiqi-trainer').is_file():
            self.backup()
            self.activate(self.build())
        self.data.mkdir(parents=True, exist_ok=True)
        env = {**self.env, 'WEIQI_PORT': str(self.port), 'WEIQI_DATA_DIR': str(self.data),
               'WEIQI_DIST_DIR': str(self.current.resolve() / 'dist')}
        with self.log.open('ab') as log:
            child = subprocess.Popen([str(self.current.resolve() / 'weiqi-trainer')],
                                     cwd=self.root, env=env, stdin=subprocess.DEVNULL,
                                     stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        deadline = time.monotonic() + 20
        while time.monotonic() < deadline:
            if child.poll() is not None:
                raise RuntimeError(f'The app stopped during startup. See {self.log}')
            try:
                self.state()
                return child.pid
            except (OSError, ValueError, urllib.error.URLError):
                time.sleep(.1)
        child.send_signal(signal.SIGINT)
        raise RuntimeError(f'The app did not become available. See {self.log}')

    def stop(self):
        pid = self.owned_pid()
        if not pid:
            return
        self.state()
        os.kill(pid, signal.SIGINT)
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            if self.owned_pid() is None:
                return
            time.sleep(.1)
        raise RuntimeError('The app is still shutting down; no force kill was attempted. '
                           f'See {self.log}')

    def update(self):
        if self.owned_pid():
            self.state()
        release = self.build()
        self.backup()
        self.stop()
        self.activate(release)
        return self.start()

    def install(self):
        if sys.platform != 'darwin':
            raise RuntimeError('The Finder launcher is for macOS. Use the start/open commands here.')
        app = self.root / 'Weiqi Trainer.app'
        contents = app / 'Contents'
        (contents / 'MacOS').mkdir(parents=True, exist_ok=True)
        with (contents / 'Info.plist').open('wb') as handle:
            plistlib.dump({'CFBundleName': 'Weiqi Trainer', 'CFBundleDisplayName': 'Weiqi Trainer',
                          'CFBundleIdentifier': 'local.weiqi-trainer.launcher',
                          'CFBundleExecutable': 'Weiqi Trainer', 'CFBundlePackageType': 'APPL',
                          'CFBundleVersion': '1', 'LSUIElement': True}, handle)
        executable = contents / 'MacOS/Weiqi Trainer'
        executable.write_text('''#!/bin/sh
set -eu
export PATH="$HOME/.cargo/bin:/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin:$PATH"
cd "$(dirname "$0")/../../.."
mkdir -p .local/app
exec python3 scripts/app.py open --gui >> .local/app/launcher.log 2>&1
''')
        executable.chmod(0o755)
        print(f'Launcher installed: {app}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['install', 'open', 'start', 'stop', 'status', 'update'])
    parser.add_argument('--gui', action='store_true', help=argparse.SUPPRESS)
    args = parser.parse_args()
    try:
        manager = Manager(Path(__file__).resolve().parent.parent)
        with manager.lock():
            if args.action == 'install':
                manager.install()
                return
            if args.action == 'stop':
                manager.stop()
                print('Weiqi Trainer stopped.')
                return
            if args.action == 'status':
                pid = manager.owned_pid()
                if pid:
                    manager.state()
                print(f'{"Running (PID " + str(pid) + ")" if pid else "Stopped"}: {manager.url}')
                print(f'Data: {manager.data}\nServer log: {manager.log}')
                return
            pid = manager.update() if args.action == 'update' else manager.start()
            print(f'Weiqi Trainer is running (PID {pid}): {manager.url}', flush=True)
        if args.action == 'open':
            if sys.platform == 'darwin':
                manager.command(['open', manager.url])
            else:
                import webbrowser
                webbrowser.open(manager.url)
    except (OSError, ValueError, RuntimeError, sqlite3.Error, subprocess.CalledProcessError) as error:
        message = str(error)
        print(message, file=sys.stderr)
        if args.gui and sys.platform == 'darwin':
            subprocess.run(['osascript', '-e', 'on run argv\n'
                            'display alert "Could not open Weiqi Trainer" message (item 1 of argv) as critical\n'
                            'end run', message], check=False)
        sys.exit(1)


if __name__ == '__main__':
    main()
