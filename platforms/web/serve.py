#!/usr/bin/env python3
"""Simple HTTP server with correct WASM MIME type for iOS Safari."""

import http.server
import socketserver
import os

PORT = 8887

class WasmHandler(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header('Access-Control-Allow-Origin', '*')
        self.send_header('Cache-Control', 'no-store, no-cache, must-revalidate, max-age=0')
        self.send_header('Pragma', 'no-cache')
        self.send_header('Expires', '0')
        super().end_headers()

    def do_GET(self):
        # Serve index.html for root URL
        if self.path == '/':
            self.path = '/index.html'
        return super().do_GET()


WasmHandler.extensions_map['.wasm'] = 'application/wasm'

class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True

if __name__ == '__main__':
    with ReusableTCPServer(("", PORT), WasmHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        print("Press Ctrl+C to stop\n")
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nStopped.")
            os._exit(0)
