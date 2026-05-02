use yew::prelude::*;
use crate::services::warehouse_service::WarehouseService;
use crate::services::crud_service::CrudService;

#[function_component(WarehouseListPage)]
pub fn warehouse_list_page() -> Html {
    let warehouses = use_state(Vec::new);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let warehouses = warehouses.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match WarehouseService::list().await {
                    Ok(data) => {
                        warehouses.set(data.warehouses);
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
        let warehouses = warehouses.clone();
        let error = error.clone();
        Callback::from(move |id: i32| {
            let warehouses = warehouses.clone();
            let error = error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match WarehouseService::delete(id).await {
                    Ok(_) => {
                        if let Ok(data) = WarehouseService::list().await {
                            warehouses.set(data.warehouses);
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
        <div class="warehouse-list-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"仓库管理"}</h1>
                <button class="btn btn-primary">{"新建仓库"}</button>
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
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"仓库编码"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"仓库名称"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"负责人"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"联系电话"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"地址"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for warehouses.iter().map(|w| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ w.id }</td>
                                    <td style="padding: 10px;">{ &w.code }</td>
                                    <td style="padding: 10px;">{ &w.name }</td>
                                    <td style="padding: 10px;">{ w.manager.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ w.phone.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ w.address.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"编辑"}</button>
                                        <button class="btn btn-sm btn-danger" onclick={{ let id = w.id; on_delete.reform(move |_| id) }}>{"删除"}</button>
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
