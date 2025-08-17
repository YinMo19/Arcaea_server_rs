from http.server import BaseHTTPRequestHandler, HTTPServer
import socketserver


class RequestHandler(BaseHTTPRequestHandler):
    def _set_headers(self):
        self.send_response(200)
        self.send_header("Content-type", "text/plain")
        self.end_headers()

    def do_GET(self):
        print("\n===== GET REQUEST ======")
        print(f"Path: {self.path}")
        print("Headers:")
        for header, value in self.headers.items():
            print(f"  {header}: {value}")
        self._set_headers()
        self.wfile.write(b"GET request received")

    def do_POST(self):
        content_length = int(self.headers.get("Content-Length", 0))
        post_data = self.rfile.read(content_length)

        print("\n===== POST REQUEST ======")
        print(f"Path: {self.path}")
        print("Headers:")
        for header, value in self.headers.items():
            print(f"  {header}: {value}")
        print(f"Body:\n{post_data.decode('utf-8')}")

        self._set_headers()
        self.wfile.write(b"POST request received")


def run(server_class=HTTPServer, handler_class=RequestHandler, port=8090):
    server_address = ("", port)
    httpd = server_class(server_address, handler_class)
    print(f"Starting server on port {port}...")
    httpd.serve_forever()


if __name__ == "__main__":
    run()
