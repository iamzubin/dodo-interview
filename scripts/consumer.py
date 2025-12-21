import http.server
import socketserver
import json
import sys

class RequestHandler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers['Content-Length'])
        post_data = self.rfile.read(content_length)
        print(f"Received webhook: {post_data.decode('utf-8')}")
        sys.stdout.flush()
        
        self.send_response(200)
        self.send_header('Content-type', 'application/json')
        self.end_headers()
        self.wfile.write(json.dumps({"status": "received"}).encode('utf-8'))

    def do_GET(self):
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"Webhook consumer is running")

PORT = 8000
Handler = RequestHandler

with socketserver.TCPServer(("", PORT), Handler) as httpd:
    print(f"Serving at port {PORT}")
    sys.stdout.flush()
    httpd.serve_forever()
