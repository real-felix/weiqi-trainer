"""Launch the actual HTTP service and built SPA with isolated SQLite and fixture engines."""
import json
import os
from pathlib import Path
import signal
import sqlite3
import subprocess
import tempfile

root=Path(__file__).resolve().parent.parent
with tempfile.TemporaryDirectory(prefix='weiqi-e2e-') as data:
    conn=sqlite3.connect(Path(data)/'trainer.sqlite3')
    conn.executescript((root/'migrations/001_initial.sql').read_text())
    settings=dict(executable=str(root/'tests/fake_katago.py'),human_model=str(root/'tests/fixtures/human-5k.json'),analysis_model=str(root/'tests/fixtures/strong.json'),config=str(root/'config/analysis.cfg'))
    conn.execute('INSERT INTO settings VALUES(1,?)',(json.dumps(settings),));conn.commit();conn.close()
    process=subprocess.Popen(['cargo','run','--offline','--quiet'],cwd=root,env={**os.environ,'WEIQI_DATA_DIR':data,'WEIQI_PORT':'43127'})
    def stop(*_):
        process.send_signal(signal.SIGINT)
        process.wait(timeout=15)
    signal.signal(signal.SIGTERM,stop)
    try:process.wait()
    except KeyboardInterrupt:stop()
