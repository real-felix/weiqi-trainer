import io
import json
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.app import Manager


class LauncherTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix='weiqi-launcher-')
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.manager = Manager(self.root, {'WEIQI_DATA_DIR': str(self.root / 'data')})
        self.manager.runtime.mkdir(parents=True)

    def test_failed_build_does_not_stop_or_replace_the_running_release(self):
        old = self.manager.releases / 'old'
        old.mkdir(parents=True)
        self.manager.activate(old)
        with patch.object(self.manager, 'owned_pid', return_value=123), \
                patch.object(self.manager, 'state'), \
                patch.object(self.manager, 'build', side_effect=RuntimeError('Build failed')), \
                patch.object(self.manager, 'backup') as backup, \
                patch.object(self.manager, 'stop') as stop:
            with self.assertRaisesRegex(RuntimeError, 'Build failed'):
                self.manager.update()
            stop.assert_not_called()
            backup.assert_not_called()
            self.assertEqual(self.manager.current.resolve(), old)

    def test_backup_includes_committed_wal_and_leaves_the_source_unchanged(self):
        self.manager.data.mkdir()
        with sqlite3.connect(self.manager.data / 'trainer.sqlite3') as source:
            source.execute('PRAGMA journal_mode=WAL')
            source.execute('CREATE TABLE games(id TEXT, result TEXT)')
            source.execute("INSERT INTO games VALUES('game', 'W+R')")
            source.commit()
            backup = self.manager.backup()
            with sqlite3.connect(backup) as saved:
                self.assertEqual(saved.execute('SELECT * FROM games').fetchall(), [('game', 'W+R')])
            self.assertEqual(source.execute('SELECT * FROM games').fetchall(), [('game', 'W+R')])

    def test_an_unrelated_listener_is_never_stopped(self):
        responses = [
            subprocess.CompletedProcess([], 0, stdout='123\n', stderr=''),
            subprocess.CompletedProcess([], 0, stdout='/usr/bin/python3 -m http.server\n'),
            subprocess.CompletedProcess([], 0, stdout=f'p123\nn{self.root}\n'),
        ]
        with patch('scripts.app.subprocess.run', side_effect=responses), \
                patch('scripts.app.os.kill') as kill:
            with self.assertRaisesRegex(RuntimeError, 'another program or checkout'):
                self.manager.stop()
            kill.assert_not_called()

    def test_refuses_to_manage_a_different_data_directory(self):
        response = io.BytesIO(json.dumps({'data_dir': str(self.root / 'other-data')}).encode())
        with patch('scripts.app.urllib.request.urlopen', return_value=response):
            with self.assertRaisesRegex(RuntimeError, 'another data directory'):
                self.manager.state()

    def test_stop_uses_graceful_interrupt(self):
        with patch.object(self.manager, 'owned_pid', side_effect=[123, None]), \
                patch.object(self.manager, 'state'), patch('scripts.app.os.kill') as kill:
            self.manager.stop()
            kill.assert_called_once_with(123, signal.SIGINT)


if __name__ == '__main__':
    unittest.main()
