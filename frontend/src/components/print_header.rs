use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PrintHeaderProps {
    pub title: String,
    pub doc_no: String,
    pub date: String,
}

#[function_component(PrintHeader)]
pub fn print_header(props: &PrintHeaderProps) -> Html {
    html! {
        <div class="print-header" style="display: none;">
            <style>
            {r#"
            @media print {
                .print-header { display: block !important; }
            }
            "#}
            </style>
            <h2>{ &props.title }</h2>
            <div class="doc-info">
                <span>{ format!("单据号: {}", props.doc_no) }</span>
                <span style="margin-left: 20px;">{ format!("日期: {}", props.date) }</span>
            </div>
            <hr style="margin-bottom: 20px; border-top: 1px solid #333;" />
        </div>
    }
}
