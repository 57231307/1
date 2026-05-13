//! URL 编码工具
//! 使用 js_sys::encode_uri_component 替代 percent-encoding 库

use wasm_bindgen::prelude::*;

/// 对字符串进行 URL 编码
pub fn encode_uri_component(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}
