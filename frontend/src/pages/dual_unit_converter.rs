use yew::prelude::*;
use crate::components::main_layout::MainLayout;
use crate::models::unit_converter::{GlobalUnitConstant, ProductConversion};
use crate::services::unit_converter::UnitConverterService;
use wasm_bindgen_futures::spawn_local;

#[function_component(DualUnitConverterPage)]
pub fn dual_unit_converter_page() -> Html {
    let constants = use_state(Vec::new);
    let products = use_state(Vec::new);
    let loading = use_state(|| true);

    {
        let constants = constants.clone();
        let products = products.clone();
        let loading = loading.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(c) = UnitConverterService::get_global_constants().await {
                    constants.set(c);
                }
                if let Ok(p) = UnitConverterService::get_product_conversions().await {
                    products.set(p);
                }
                loading.set(false);
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"双单位换算"}>
            <div class="p-4">
                <h2 class="text-xl font-bold mb-4">{"双单位换算规则中心"}</h2>
                
                if *loading {
                    <div>{"数据加载中..."}</div>
                } else {
                    <div class="mb-8">
                        <h3 class="text-lg font-semibold mb-2">{"全局固定公式 (物理换算)"}</h3>
                        <div class="table-responsive">
                            <table class="data-table w-full">
                                <thead>
                                    <tr>
                                        <th>{"换算前单位"}</th>
                                        <th>{"换算后单位 (主库存单位)"}</th>
                                        <th class="text-right">{"固定换算系数"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for constants.iter().map(|c| html! {
                                        <tr>
                                            <td>{&c.from_unit}</td>
                                            <td>{&c.to_unit}</td>
                                            <td class="numeric-cell">{format!("{:.4}", c.ratio)}</td>
                                        </tr>
                                    })}
                                </tbody>
                            </table>
                        </div>
                    </div>

                    <div>
                        <h3 class="text-lg font-semibold mb-2">{"产品级绑定公式 (米 ↔ 公斤)"}</h3>
                        <div class="table-responsive">
                            <table class="data-table w-full">
                                <thead>
                                    <tr>
                                        <th>{"产品编号"}</th>
                                        <th>{"产品名称"}</th>
                                        <th class="text-right">{"门幅 (cm)"}</th>
                                        <th class="text-right">{"克重 (g/m²)"}</th>
                                        <th class="text-right">{"米/公斤 系数 (1公斤=X米)"}</th>
                                        <th class="text-center">{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    {for products.iter().map(|p| html! {
                                        <tr>
                                            <td>{&p.product_code}</td>
                                            <td>{&p.product_name}</td>
                                            <td class="numeric-cell">{format!("{:.1}", p.width_cm)}</td>
                                            <td class="numeric-cell">{format!("{:.1}", p.weight_gsm)}</td>
                                            <td class="numeric-cell font-bold text-blue-600">{format!("{:.4}", p.meters_per_kg)}</td>
                                            <td class="text-center">
                                                <button class="btn-secondary text-xs px-2 py-1">{"微调"}</button>
                                            </td>
                                        </tr>
                                    })}
                                </tbody>
                            </table>
                        </div>
                    </div>
                }
            </div>
        </MainLayout>
    }
}