pub const CDN_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 3\r\n\
\r\n\
cdn";
pub const OK_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: no-cache\r\n\
Connection: close\r\n\
Content-Type: text/plain\r\n\
Content-Length: 3\r\n\
\r\n\
ok\n";
pub const FAVICON_SVG_RESPONSE: &[u8] = b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Type: image/svg+xml\r\n\
Content-Length: 209\r\n\
\r\n\
<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\"><path fill=\"#96c\" d=\"M0 10v1h1v1h1v1h1v1h4v-1h1v-1h1v-1h2v1h1v1h1v1h4v-1h1v-1h1v-1h5v-1H0zm5 2v1H4v-1H3v-1h1v1h1zm10 0v1h-1v-1h-1v-1h1v1h1z\"/></svg>\
\n";
pub const FAVICON_ICO_RESPONSE: &[u8] = const_str::concat_bytes!(
    b"HTTP/1.1 200 OK\r\n\
Cache-Control: immutable\r\n\
Connection: close\r\n\
Content-Encoding: gzip\r\n\
Content-Type: image/x-icon\r\n\
Content-Length: 14846\r\n\
",
    include_bytes!("fav.ico.gz")
);
