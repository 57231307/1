use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct PaginationProps {
    pub current_page: u64,
    pub page_size: u64,
    pub total: u64,
    pub on_page_change: Callback<u64>,
}

#[function_component(Pagination)]
pub fn pagination(props: &PaginationProps) -> Html {
    let total_pages = if props.total == 0 {
        1
    } else {
        ((props.total + props.page_size - 1) / props.page_size).max(1)
    };

    let current_page = props.current_page;
    let on_page_change = props.on_page_change.clone();

    let on_prev = {
        let on_page_change = on_page_change.clone();
        Callback::from(move |_| {
            if current_page > 0 {
                on_page_change.emit(current_page - 1);
            }
        })
    };

    let on_next = {
        let on_page_change = on_page_change.clone();
        Callback::from(move |_| {
            if current_page + 1 < total_pages {
                on_page_change.emit(current_page + 1);
            }
        })
    };

    let start_item = if props.total == 0 {
        0
    } else {
        current_page * props.page_size + 1
    };
    let end_item = ((current_page + 1) * props.page_size).min(props.total);

    html! {
        <div class="pagination-container">
            <div class="pagination-info">
                {format!("显示 {}-{} 条，共 {} 条", start_item, end_item, props.total)}
            </div>
            <div class="pagination-controls">
                <button
                    class="btn btn-sm btn-secondary"
                    onclick={on_prev}
                    disabled={current_page == 0}
                >
                    {"上一页"}
                </button>

                <div class="page-numbers">
                    {for (0..total_pages).map(|page| {
                        let is_active = page == current_page;
                        let on_page_change = on_page_change.clone();
                        let page_class = if is_active {
                            "page-number active"
                        } else {
                            "page-number"
                        };
                        html! {
                            <button
                                class={page_class}
                                onclick={Callback::from(move |_| on_page_change.emit(page))}
                            >
                                {page + 1}
                            </button>
                        }
                    })}
                </div>

                <button
                    class="btn btn-sm btn-secondary"
                    onclick={on_next}
                    disabled={current_page + 1 >= total_pages}
                >
                    {"下一页"}
                </button>
            </div>
        </div>
    }
}
