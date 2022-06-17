use std::process;
use wasm_bindgen::prelude::*;
use brotli_decompressor::BrotliDecompress;


// #[wasm_bindgen]
// extern {
//     #[wasm_bindgen(js_namespace = console)]
//     fn log(s: &str);
// }

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 8);
    match BrotliDecompress(&mut &*data, &mut out) {
        Ok(_) => {},
        Err(_) => process::abort()
    };
    out
}
