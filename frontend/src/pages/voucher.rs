//! 凭证管理页面

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
        <div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "凭证管理" }</h1>
            <table class="min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "凭证号" }</th>
                        <th class="py-2 px-4 border-b">{ "凭证类型" }</th>
                        <th class="py-2 px-4 border-b">{ "凭证日期" }</th>
                        <th class="py-2 px-4 border-b">{ "借方总计" }</th>
                        <th class="py-2 px-4 border-b">{ "贷方总计" }</th>
                        <th class="py-2 px-4 border-b">{ "状态" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if vouchers.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for vouchers.iter().map(|voucher| html! {
                                    <tr key={voucher.id}>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.voucher_no }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.voucher_type }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.voucher_date }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.total_debit }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.total_credit }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &voucher.status }</td>
                                    </tr>
                                })
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    }
}
