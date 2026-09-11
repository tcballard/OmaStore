"""Actual Qt keyboard checks and a second-process preferences recovery check."""
import os, pathlib, subprocess, sys, tempfile
with tempfile.TemporaryDirectory() as temp:
    env=dict(os.environ, QT_QPA_PLATFORM='offscreen', QT_QUICK_BACKEND='software',
             XDG_CONFIG_HOME=temp+'/config',XDG_CACHE_HOME=temp+'/cache',
             OMASTORE_CATALOGUE_URL='http://127.0.0.1/disabled')
    subprocess.run([sys.argv[1],'--interaction-test'],env=env,check=True,timeout=20)
    subprocess.run([sys.argv[1],'--expect-search','localsend','--smoke-test'],env=env,check=True,timeout=10)
    print('Separate process restored search preferences.')
