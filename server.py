import json
from http.server import BaseHTTPRequestHandler, HTTPServer

class SimpleHandler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header('content-type', 'text/plain')
        self.end_headers()
        self.wfile.write(b'200 OK')

    def do_POST(self):
        print(self.headers)
        content_length = int(self.headers['content-length'])
        body = self.rfile.read(content_length)
        json_body = json.loads(body.decode('utf-8'))

        print(body)
        
        self.send_response(200)
        self.send_header('content-type', 'application/json')
        self.end_headers()
        
        response = json.dumps({'status': '200 OK', 'body': json_body})
        self.wfile.write(response.encode('utf-8'))

def run_server():
    server_address = ('', 8001)
    httpd = HTTPServer(server_address, SimpleHandler)
    print('Server running on http://localhost:8001')
    httpd.serve_forever()

run_server()