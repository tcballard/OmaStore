#!/usr/bin/env python3
"""Controlled protocol peer for session tests; never runs a package command."""
import json
import os
from pathlib import Path
import sys
import time

state = Path(os.environ['OMASTORE_BRIDGE_TEST_STATE'])
for line in sys.stdin:
    req = json.loads(line)
    mode = state.read_text().strip()
    method, params = req['method'], req['params']
    with state.with_suffix('.requests').open('a') as log:
        log.write(method + '\n')
    result, error = {}, None
    if method == 'core.info':
        result = {'service': 'omastore-core', 'version': 'test'}
    elif method in ('catalogue.info', 'catalogue.refresh'):
        result = {'snapshot': 'catalogue', 'appCount': 2}
    elif method == 'apps.list':
        result = {'items': [], 'total': 0}
    elif method == 'library.inventory':
        offset = params.get('offset', 0)
        if mode == 'crash':
            sys.exit(0)
        if mode == 'offline' or (mode == 'changed' and offset):
            error = 'snapshot_changed' if mode == 'changed' else 'local_database_unavailable'
        else:
            if offset:
                time.sleep(.4)
            installed = mode != 'initial'
            app = {'id': 'second' if offset else 'first', 'name': 'Test app',
                   'state': 'update_available' if mode == 'updates' else 'installed' if installed else 'not_installed',
                   'package': 'test-package', 'installedVersion': '1-1' if installed else None, 'observedAt': 123,
                   'primaryAction': 'open' if installed else 'review_install'}
            result = {'snapshot': mode, 'items': [app], 'offset': offset,
                      'nextOffset': None if offset else 30, 'observationState': 'available',
                      'installedCount': 2 if installed else 0, 'observedAt': 123}
    elif method == 'library.launchers':
        result = {'id': params['id'], 'items': [] if mode == 'no_launcher' else ['test.desktop']}
    elif method == 'library.activity':
        result = {'items': [] if mode == 'initial' else [{'id': 'operation', 'state': 'failed',
                  'apps': [{'appId': 'first', 'action': 'install'}], 'cancelRequested': False}]}
    response = {'protocol_version': 1, 'id': req['id'], 'ok': error is None}
    response['error' if error else 'result'] = {'code': error} if error else result
    print(json.dumps(response), flush=True)
