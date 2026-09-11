"""Exercise the actual read-service executable on an ephemeral loopback port."""
import json, subprocess, sys, urllib.request, urllib.error, pathlib, tempfile
binary=sys.argv[1]
with tempfile.TemporaryDirectory() as temp:
    path=pathlib.Path(temp)/'catalogue.json'
    snapshot=json.loads(pathlib.Path('data/registry.json').read_text())
    path.write_text(json.dumps(snapshot))
    p=subprocess.Popen([binary,str(path),'127.0.0.1:0'],stdout=subprocess.PIPE,text=True)
    try:
        origin='http://'+p.stdout.readline().strip()
        with urllib.request.urlopen(origin+'/api/v1/catalogue',timeout=5) as r:
            assert json.load(r)==snapshot
            etag=r.headers['ETag']
        req=urllib.request.Request(origin+'/api/v1/catalogue',headers={'If-None-Match':etag})
        try: urllib.request.urlopen(req,timeout=5); raise AssertionError('expected 304')
        except urllib.error.HTTPError as e: assert e.code==304
        for url,code in [('/api/v1/apps?limit=0',400),('/api/v1/apps/missing',404)]:
            try: urllib.request.urlopen(origin+url,timeout=5); raise AssertionError('expected error')
            except urllib.error.HTTPError as e: assert e.code==code
        snapshot['revision']='changed'
        path.write_text(json.dumps(snapshot))
        with urllib.request.urlopen(origin+'/api/v1/catalogue',timeout=5) as r: assert r.headers['ETag']!=etag
        print('HTTP entrypoint: catalogue, ETag, invalid queries, not found and reload passed')
    finally: p.terminate();p.wait(timeout=5)
