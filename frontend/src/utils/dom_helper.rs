use wasm_bindgen::JsCast; 
use web_sys::{HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement}; 

pub fn get_input_value(id: &str) -> Option<String> { 
    web_sys::window() 
        .and_then(|w| w.document()) 
        .and_then(|d| d.get_element_by_id(id)) 
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) 
        .map(|input| input.value()) 
} 

pub fn get_select_value(id: &str) -> Option<String> { 
    web_sys::window() 
        .and_then(|w| w.document()) 
        .and_then(|d| d.get_element_by_id(id)) 
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok()) 
        .map(|select| select.value()) 
} 

pub fn get_textarea_value(id: &str) -> Option<String> { 
    web_sys::window() 
        .and_then(|w| w.document()) 
        .and_then(|d| d.get_element_by_id(id)) 
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok()) 
        .map(|textarea| textarea.value()) 
} 

pub fn get_numeric_value(id: &str) -> Option<f64> { 
    get_input_value(id).and_then(|v| v.parse().ok()) 
} 
