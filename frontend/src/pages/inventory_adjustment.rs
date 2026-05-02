use yew::prelude::*;
use crate::services::inventory_adjustment_service::InventoryAdjustmentService;
use crate::services::crud_service::CrudService;
use crate::models::inventory_adjustment::AdjustmentSummary;

#[function_component(InventoryAdjustmentPage)]
pub fn inventory_adjustment_page() -> Html {
    let adjustments = use_state(|| Vec::<AdjustmentSummary>::new());
    let loading = use_state(|| false);
    let error = use_state(|| String::new());

    let load_data = {
        let adjustments = adjustments.clone();
        let loading = loading.clone();
        let error = error.clone();

        Callback::from(move |_| {
            let adjustments = adjustments.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match InventoryAdjustmentService::list_adjustments(Some(1), Some(50)).await {
                    Ok(resp) => {
                        adjustments.set(resp.adjustments);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载库存调整单失败: {}", e));
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
        <div class="inventory-adjustment-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"库存调整单管理"}</h1>
                <div>
                    <button class="btn btn-primary" onclick={load_data.reform(|_| ())} style="margin-right: 10px;">{"刷新数据"}</button>
                    <button class="btn btn-success">{"新建调整单"}</button>
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
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"单号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"仓库ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"调整类型"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"原因类型"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"调整数量"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"创建时间"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for adjustments.iter().map(|a| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px; font-weight: bold;">{ &a.adjustment_no }</td>
                                    <td style="padding: 10px;">{ a.warehouse_id }</td>
                                    <td style="padding: 10px;">{ &a.adjustment_type }</td>
                                    <td style="padding: 10px;">{ &a.reason_type }</td>
                                    <td style="padding: 10px;">{ &a.total_quantity }</td>
                                    <td style="padding: 10px;">
                                        if a.status == "APPROVED" {
                                            <span style="color: green;">{"已审核"}</span>
                                        } else if a.status == "DRAFT" {
                                            <span style="color: orange;">{"草稿"}</span>
                                        } else {
                                            <span>{ &a.status }</span>
                                        }
                                    </td>
                                    <td style="padding: 10px;">{ &a.created_at }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"详情"}</button>
                                        if a.status == "DRAFT" {
                                            <button class="btn btn-sm btn-success" style="margin-right: 5px;">{"审核"}</button>
                                            <button class="btn btn-sm btn-danger">{"驳回"}</button>
                                        }
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
