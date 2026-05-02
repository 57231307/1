use yew::prelude::*;
use crate::services::voucher_service::VoucherService;
use crate::models::voucher::{Voucher, VoucherQueryParams};

#[function_component(VoucherPage)]
pub fn voucher_page() -> Html {
    let vouchers = use_state(Vec::new);
    let total = use_state(|| 0u64);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let vouchers = vouchers.clone();
        let total = total.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                let params = VoucherQueryParams {
                    voucher_type: None,
                    status: None,
                    start_date: None,
                    end_date: None,
                    batch_no: None,
                    color_no: None,
                    page: Some(1),
                    page_size: Some(20),
                };
                
                match VoucherService::list_vouchers(params).await {
                    Ok(resp) => {
                        vouchers.set(resp.data);
                        total.set(resp.total);
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

    html! {
        <div class="voucher-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"凭证管理"}</h1>
                <button class="btn btn-primary">{"新建凭证"}</button>
            </div>
            
            if *loading {
                <div class="loading">{"加载中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red; margin-bottom: 10px;">{ (*error).clone() }</div>
            } else {
                <div class="summary" style="margin-bottom: 10px;">
                    {format!("共找到 {} 条记录", *total)}
                </div>
                <table class="table" style="width: 100%; border-collapse: collapse;">
                    <thead>
                        <tr style="background-color: #f5f5f5; text-align: left;">
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"凭证号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"类型"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"日期"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"总借方"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"总贷方"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"制单人"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for vouchers.iter().map(|v| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ &v.voucher_no }</td>
                                    <td style="padding: 10px;">{ &v.voucher_type }</td>
                                    <td style="padding: 10px;">{ &v.voucher_date }</td>
                                    <td style="padding: 10px;">{ &v.total_debit }</td>
                                    <td style="padding: 10px;">{ &v.total_credit }</td>
                                    <td style="padding: 10px;">{ &v.status }</td>
                                    <td style="padding: 10px;">{ v.creator_name.clone().unwrap_or_default() }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"查看"}</button>
                                        if v.status == "DRAFT" {
                                            <button class="btn btn-sm" style="margin-right: 5px;">{"提交"}</button>
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
