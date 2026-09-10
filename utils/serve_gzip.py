#!/usr/bin/env python3
"""Static server with gzip for text assets, so local Lighthouse runs see
transfer sizes comparable to the CDN. usage: gzserver.py <dir> <port>"""
import sys, os, gzip, io, mimetypes
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
mimetypes.add_type('application/javascript', '.mjs'); mimetypes.add_type('font/woff2', '.woff2')
class H(SimpleHTTPRequestHandler):
    def __init__(self,*a,**k): super().__init__(*a,directory=sys.argv[1],**k)
    def log_message(self,*a): pass
    def end_headers(self):
        self.send_header('Cache-Control','public, max-age=604800'); super().end_headers()
    def send_head(self):
        path=self.translate_path(self.path)
        if os.path.isdir(path):
            if not self.path.endswith('/'):
                self.send_response(301); self.send_header('Location',self.path+'/'); self.end_headers(); return None
            path=os.path.join(path,'index.html')
        if not os.path.isfile(path):
            self.send_error(404); return None
        ctype=self.guess_type(path)
        data=open(path,'rb').read()
        gz = 'gzip' in self.headers.get('Accept-Encoding','') and (ctype.startswith('text/') or 'javascript' in ctype or 'json' in ctype or 'svg' in ctype)
        if gz: data=gzip.compress(data,6)
        self.send_response(200)
        self.send_header('Content-Type',ctype)
        if gz: self.send_header('Content-Encoding','gzip'); self.send_header('Vary','Accept-Encoding')
        self.send_header('Content-Length',str(len(data)))
        self.end_headers()
        return io.BytesIO(data)
ThreadingHTTPServer(('127.0.0.1',int(sys.argv[2])),H).serve_forever()
