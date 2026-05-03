use crate::utils::permissions;
use yew::prelude::*;
use crate::services::product_service::ProductService;
use crate::services::crud_service::CrudService;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    let products = use_state(Vec::new);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let products = products.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match ProductService::list().await {
                    Ok(data) => {
                        products.set(data.products);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载失败: {}", e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }
    
    let on_delete = {
        let products = products.clone();
        let error = error.clone();
        Callback::from(move |id: i32| {
            let products = products.clone();
            let error = error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match ProductService::delete(id).await {
                    Ok(_) => {
                        if let Ok(data) = ProductService::list().await {
                            products.set(data.products);
                        }
                    }
                    Err(e) => {
                        error.set(format!("删除失败: {}", e));
                    }
                }
            });
        })
    };

    html! {
        <div class="product-list-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"产品管理"}</h1>
                <button class="btn btn-primary">{"新建产品"}</button>
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
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"产品编码"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"产品名称"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"类别"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"单位"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"价格"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for products.iter().map(|p| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ p.id }</td>
                                    <td style="padding: 10px;">{ &p.code }</td>
                                    <td style="padding: 10px;">{ &p.name }</td>
                                    <td style="padding: 10px;">{ p.category_id.map(|id| id.to_string()).unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ &p.unit }</td>
                                    <td style="padding: 10px;">{ p.price.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"编辑"}</button>
                                        <button class="btn btn-sm btn-danger" onclick={{ let id = p.id; on_delete.reform(move |_| id) }}>{"删除"}</button>
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
