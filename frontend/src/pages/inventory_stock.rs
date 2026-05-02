use yew::prelude::*;
use crate::services::inventory_service::InventoryService;
use crate::services::crud_service::CrudService;
use crate::models::inventory::{StockFabricResponse, InventorySummaryResponse};

#[function_component(InventoryStockPage)]
pub fn inventory_stock_page() -> Html {
    let stocks = use_state(|| Vec::<StockFabricResponse>::new());
    let summary = use_state(|| None::<InventorySummaryResponse>);
    let total = use_state(|| 0u64);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());

    let batch_filter = use_state(|| String::new());
    let color_filter = use_state(|| String::new());
    
    let load_data = {
        let stocks = stocks.clone();
        let summary = summary.clone();
        let total = total.clone();
        let loading = loading.clone();
        let error = error.clone();
        let batch = batch_filter.clone();
        let color = color_filter.clone();

        Callback::from(move |_| {
            let stocks = stocks.clone();
            let summary = summary.clone();
            let total = total.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            let b = if (*batch).is_empty() { None } else { Some((*batch).as_str()) };
            let c = if (*color).is_empty() { None } else { Some((*color).as_str()) };

            loading.set(true);
            let b_loading = loading.clone();
            let b_owned = b.map(|s| s.to_string());
            let c_owned = c.map(|s| s.to_string());
            wasm_bindgen_futures::spawn_local(async move {
                // 加载面料库存
                match InventoryService::list_stock_fabric(1, 50, b_owned.as_deref(), c_owned.as_deref()).await {
                    Ok(resp) => {
                        stocks.set(resp.stock);
                        total.set(resp.total);
                    }
                    Err(e) => error.set(format!("加载库存失败: {}", e))
                }
                
                // 顺便加载汇总数据
                if b_owned.is_none() && c_owned.is_none() {
                    if let Ok(sum) = InventoryService::get_inventory_summary().await {
                        summary.set(Some(sum));
                    }
                }
                b_loading.set(false);
            });
        })
    };

    // 初始加载
    {
        let load_data = load_data.clone();
        use_effect_with((), move |_| {
            load_data.emit(());
            || ()
        });
    }

    let on_batch_change = {
        let batch_filter = batch_filter.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            batch_filter.set(input.value());
        })
    };

    let on_color_change = {
        let color_filter = color_filter.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            color_filter.set(input.value());
        })
    };

    html! {
        <div class="inventory-stock-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"面料库存查询"}</h1>
                <button class="btn btn-primary" onclick={load_data.reform(|_| ())}>{"刷新数据"}</button>
            </div>
            
            if let Some(sum) = (*summary).clone() {
                <div class="dashboard-grid" style="display: grid; grid-template-columns: repeat(2, 1fr); gap: 20px; margin-bottom: 20px;">
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"库存总米数"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #2980b9;">
                            {format!("{} M", sum.total_meters)}
                        </div>
                    </div>
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"库存总重量"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #27ae60;">
                            {format!("{} KG", sum.total_kg)}
                        </div>
                    </div>
                </div>
            }

            <div class="filters" style="margin-bottom: 20px; padding: 15px; background: #f9f9f9; border-radius: 4px;">
                <input type="text" placeholder="按批号搜索" value={(*batch_filter).clone()} oninput={on_batch_change} style="margin-right: 10px; padding: 5px;" />
                <input type="text" placeholder="按色号搜索" value={(*color_filter).clone()} oninput={on_color_change} style="margin-right: 10px; padding: 5px;" />
                <button class="btn" onclick={load_data.reform(|_| ())}>{"搜索"}</button>
            </div>
            
            if *loading {
                <div class="loading">{"加载中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red; margin-bottom: 10px;">{ (*error).clone() }</div>
            } else {
                <table class="table" style="width: 100%; border-collapse: collapse;">
                    <thead>
                        <tr style="background-color: #f5f5f5; text-align: left;">
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"仓库ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"产品ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"批号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"色号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"缸号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"等级"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"米数"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"重量"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"库位"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for stocks.iter().map(|s| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ s.warehouse_id }</td>
                                    <td style="padding: 10px;">{ s.product_id }</td>
                                    <td style="padding: 10px;">{ &s.batch_no }</td>
                                    <td style="padding: 10px;">{ &s.color_no }</td>
                                    <td style="padding: 10px;">{ s.dye_lot_no.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ &s.grade }</td>
                                    <td style="padding: 10px;">{ &s.quantity_meters }</td>
                                    <td style="padding: 10px;">{ &s.quantity_kg }</td>
                                    <td style="padding: 10px;">{ s.bin_location.clone().unwrap_or_default() }</td>
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            }
        </div>
    }
}
