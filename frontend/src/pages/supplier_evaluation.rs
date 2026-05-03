use crate::utils::permissions;
use yew::prelude::*;
use crate::services::supplier_service::SupplierService;
use crate::services::crud_service::CrudService;

#[function_component(SupplierEvaluationPage)]
pub fn supplier_evaluation_page() -> Html {
    let suppliers = use_state(Vec::new);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let suppliers = suppliers.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match SupplierService::list().await {
                    Ok(resp) => {
                        suppliers.set(resp.data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载供应商数据失败: {}", e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    html! {
        <div class="supplier-evaluation-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <div>
                    <h1>{"供应商绩效与评估"}</h1>
                    <p style="color: #666;">{"基于质量、交期和服务的供应商评级体系"}</p>
                </div>
                <button class="btn btn-primary">{"发起评估"}</button>
            </div>
            
            if *loading {
                <div class="loading">{"加载数据中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red;">{ (*error).clone() }</div>
            } else {
                <div class="dashboard-grid" style="display: grid; grid-template-columns: repeat(3, 1fr); gap: 20px; margin-bottom: 20px;">
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"优秀 (A级)"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #27ae60;">
                            {suppliers.iter().filter(|s| s.grade.as_deref() == Some("A")).count()}
                        </div>
                    </div>
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"合格 (B级)"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #f39c12;">
                            {suppliers.iter().filter(|s| s.grade.as_deref() == Some("B")).count()}
                        </div>
                    </div>
                    <div class="card" style="padding: 20px; border: 1px solid #ddd; border-radius: 8px; background: #fff;">
                        <h3>{"淘汰预警 (C/D级)"}</h3>
                        <div style="font-size: 24px; font-weight: bold; color: #e74c3c;">
                            {suppliers.iter().filter(|s| s.grade.as_deref() == Some("C") || s.grade.as_deref() == Some("D")).count()}
                        </div>
                    </div>
                </div>
                
                <table class="table" style="width: 100%; border-collapse: collapse; margin-top: 20px;">
                    <thead>
                        <tr style="background-color: #f5f5f5; text-align: left;">
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"供应商编码"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"供应商名称"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"当前等级"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"评估分数"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"状态"}</th>
                            <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody>
                        { for suppliers.iter().map(|s| {
                            html! {
                                <tr style="border-bottom: 1px solid #eee;">
                                    <td style="padding: 10px;">{ &s.credit_code }</td>
                                    <td style="padding: 10px;">{ &s.supplier_name }</td>
                                    <td style="padding: 10px;">
                                        <span class={format!("badge grade-{}", s.grade.clone().unwrap_or_default())}>
                                            { s.grade.clone().unwrap_or_else(|| "未评估".to_string()) }
                                        </span>
                                    </td>
                                    <td style="padding: 10px;">{ s.grade_score.clone().unwrap_or_else(|| "-".to_string()) }</td>
                                    <td style="padding: 10px;">
                                        if s.is_enabled {
                                            <span style="color: green;">{"启用"}</span>
                                        } else {
                                            <span style="color: red;">{"禁用"}</span>
                                        }
                                    </td>
                                    <td style="padding: 10px;">
                                        <button class="btn btn-sm" style="margin-right: 5px;">{"评估明细"}</button>
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
