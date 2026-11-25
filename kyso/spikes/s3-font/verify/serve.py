"""Spike S3: máy chủ tĩnh tạm thời trên 127.0.0.1 để Edge nạp được module pdf.js.

Chỉ dùng trong lúc chụp ảnh kiểm tra của spike; không liên quan tới sản phẩm (sản phẩm không mở cổng nào).
Cách dùng: python serve.py <thư mục> <cổng>
"""

import functools
import http.server
import sys

handler = http.server.SimpleHTTPRequestHandler
# Windows có thể trả sai kiểu cho .mjs (lấy từ registry), khiến trình duyệt từ chối nạp module.
handler.extensions_map.update(
    {".mjs": "text/javascript", ".js": "text/javascript", ".wasm": "application/wasm"}
)
server = http.server.ThreadingHTTPServer(
    ("127.0.0.1", int(sys.argv[2])), functools.partial(handler, directory=sys.argv[1])
)
server.serve_forever()
