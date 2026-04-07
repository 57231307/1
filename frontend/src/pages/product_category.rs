//! 产品类别管理页面

use yew::prelude::*;

#[function_component(ProductCategoryPage)]
pub fn product_category_page() -> Html {
    html! {
        <div class="product-category-page">
            <div class="header">
                <h1>{"产品类别管理"}</h1>
            </div>
            <div class="content">
                <table class="table">
                    <thead>
                        <tr>
                            <th>{"ID"}</th>
                            <th>{"类别名称"}</th>
                            <th>{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td colspan="3" class="text-center">{"暂无数据"}</td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    }
}
