use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    // 假设 html2pdf 或者 jsPDF 已经通过 <script> 标签或构建系统注入到全局
    #[wasm_bindgen(js_name = "html2pdf")]
    pub fn html2pdf() -> JsValue;
}

pub fn export_to_pdf(element_id: &str, filename: &str) {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    
    if let Some(element) = document.get_element_by_id(element_id) {
        // 调用 JS 脚本执行 html2pdf.bundle.js 或者 jspdf 逻辑
        // 这里提供一种利用原生 eval 执行第三方库的实现，前提是页面引入了库
        let js_code = format!(
            r#"
            if (typeof html2pdf !== 'undefined') {{
                var element = document.getElementById('{}');
                var opt = {{
                    margin:       10,
                    filename:     '{}',
                    image:        {{ type: 'jpeg', quality: 0.98 }},
                    html2canvas:  {{ scale: 2 }},
                    jsPDF:        {{ unit: 'mm', format: 'a4', orientation: 'portrait' }}
                }};
                html2pdf().set(opt).from(element).save();
            }} else {{
                console.warn('PDF export requires html2pdf.js to be loaded in the page.');
                alert('请在 index.html 中引入 html2pdf.js');
            }}
            "#,
            element_id, filename
        );
        let _ = js_sys::eval(&js_code);
    }
}
