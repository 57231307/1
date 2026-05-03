use crate::utils::permissions;
use yew::prelude::*;
use crate::services::quality_inspection_service::QualityInspectionService;
use crate::services::crud_service::CrudService;
use crate::models::quality_inspection::InspectionRecord;

#[function_component(QualityInspectionPage)]
pub fn quality_inspection_page() -> Html {
    let records = use_state(|| Vec::<InspectionRecord>::new());
    let loading = use_state(|| false);
    let error = use_state(|| String::new());

    let load_data = {
        let records = records.clone();
        let loading = loading.clone();
        let error = error.clone();

        Callback::from(move |_| {
            let records = records.clone();
            let loading = loading.clone();
            let error = error.clone();
            
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match QualityInspectionService::list_records(None, None, None, None, None, 1, 50).await {
                    Ok(data) => {
                        records.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载质检记录失败: {}", e));
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
        <div class="quality-inspection-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"质量检验记录"}</h1>
                <div>
                    <button class="btn btn-primary" onclick={load_data.reform(|_| ())} style="margin-right: 10px;">{"刷新数据"}</button>
                    <button class="btn btn-success">{"新建检验记录"}</button>
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
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"记录单号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"产品ID"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"批次号"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"送检数"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"合格数"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"不合格数"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"检验结果"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"检验日期"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for records.iter().map(|r| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ &r.record_number }</td>
                                    <td style="padding: 10px;">{ r.product_id }</td>
                                    <td style="padding: 10px;">{ &r.batch_number }</td>
                                    <td style="padding: 10px;">{ r.quantity }</td>
                                    <td style="padding: 10px; color: green;">{ r.qualified_quantity }</td>
                                    <td style="padding: 10px; color: red;">{ r.unqualified_quantity }</td>
                                    <td style="padding: 10px;">
                                        if r.inspection_result == "PASS" {
                                            <span style="color: green; font-weight: bold;">{"合格"}</span>
                                        } else {
                                            <span style="color: red; font-weight: bold;">{ &r.inspection_result }</span>
                                        }
                                    </td>
                                    <td style="padding: 10px;">{ &r.inspection_date }</td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"查看详情"}</button>
                                        if r.unqualified_quantity > 0 {
                                            <button class="btn btn-sm btn-warning">{"处理缺陷"}</button>
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
