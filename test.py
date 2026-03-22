import http.server
import socketserver
import logging

# Configure logging
logging.basicConfig(level=logging.INFO)

class HeaderLoggingHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        # Log all headers
        logging.info(f"Headers:\n{self.headers}")
        
        # Respond to client
        self.send_response(200)
        self.end_headers()
        self.wfile.write(b"Headers logged")

    def do_POST(self):
        self.do_GET()

# Run server on port 8080
PORT = 8080
with socketserver.TCPServer(("0.0.0.0", PORT), HeaderLoggingHandler) as httpd:
    print(f"Serving on port {PORT}")
    httpd.serve_forever()
