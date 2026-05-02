use gloo_dialogs;
use crate::components::main_layout::MainLayout;
use yew::prelude::*;

#[function_component(SystemSettingsPage)]
pub fn system_settings_page() -> Html {
    let active_tab = use_state(|| "company".to_string());
    let allow_negative_stock = use_state(|| true);
    let enable_credit_risk = use_state(|| true);

    let render_company_settings = || html! {
        <div class="bg-white p-6 rounded-lg shadow-sm border border-gray-100">
            <h3 class="text-lg font-medium text-gray-900 mb-4">{"公司基本信息"}</h3>
            <div class="space-y-4 max-w-2xl">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">{"公司名称"}</label>
                    <input type="text" value="秉羲面料管理" class="w-full border border-gray-300 rounded-md px-3 py-2 focus:ring-indigo-500 focus:border-indigo-500" />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">{"联系电话"}</label>
                    <input type="text" placeholder="请输入公司联系电话" class="w-full border border-gray-300 rounded-md px-3 py-2 focus:ring-indigo-500 focus:border-indigo-500" />
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">{"公司地址"}</label>
                    <textarea rows="3" placeholder="请输入公司详细地址" class="w-full border border-gray-300 rounded-md px-3 py-2 focus:ring-indigo-500 focus:border-indigo-500"></textarea>
                </div>
                <div class="pt-4">
                    <button onclick={Callback::from(|_| gloo_dialogs::alert("保存成功"))} class="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500">
                        {"保存设置"}
                    </button>
                </div>
            </div>
        </div>
    };

    let render_dict_settings = || html! {
        <div class="bg-white p-6 rounded-lg shadow-sm border border-gray-100">
            <div class="flex justify-between items-center mb-4">
                <h3 class="text-lg font-medium text-gray-900">{"数据字典配置"}</h3>
                
            </div>
            <div class="overflow-x-auto">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"字典类型"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"字典名称"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase tracking-wider">{"状态"}</th>
                            <th class="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase tracking-wider">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200">
                        <tr>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{"customer_tier"}</td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{"客户等级"}</td>
                            <td class="px-6 py-4 whitespace-nowrap">
                                <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800">{"启用"}</span>
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                <span>{"- "}</span>
                            </td>
                        </tr>
                        <tr>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{"order_status"}</td>
                            <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{"订单状态"}</td>
                            <td class="px-6 py-4 whitespace-nowrap">
                                <span class="px-2 inline-flex text-xs leading-5 font-semibold rounded-full bg-green-100 text-green-800">{"启用"}</span>
                            </td>
                            <td class="px-6 py-4 whitespace-nowrap text-right text-sm font-medium">
                                <span>{"- "}</span>
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    };

    let render_system_params = || html! {
        <div class="bg-white p-6 rounded-lg shadow-sm border border-gray-100">
            <h3 class="text-lg font-medium text-gray-900 mb-4">{"系统运行参数"}</h3>
            <div class="space-y-4 max-w-2xl">
                <div class="flex items-center justify-between">
                    <div>
                        <h4 class="text-sm font-medium text-gray-900">{"允许负库存发货"}</h4>
                        <p class="text-xs text-gray-500">{"开启后，在面料二批扫码发货时若无库存将自动生成负库存记录"}</p>
                    </div>
                                        <button onclick={{
                        let allow_negative_stock = allow_negative_stock.clone();
                        Callback::from(move |_| allow_negative_stock.set(!*allow_negative_stock))
                    }} class={format!("{} relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500", if *allow_negative_stock { "bg-indigo-600" } else { "bg-gray-200" })}>
                        <span class={format!("{} pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200", if *allow_negative_stock { "translate-x-5" } else { "translate-x-0" })}></span>
                    </button>
                </div>
                <div class="flex items-center justify-between pt-4 border-t border-gray-200">
                    <div>
                        <h4 class="text-sm font-medium text-gray-900">{"开启客户信用风控"}</h4>
                        <p class="text-xs text-gray-500">{"开单时若订单金额超过客户信用额度，将强制拦截"}</p>
                    </div>
                                        <button onclick={{
                        let enable_credit_risk = enable_credit_risk.clone();
                        Callback::from(move |_| enable_credit_risk.set(!*enable_credit_risk))
                    }} class={format!("{} relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500", if *enable_credit_risk { "bg-indigo-600" } else { "bg-gray-200" })}>
                        <span class={format!("{} pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200", if *enable_credit_risk { "translate-x-5" } else { "translate-x-0" })}></span>
                    </button>
                </div>
            </div>
        </div>
    };

    html! {
        <MainLayout current_page="系统设置">
            <div class="px-4 sm:px-6 lg:px-8 py-8">
                <div class="sm:flex sm:items-center">
                    <div class="sm:flex-auto">
                        <h1 class="text-2xl font-semibold text-gray-900">{"系统设置"}</h1>
                        <p class="mt-2 text-sm text-gray-700">{"管理企业信息、数据字典与全局业务参数"}</p>
                    </div>
                </div>
                
                <div class="mt-6 border-b border-gray-200">
                    <nav class="-mb-px flex space-x-8">
                        <button 
                            onclick={let active_tab = active_tab.clone(); Callback::from(move |_| active_tab.set("company".to_string()))}
                            class={if *active_tab == "company" {
                                "border-indigo-500 text-indigo-600 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            }}>
                            {"公司信息"}
                        </button>
                        <button 
                            onclick={let active_tab = active_tab.clone(); Callback::from(move |_| active_tab.set("dict".to_string()))}
                            class={if *active_tab == "dict" {
                                "border-indigo-500 text-indigo-600 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            }}>
                            {"数据字典"}
                        </button>
                        <button 
                            onclick={let active_tab = active_tab.clone(); Callback::from(move |_| active_tab.set("params".to_string()))}
                            class={if *active_tab == "params" {
                                "border-indigo-500 text-indigo-600 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            } else {
                                "border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300 whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm"
                            }}>
                            {"业务参数"}
                        </button>
                    </nav>
                </div>
                
                <div class="mt-6">
                    {match active_tab.as_str() {
                        "company" => render_company_settings(),
                        "dict" => render_dict_settings(),
                        "params" => render_system_params(),
                        _ => html! { <div></div> }
                    }}
                </div>
            </div>
        </MainLayout>
    }
}
