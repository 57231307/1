use yew::prelude::*;
use crate::services::product_category_service::ProductCategoryService;
use crate::services::crud_service::CrudService;
use crate::models::product_category::ProductCategory;

#[function_component(ProductCategoryPage)]
pub fn product_category_page() -> Html {
    let categories = use_state(|| Vec::<ProductCategory>::new());
    let loading = use_state(|| false);
    let error = use_state(|| String::new());

    let load_data = {
        let categories = categories.clone();
        let loading = loading.clone();
        let error = error.clone();

        Callback::from(move |_| {
            let categories = categories.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match ProductCategoryService::list().await {
                    Ok(data) => {
                        categories.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载产品分类数据失败: {}", e));
                        loading.set(false);
                    }
                }
            });
        })
    };

    {
        let load_data = load_data.clone();
        use_effect_with((), move |_| {
            load_data.emit(());
            || ()
        });
    }

    html! {
        <div class="product-category-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"产品类别管理"}</h1>
                <div>
                    <button class="btn btn-primary" onclick={load_data.reform(|_| ())} style="margin-right: 10px;">{"刷新数据"}</button>
                    <button class="btn btn-success">{"新增类别"}</button>
                </div>
            </div>
            
            if *loading {
                <div class="loading">{"加载中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red; margin-bottom: 10px;">{ (*error).clone() }</div>
            } else {
                <table class="table" style="width: 100%; border-collapse: collapse;">
                    <thead>
                        <tr style="background-color: #f5f5f5; text-align: left;">
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"编码"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"名称"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"上级类别ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"描述"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for categories.iter().map(|c| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ c.id }</td>
                                    <td style="padding: 10px;">{ &c.code }</td>
                                    <td style="padding: 10px;">{ &c.name }</td>
                                    <td style="padding: 10px;">{ c.parent_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string()) }</td>
                                    <td style="padding: 10px;">{ c.description.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">
                                        if c.is_enabled {
                                            <span style="color: green;">{"启用"}</span>
                                        } else {
                                            <span style="color: red;">{"禁用"}</span>
                                        }
                                    </td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"编辑"}</button>
                                        <button class="btn btn-sm btn-danger">{"删除"}</button>
                                    </td>
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            }
        </div>
    }
}
