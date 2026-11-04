"""Opt-in real process lifecycle test; uses fixture engines and isolated game data."""
import json
import os
from pathlib import Path
import socket
import sqlite3
import subprocess
import sys
import tempfile
import time
import urllib.request

root = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(root))
from scripts.app import Manager  # noqa: E402


with tempfile.TemporaryDirectory(prefix='weiqi-launcher-smoke-') as temporary:
    data = Path(temporary)
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
    env = {**os.environ, 'WEIQI_PORT': str(port), 'WEIQI_DATA_DIR': str(data)}
    manager = Manager(root, env)
    with sqlite3.connect(data / 'trainer.sqlite3') as conn:
        conn.executescript((root / 'migrations/001_initial.sql').read_text())
        settings = dict(executable=str(root / 'tests/fake_katago.py'),
                        human_model=str(root / 'tests/fixtures/human-5k.json'),
                        analysis_model=str(root / 'tests/fixtures/strong.json'),
                        config=str(root / 'config/analysis.cfg'))
        conn.execute('INSERT INTO settings VALUES(1,?)', (json.dumps(settings),))

    def control(action):
        subprocess.run([sys.executable, str(root / 'scripts/app.py'), action],
                       cwd=root, env=env, check=True, timeout=120)

    def request(path, body=None):
        command = urllib.request.Request(manager.url + '/api' + path,
                                         data=None if body is None else json.dumps(body).encode(),
                                         headers={'Content-Type': 'application/json'})
        with urllib.request.urlopen(command, timeout=5) as response:
            return json.load(response)

    def ready():
        for _ in range(100):
            if len(request('/state')['engine']['human']['supported_ranks']) == 29:
                return
            time.sleep(.1)
        raise RuntimeError('Fixture engine did not start')

    try:
        control('update')
        ready()
        first_pid = manager.owned_pid()
        control('start')
        assert manager.owned_pid() == first_pid, 'Reopening started a duplicate server'
        game = request('/games', dict(mode='manual', rank=-5, color='B'))['id']
        view = request('/games/' + game)
        root_node = view['game']['root']
        request('/games/' + game + '/moves', dict(parent=root_node, point=60, review=False,
                                                expected_head=root_node))
        for _ in range(100):
            view = request('/games/' + game)
            if len(view['game']['nodes']) == 3 and not view['thinking']:
                break
            time.sleep(.1)
        assert len(view['game']['nodes']) == 3
        request('/games/' + game + '/moves', dict(parent=root_node, point=61, review=True))
        head = view['game']['active_head']
        request('/games/' + game + '/resign', dict(head=head))
        before = request('/games/' + game)['game']
        audit = request('/games/' + game + '/audit')
        control('update')
        ready()
        assert manager.owned_pid() != first_pid, 'Update did not replace the process'
        after = request('/games/' + game)['game']
        for key in ['nodes', 'active_head', 'result', 'rank', 'komi', 'initial_stones']:
            assert before[key] == after[key], f'Update changed {key}'
        assert request('/games/' + game + '/audit') == audit
        backup = sorted((data / 'backups').glob('*.sqlite3'))[-1]
        with sqlite3.connect(backup) as saved:
            assert saved.execute('SELECT count(*) FROM nodes').fetchone()[0] == 4
            assert saved.execute('PRAGMA integrity_check').fetchone()[0] == 'ok'
        print('PASS: start/reopen/update preserve played moves, review branches, result and audit; backup verified.')
    finally:
        control('stop')
    assert manager.owned_pid() is None, 'Stop left a server listening'
