//! 质量检验页面

use crate::components::main_layout::MainLayout;
use crate::models::quality_inspection::InspectionRecord;
use crate::services::quality_inspection_service::QualityInspectionService;
use yew::prelude::*;

#[function_component(QualityInspectionPage)]
pub fn quality_inspection_page() -> Html {
    let records = use_state(|| Vec::<InspectionRecord>::new());

    {
        let records = records.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                if let Ok(res) = QualityInspectionService::list_records(None, None, None, None, None, 1, 10).await {
                    records.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={""}>
<div class="p-4">
            <h1 class="text-2xl font-bold mb-4">{ "质量检验记录" }</h1>
            <table class="min-w-full bg-white border border-gray-200">
                <thead>
                    <tr>
                        <th class="py-2 px-4 border-b">{ "记录号" }</th>
                        <th class="py-2 px-4 border-b">{ "产品ID" }</th>
                        <th class="py-2 px-4 border-b">{ "批号" }</th>
                        <th class="py-2 px-4 border-b">{ "检验结果" }</th>
                        <th class="py-2 px-4 border-b">{ "数量" }</th>
                        <th class="py-2 px-4 border-b">{ "检验日期" }</th>
                    </tr>
                </thead>
                <tbody>
                    {
                        if records.is_empty() {
                            html! { <tr><td colspan="6" class="text-center py-4">{ "暂无数据" }</td></tr> }
                        } else {
                            html! {
                                for records.iter().map(|record| html! {
                                    <tr key={record.id}>
                                        <td class="py-2 px-4 border-b text-center">{ &record.record_number }</td>
                                        <td class="py-2 px-4 border-b text-center">{ record.product_id }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &record.batch_number }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &record.inspection_result }</td>
                                        <td class="py-2 px-4 border-b text-center">{ record.quantity }</td>
                                        <td class="py-2 px-4 border-b text-center">{ &record.inspection_date }</td>
                                    </tr>
                                })
                            }
                        }
                    }
                </tbody>
            </table>
        </div>
    
</MainLayout>}
}
