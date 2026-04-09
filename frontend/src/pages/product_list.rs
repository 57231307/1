use crate::components::main_layout::MainLayout;
use crate::services::product_service::ProductService;
use crate::models::product::{Product, CreateProductRequest};
use yew::prelude::*;
use web_sys::window;
use wasm_bindgen_futures::spawn_local;
use wasm_bindgen::JsCast;

#[function_component(ProductListPage)]
pub fn product_list_page() -> Html {
    let products = use_state(|| Vec::<Product>::new());
    let active_tab = use_state(|| "all".to_string());
    let search_query = use_state(String::new);
    let is_mobile = use_state(|| {
        window().unwrap().inner_width().unwrap().as_f64().unwrap() < 768.0
    });

    // Handle resize
    {
        let is_mobile = is_mobile.clone();
        use_effect_with((), move |_| {
            let win = window().unwrap();
            let closure = wasm_bindgen::closure::Closure::wrap(Box::new(move || {
                is_mobile.set(window().unwrap().inner_width().unwrap().as_f64().unwrap() < 768.0);
            }) as Box<dyn FnMut()>);
            win.add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref()).unwrap();
            closure.forget();
            || ()
        });
    }

    {
        let products = products.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ProductService::list_products().await {
                    products.set(res.products);
                }
            });
            || ()
        });
    }

    let on_tab_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: &str| {
            active_tab.set(tab.to_string());
        })
    };

    let filtered_products = products.iter().filter(|p| {
        let matches_tab = match active_tab.as_str() {
            "knit" => p.name.contains("针织") || p.description.as_deref().unwrap_or("").contains("针织"),
            "woven" => p.name.contains("梭织") || p.description.as_deref().unwrap_or("").contains("梭织"),
            _ => true,
        };
        let matches_search = p.name.contains(&*search_query) || p.code.contains(&*search_query);
        matches_tab && matches_search
    }).collect::<Vec<_>>();

    html! {
        <MainLayout current_page="面料档案">
            <div class="space-y-4">
                
                <div class="flex flex-col md:flex-row md:items-center justify-between gap-4">
                    <div class="flex flex-col md:flex-row md:items-center gap-4">
                        <h1 class="text-[18px] font-bold text-[#1D2129]">{"面料档案管理"}</h1>
                        
                        <div class="flex items-center gap-2 bg-[#F5F7FA] p-1 rounded">
                            <button 
                                onclick={let on_tab_change = on_tab_change.clone(); Callback::from(move |_| on_tab_change.emit("all"))}
                                class={format!("px-4 py-1 text-[14px] rounded transition-colors {}", if *active_tab == "all" { "bg-white text-[#165DFF] shadow-sm font-medium" } else { "text-[#4E5969] hover:text-[#165DFF]" })}>
                                {"全部"}
                            </button>
                            <button 
                                onclick={let on_tab_change = on_tab_change.clone(); Callback::from(move |_| on_tab_change.emit("knit"))}
                                class={format!("px-4 py-1 text-[14px] rounded transition-colors flex items-center gap-1 {}", if *active_tab == "knit" { "bg-[#E8F5E9] text-[#4CAF50] shadow-sm font-medium" } else { "text-[#4E5969] hover:text-[#4CAF50]" })}>
                                <span class="w-2 h-2 rounded-full bg-[#4CAF50]"></span>{"针织"}
                            </button>
                            <button 
                                onclick={let on_tab_change = on_tab_change.clone(); Callback::from(move |_| on_tab_change.emit("woven"))}
                                class={format!("px-4 py-1 text-[14px] rounded transition-colors flex items-center gap-1 {}", if *active_tab == "woven" { "bg-[#E3F2FD] text-[#2196F3] shadow-sm font-medium" } else { "text-[#4E5969] hover:text-[#2196F3]" })}>
                                <span class="w-2 h-2 rounded-full bg-[#2196F3]"></span>{"梭织"}
                            </button>
                        </div>
                    </div>
                    <div class="flex items-center gap-2 overflow-x-auto pb-2 md:pb-0">
                        <button class="btn-primary shrink-0">{"新增面料"}</button>
                        <button class="btn-secondary shrink-0">{"导入"}</button>
                        <button class="btn-secondary shrink-0">{"批量打印标签"}</button>
                        <button class="btn-text shrink-0">{"导出"}</button>
                        <button class="btn-text shrink-0" onclick={Callback::from(|_| window().unwrap().location().reload().unwrap())}>{"刷新"}</button>
                    </div>
                </div>

                
                <div class="card p-4 flex flex-wrap gap-3 items-center">
                    <div class="w-full md:w-[220px]">
                        <input type="text" placeholder="面料编号" 
                            value={(*search_query).clone()}
                            oninput={Callback::from(move |e: InputEvent| {
                                let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                search_query.set(input.value());
                            })} 
                        />
                    </div>
                    <div class="w-full md:w-[220px]">
                        <input type="text" placeholder="面料名称" />
                    </div>
                    <div class="w-full md:w-[150px]">
                        <select class="text-[#86909C]">
                            <option value="">{"成分"}</option>
                            <option value="cotton">{"全棉"}</option>
                            <option value="polyester">{"聚酯纤维"}</option>
                        </select>
                    </div>
                    <div class="w-full md:w-[150px] flex items-center gap-2">
                        <input type="text" placeholder="克重" class="w-full" />
                        <span class="text-[#86909C]">{"g/㎡"}</span>
                    </div>
                    <div class="w-full md:w-[150px] flex items-center gap-2">
                        <input type="text" placeholder="门幅" class="w-full" />
                        <span class="text-[#86909C]">{"cm"}</span>
                    </div>
                    <div class="w-full md:w-[180px]">
                        <select class="text-[#86909C]">
                            <option value="">{"供应商"}</option>
                        </select>
                    </div>
                    <button class="text-[#165DFF] text-[14px] flex items-center gap-1 hover:text-[#0F4CD0]">
                        {"高级搜索"}
                        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path></svg>
                    </button>
                </div>

                
                if *is_mobile {
                    
                    <div class="grid grid-cols-1 gap-3 pb-20">
                        {
                            for filtered_products.iter().enumerate().map(|(i, product)| {
                                let is_knit = product.product_type.contains("针织");
                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                let badge_text = if product.product_type.is_empty() { "未分类" } else { &product.product_type };
                                let stock_qty = product.stock_qty.unwrap_or(0.0);
                                let stock_class = if stock_qty < 100.0 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129] font-medium" };
                                
                                let specs = format!("{} / {} / {}", 
                                    product.fabric_composition.as_deref().unwrap_or("-"),
                                    product.gram_weight.map(|w| format!("{}g", w)).unwrap_or("-".to_string()),
                                    product.width.map(|w| format!("{}cm", w)).unwrap_or("-".to_string())
                                );
                                
                                html! {
                                    <div key={product.id} class="card p-3 flex gap-3">
                                        <div class="w-[60px] h-[60px] bg-gray-200 rounded shrink-0 overflow-hidden flex items-center justify-center text-gray-400">
                                            <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>
                                        </div>
                                        <div class="flex-1 flex flex-col justify-between">
                                            <div class="flex justify-between items-start">
                                                <div>
                                                    <div class="font-bold text-[#1D2129] text-[14px]">{&product.code}</div>
                                                    <div class="text-[12px] text-[#4E5969] mt-0.5">{&product.name}</div>
                                                </div>
                                                <span class={format!("px-1.5 py-0.5 rounded text-[10px] {}", badge_class)}>{badge_text}</span>
                                            </div>
                                            <div class="flex justify-between items-end mt-2">
                                                <div class="text-[12px] text-[#4E5969]">
                                                    <div class="truncate w-32">{specs}</div>
                                                    <div>{"库存: "}<span class={stock_class}>{format!("{:.1} {}", stock_qty, product.unit)}</span></div>
                                                </div>
                                                <div class="text-[14px] font-bold text-[#F53F3F]">{"¥"}{product.standard_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())}</div>
                                            </div>
                                        </div>
                                    </div>
                                }
                            })
                        }
                        if filtered_products.is_empty() {
                            <div class="text-center py-10 text-[#86909C]">
                                <svg class="w-12 h-12 mx-auto mb-2 opacity-50" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"></path></svg>
                                {"暂无数据，点击新增"}
                            </div>
                        }
                    </div>
                } else {
                    
                    <div class="card p-0 overflow-hidden">
                        <div class="table-responsive">
                            <table class="data-table">
                                <thead>
                                    <tr>
                                        <th class="w-12 text-center">{"序号"}</th>
                                        <th class="w-16">{"图片"}</th>
                                        <th>{"面料编号"}</th>
                                        <th>{"面料名称"}</th>
                                        <th>{"品类"}</th>
                                        <th>{"核心规格"}</th>
                                        <th class="text-right">{"采购/批发价"}</th>
                                        <th class="text-right">{"当前库存"}</th>
                                        <th>{"所属仓库"}</th>
                                        <th class="text-center">{"操作"}</th>
                                    </tr>
                                </thead>
                                <tbody>
                                    if filtered_products.is_empty() {
                                        <tr><td colspan="10" class="text-center py-10 text-[#86909C]">{"暂无数据"}</td></tr>
                                    } else {
                                        {
                                            for filtered_products.iter().enumerate().map(|(index, product)| {
                                                let is_knit = product.product_type.contains("针织");
                                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                                let badge_text = if product.product_type.is_empty() { "未分类" } else { &product.product_type };
                                                let stock_qty = product.stock_qty.unwrap_or(0.0);
                                                let stock_class = if stock_qty < 100.0 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129]" };
                                                
                                                let specs = format!("{} / {} / {}", 
                                                    product.fabric_composition.as_deref().unwrap_or("-"),
                                                    product.gram_weight.map(|w| format!("{}g", w)).unwrap_or("-".to_string()),
                                                    product.width.map(|w| format!("{}cm", w)).unwrap_or("-".to_string())
                                                );
                                                
                                                html! {
                                                    <tr key={product.id}>
                                                        <td class="text-center text-[#86909C]">{index + 1}</td>
                                                        <td>
                                                            <div class="w-20 h-20 bg-gray-200 rounded flex items-center justify-center text-gray-400 cursor-pointer hover:opacity-80">
                                                                <svg class="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path></svg>
                                                            </div>
                                                        </td>
                                                        <td class="font-bold text-[#1D2129] cursor-pointer hover:text-[#165DFF]">{&product.code}</td>
                                                        <td>{&product.name}</td>
                                                        <td><span class={format!("px-2 py-0.5 rounded text-[12px] {}", badge_class)}>{badge_text}</span></td>
                                                        <td class="text-[#4E5969] text-[12px]">{specs}</td>
                                                        <td class="text-right text-[#1D2129]">
                                                            <div>{"¥"}{product.standard_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())} <span class="text-xs text-[#86909C]">{"(大货)"}</span></div>
                                                            <div class="text-[#86909C] text-[12px]">{"¥"}{product.sample_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())} <span class="text-xs text-[#86909C]">{"(剪样)"}</span></div>
                                                        </td>
                                                        <td class={format!("text-right {}", stock_class)}>{stock_qty}{" kg"}</td>
                                                        <td class="text-[#4E5969]">{"主仓库 - 针织A区"}</td>
                                                        <td>
                                                            <div class="flex items-center justify-center gap-2">
                                                                <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{"查看"}</button>
                                                                <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{"编辑"}</button>
                                                                <button class="text-[#165DFF] hover:text-[#0F4CD0] text-[14px]">{"打印标签"}</button>
                                                                <button class="text-[#F53F3F] hover:text-[#E03535] text-[14px]">{"删除"}</button>
                                                            </div>
                                                        </td>
                                                    </tr>
                                                }
                                            })
                                        }
                                    }
                                </tbody>
                            </table>
                        </div>
                        
                        <div class="p-4 border-t border-[#E5E6EB] flex justify-between items-center text-[14px]">
                            <div class="text-[#86909C]">{"共 "}{filtered_products.len()}{" 条记录"}</div>
                            <div class="flex items-center gap-2">
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]">{"上一页"}</button>
                                <span class="text-[#165DFF] bg-[#E8F3FF] px-3 py-1 rounded">{"1"}</span>
                                <button class="px-3 py-1 border border-[#E5E6EB] rounded text-[#4E5969] hover:bg-[#F5F7FA]">{"下一页"}</button>
                            </div>
                        </div>
                    </div>
                }
            </div>
        </MainLayout>
    }
}
