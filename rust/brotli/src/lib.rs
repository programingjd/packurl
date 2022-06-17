use std::process;
use wasm_bindgen::prelude::*;
use brotli::enc::BrotliEncoderParams;
use brotli::BrotliCompress;
use brotli::BrotliDecompress;
use brotli::enc::backward_references::BrotliEncoderMode;


// #[wasm_bindgen]
// extern {
//     #[wasm_bindgen(js_namespace = console)]
//     fn log(s: &str);
// }

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn compress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 64);
    let mut params= BrotliEncoderParams::default();
    params.mode = BrotliEncoderMode::BROTLI_MODE_TEXT;
    match BrotliCompress(&mut &*data, &mut out, &params) {
        Ok(_) => {}
        Err(_) => process::abort()
    }
    out
}

#[wasm_bindgen]
pub fn decompress(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() * 8);
    match BrotliDecompress(&mut &*data, &mut out) {
        Ok(_) => {}
        Err(_) => process::abort()
    }
    out
}
