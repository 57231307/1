use yew::prelude::*;
use crate::services::purchase_price_service::PurchasePriceService;
use crate::services::crud_service::CrudService;
use crate::models::purchase_price::PurchasePrice;

#[function_component(PurchasePricePage)]
pub fn purchase_price_page() -> Html {
    let prices = use_state(|| Vec::<PurchasePrice>::new());
    let loading = use_state(|| false);
    let error = use_state(|| String::new());

    let load_data = {
        let prices = prices.clone();
        let loading = loading.clone();
        let error = error.clone();

        Callback::from(move |_| {
            let prices = prices.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match PurchasePriceService::list(None, None, None, None, 1, 50).await {
                    Ok(data) => {
                        prices.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载价格数据失败: {}", e));
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
        <div class="purchase-price-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"采购价格管理"}</h1>
                <div>
                    <button class="btn btn-primary" onclick={load_data.reform(|_| ())} style="margin-right: 10px;">{"刷新数据"}</button>
                    <button class="btn btn-success">{"新增报价"}</button>
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
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"产品ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"供应商ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"单价"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"币种"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"起订量"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"生效日期"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for prices.iter().map(|p| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ p.id }</td>
                                    <td style="padding: 10px;">{ p.product_id }</td>
                                    <td style="padding: 10px;">{ p.supplier_id }</td>
                                    <td style="padding: 10px; font-weight: bold; color: #2980b9;">{ &p.price }</td>
                                    <td style="padding: 10px;">{ &p.currency }</td>
                                    <td style="padding: 10px;">{ p.min_order_qty.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">{ &p.effective_date }</td>
                                    <td style="padding: 10px;">
                                        if p.status == "ACTIVE" {
                                            <span style="color: green;">{"已生效"}</span>
                                        } else {
                                            <span>{ &p.status }</span>
                                        }
                                    </td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"趋势分析"}</button>
                                        if p.status == "DRAFT" {
                                            <button class="btn btn-sm btn-success" style="margin-right: 5px;">{"审批"}</button>
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
