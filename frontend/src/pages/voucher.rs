//! 凭证管理页面

use crate::components::main_layout::MainLayout;
use crate::models::voucher::{Voucher, VoucherQueryParams};
use crate::services::voucher_service::VoucherService;
use yew::prelude::*;

#[function_component(VoucherPage)]
pub fn voucher_page() -> Html {
    let vouchers = use_state(|| Vec::<Voucher>::new());

    {
        let vouchers = vouchers.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let params = VoucherQueryParams {
                    voucher_type: None,
                    status: None,
                    start_date: None,
                    end_date: None,
                    batch_no: None,
                    color_no: None,
                    page: Some(1),
                    page_size: Some(10),
                };
                if let Ok(res) = VoucherService::list_vouchers(params).await {
                    vouchers.set(res.data);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"voucher"}>
<div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "凭证管理" }</h1>
            <table class="data-table w-full">
                <thead>
                    <tr>
                        <th>{ "凭证号" }</th>
                        <th>{ "凭证类型" }</th>
                        <th>{ "凭证日期" }</th>
                        <th class="numeric-cell text-right">{ "借方总计" }</th>
                        <th class="numeric-cell text-right">{ "贷方总计" }</th>
                        <th>{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if vouchers.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for vouchers.iter().map(|voucher| {
                                    let status = voucher.status.clone();
                                    let status_class = match status.as_str() {
                                        "草稿" => "status-draft",
                                        "已审核" => "status-approved",
                                        "已记账" => "status-posted",
                                        "已作废" => "status-cancelled",
                                        _ => "status-default",
                                    };
                                    html! {
                                        <tr key={voucher.id}>
                                            <td>{ &voucher.voucher_no }</td>
                                            <td>{ &voucher.voucher_type }</td>
                                            <td>{ &voucher.voucher_date }</td>
                                            <td class="numeric-cell text-right">{ &voucher.total_debit }</td>
                                            <td class="numeric-cell text-right">{ &voucher.total_credit }</td>
                                            <td><span class={format!("status-badge {}", status_class)}>{ status }</span></td>
                                        </tr>
                                    }
                                })
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    
</MainLayout>}
}
