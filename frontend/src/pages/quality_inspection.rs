//! 质量检验页面

use crate::components::main_layout::MainLayout;
use yew::prelude::*;
use web_sys::window;

#[derive(Clone, PartialEq)]
pub struct InspectItem {
    pub id: u32,
    pub record_no: String,
    pub roll_no: String,
    pub product_name: String,
    pub defect_type: String, // 疵点类型：破洞抽纱扣减、色差等
    pub defect_count: u32, // 疵点数
    pub pass_rate: f64, // 合格率
    pub result: String, // 检验结果
    pub inspect_date: String,
}

#[function_component(QualityInspectionPage)]
pub fn quality_inspection_page() -> Html {
    let records = use_state(|| Vec::<InspectItem>::new());
    let on_print = Callback::from(|_: yew::MouseEvent| {
        if let Some(win) = window() {
            let _ = win.print();
        }
    });

    let show_add_modal = use_state(|| false);
    let warp_defects = use_state(|| 0.0);
    let weft_defects = use_state(|| 0.0);
    let inspection_area = use_state(|| 100.0);

    {
        let records = records.clone();
        use_effect_with((), move |_| {
            let initial_data = vec![
                InspectItem {
                    id: 1,
                    record_no: "QC-20231001-001".to_string(),
                    roll_no: "匹号A101".to_string(),
                    product_name: "纯棉平布".to_string(),
                    defect_type: "无明显疵点".to_string(),
                    defect_count: 0,
                    pass_rate: 100.0,
                    result: "合格".to_string(),
                    inspect_date: "2023-10-01".to_string(),
                },
                InspectItem {
                    id: 2,
                    record_no: "QC-20231001-002".to_string(),
                    roll_no: "匹号A102".to_string(),
                    product_name: "涤纶汗布".to_string(),
                    defect_type: "破洞抽纱扣减".to_string(),
                    defect_count: 3,
                    pass_rate: 92.5,
                    result: "降级".to_string(),
                    inspect_date: "2023-10-01".to_string(),
                },
                InspectItem {
                    id: 3,
                    record_no: "QC-20231002-003".to_string(),
                    roll_no: "匹号B205".to_string(),
                    product_name: "全棉斜纹".to_string(),
                    defect_type: "严重色差".to_string(),
                    defect_count: 5,
                    pass_rate: 60.0,
                    result: "不合格".to_string(),
                    inspect_date: "2023-10-02".to_string(),
                },
            ];
            records.set(initial_data);
            || ()
        });
    }

    let show_add_modal_cb = {
        let show_add_modal = show_add_modal.clone();
        Callback::from(move |_| show_add_modal.set(true))
    };

    let close_modal_cb = {
        let show_add_modal = show_add_modal.clone();
        Callback::from(move |_| show_add_modal.set(false))
    };

    let score = if *inspection_area > 0.0 {
        (*warp_defects + *weft_defects) * 100.0 / *inspection_area
    } else {
        0.0
    };

    let result_text = if score > 40.0 {
        "C级/不合格"
    } else {
        "A级/合格"
    };

    let result_class = if score > 40.0 {
        "text-red-600 font-bold"
    } else {
        "text-green-600 font-bold"
    };

    html! {
        <MainLayout current_page={"quality_inspection"}>
            <div class="quality-inspection-page p-4 relative">
                <div class="header mb-4 flex justify-between items-center">
                    <h1 class="text-2xl font-bold">{"质量检验记录"}</h1>
                    <button onclick={show_add_modal_cb} class="bg-blue-500 hover:bg-blue-600 text-white px-4 py-2 rounded shadow-sm">{"新增检验"}</button>
                </div>


                <div class="filter-form bg-white p-4 rounded mb-4 shadow-sm border border-gray-100">
                    <div class="grid grid-cols-1 md:grid-cols-4 gap-4 items-end">
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"检验单号"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="如: QC-..." />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"匹号"}</label>
                            <input type="text" class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500" placeholder="如: 匹号A102" />
                        </div>
                        <div>
                            <label class="block text-sm font-medium text-gray-700 mb-1">{"检验结果"}</label>
                            <select class="block w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500">
                                <option value="">{"全部"}</option>
                                <option value="合格">{"合格"}</option>
                                <option value="降级">{"降级"}</option>
                                <option value="不合格">{"不合格"}</option>
                            </select>
                        </div>
                        <div>
                            <button class="w-full bg-gray-100 hover:bg-gray-200 text-gray-800 px-4 py-2 rounded border border-gray-300 shadow-sm font-medium">{"查询"}</button>
                        </div>
                    </div>
                </div>

                <div class="content bg-white rounded shadow-sm border border-gray-100 overflow-hidden">
                    <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg">
<table class="data-table w-full text-left">
                        <thead class="bg-gray-50 border-b border-gray-200">
                            <tr>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"ID"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"检验单号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"匹号"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"品名"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"疵点类型"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"疵点数"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 numeric-cell text-right">{"合格率"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"检验结果"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600">{"检验日期"}</th>
                                <th class="py-3 px-4 font-semibold text-gray-600 text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-gray-100">
                            {
                                if records.is_empty() {
                                    html! {
                                        <tr><td colspan="10" class="text-center py-8 text-gray-500">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for records.iter().map(|record| {
                                            let result_class = match record.result.as_str() {
                                                "合格" => "bg-green-100 text-green-800",
                                                "降级" => "bg-yellow-100 text-yellow-800",
                                                "不合格" => "bg-red-100 text-red-800",
                                                _ => "bg-gray-100 text-gray-800"
                                            };
                                            let pass_rate_class = if record.pass_rate >= 95.0 {
                                                "text-green-600"
                                            } else if record.pass_rate >= 80.0 {
                                                "text-yellow-600"
                                            } else {
                                                "text-red-600"
                                            };
                                            html! {
                                                <tr key={record.id} class="hover:bg-gray-50 transition-colors">
                                                    <td class="py-3 px-4">{ record.id }</td>
                                                    <td class="py-3 px-4 font-medium">{ &record.record_no }</td>
                                                    <td class="py-3 px-4">{ &record.roll_no }</td>
                                                    <td class="py-3 px-4">{ &record.product_name }</td>
                                                    <td class="py-3 px-4 text-sm text-gray-600">{ &record.defect_type }</td>
                                                    <td class="py-3 px-4 numeric-cell text-right font-mono">{ record.defect_count }</td>
                                                    <td class={format!("py-3 px-4 numeric-cell text-right font-mono font-medium {}", pass_rate_class)}>
                                                        { format!("{:.1}%", record.pass_rate) }
                                                    </td>
                                                    <td class="py-3 px-4 text-center">
                                                        <span class={format!("status-badge px-2.5 py-1 rounded-full text-xs font-medium {}", result_class)}>
                                                            { &record.result }
                                                        </span>
                                                    </td>
                                                    <td class="py-3 px-4 text-sm text-gray-500">{ &record.inspect_date }</td>
                                                    <td class="py-3 px-4 text-center">
                                                        <button class="text-blue-600 hover:text-blue-800 font-medium">{"查看"}</button>
                                                    </td>
                                                </tr>
                                            }
                                        })
                                    }
                                }
                            }
                        </tbody>
                    </table>
</div>
                </div>

                if *show_add_modal {
                    <div class="fixed inset-0 bg-gray-600 bg-opacity-50 overflow-y-auto h-full w-full z-50 flex items-center justify-center">
                        <div class="bg-white p-8 rounded-lg shadow-xl w-full max-w-md">
                            <h2 class="text-xl font-bold mb-6">{"新增质量检验 - 美标四分制"}</h2>
                            
                            <div class="space-y-4">
                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"经向疵点数"}</label>
                                    <input 
                                        type="number" 
                                        min="0"
                                        class="w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500"
                                        value={warp_defects.to_string()}
                                        oninput={
                                            let warp_defects = warp_defects.clone();
                                            Callback::from(move |e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                if let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                                    if let Ok(val) = input.value().parse::<f64>() {
                                                        warp_defects.set(val);
                                                    }
                                                }
                                            })
                                        }
                                    />
                                </div>

                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"纬向疵点数"}</label>
                                    <input 
                                        type="number" 
                                        min="0"
                                        class="w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500"
                                        value={weft_defects.to_string()}
                                        oninput={
                                            let weft_defects = weft_defects.clone();
                                            Callback::from(move |e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                if let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                                    if let Ok(val) = input.value().parse::<f64>() {
                                                        weft_defects.set(val);
                                                    }
                                                }
                                            })
                                        }
                                    />
                                </div>

                                <div>
                                    <label class="block text-sm font-medium text-gray-700 mb-1">{"检验面积(平方码)"}</label>
                                    <input 
                                        type="number" 
                                        min="0.1"
                                        step="0.1"
                                        class="w-full rounded-md border-gray-300 shadow-sm p-2 border focus:border-blue-500 focus:ring-blue-500"
                                        value={inspection_area.to_string()}
                                        oninput={
                                            let inspection_area = inspection_area.clone();
                                            Callback::from(move |e: InputEvent| {
                                                use wasm_bindgen::JsCast;
                                                if let Some(input) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok()) {
                                                    if let Ok(val) = input.value().parse::<f64>() {
                                                        inspection_area.set(val);
                                                    }
                                                }
                                            })
                                        }
                                    />
                                </div>

                                <div class="bg-gray-50 p-4 rounded-md border border-gray-200 mt-6">
                                    <div class="flex justify-between items-center mb-2">
                                        <span class="text-gray-600 font-medium">{"百平方码扣分:"}</span>
                                        <span class="text-xl font-mono">{format!("{:.1}", score)}</span>
                                    </div>
                                    <div class="flex justify-between items-center">
                                        <span class="text-gray-600 font-medium">{"检验结果:"}</span>
                                        <span class={result_class}>{result_text}</span>
                                    </div>
                                </div>
                            </div>

                            <div class="mt-8 flex justify-end space-x-3">
                                <button onclick={close_modal_cb.clone()} class="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-800 rounded-md border border-gray-300 font-medium">
                                    {"取消"}
                                </button>
                                <button onclick={close_modal_cb} class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md shadow-sm font-medium">
                                    {"保存"}
                                </button>
                            </div>
                        </div>
                    </div>
                }
            </div>
        </MainLayout>
    }
}
