pub const OK_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 3\r\n\
\r\n\
ok\n";
pub const METHOD_NOT_ALLOWED_RESPONSE: &[u8] = b"HTTP/1.1 405 Method Not Allowed\r\n\
Allow: GET\r\n\
Connection: close\r\n\
Content-Length: 0\r\n
\r\n";
pub const ROOT_REDIRECT_RESPONSE: &[u8] = b"HTTP/1.1 308 Permanent Redirect\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Location: https://cdn.packurl.net\r\n\
\r\n";
pub const CONTENT_TOO_LARGE_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 414 URI Too Long\r\n\
Cache-Control: no-store\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: text/html\r\n\
Content-Length: 2724\r\n\
\r\n",
    include_bytes!("414.html.br")
);
pub const SERVICE_WORKER_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Type: application/javascript\r\n\
Content-Length: 39\r\n\
\r\n\
import 'https://cdn.packurl.net/sw.mjs'\
\n";
pub const MANIFEST_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: application/manifest+json\r\n\
Content-Length: 862\r\n\
\r\n\
",
    include_bytes!("pwa.json.br")
);
pub const FAVICON_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Encoding: br\r\n\
Content-Type: image/x-icon\r\n\
Content-Length: 14846\r\n\
\r\n\
",
    include_bytes!("fav.ico.br")
);

pub const CDN_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 3\r\n\
\r\n\
cdn";
pub const LOCALHOST_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 6\r\n\
\r\n\
local\n";
// pub const FAVICON_SVG_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
// Cache-Control: immutable\r\n\
// Connection: close\r\n\
// Content-Type: image/svg+xml\r\n\
// Content-Length: 209\r\n\
// \r\n\
// <svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path fill=\"#96c\" d=\"M0 10v1h1v1h1v1h1v1h4v-1h1v-1h1v-1h2v1h1v1h1v1h4v-1h1v-1h1v-1h5v-1H0zm5 2v1H4v-1H3v-1h1v1h1zm10 0v1h-1v-1h-1v-1h1v1h1z\"/></svg>\
// \n";
