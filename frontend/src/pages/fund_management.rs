// 资金管理页面

use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use crate::services::fund_management_service::FundManagementService;
use crate::services::crud_service::CrudService;
use crate::models::fund_management::{FundAccount, FundAccountQueryParams, CreateFundAccountRequest};

pub struct FundManagementPage {
    accounts: Vec<FundAccount>,
    loading: bool,
    error: Option<String>,
    show_modal: bool,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<FundAccount>),
    LoadError(String),
    OpenCreateModal,
    CloseModal,
    CreateAccount(CreateFundAccountRequest),
    DeleteAccount(i32),
}

impl Component for FundManagementPage {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            accounts: Vec::new(),
            loading: true,
            error: None,
            show_modal: false,
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            ctx.link().send_message(Msg::LoadData);
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                let link = ctx.link().clone();
                spawn_local(async move {
                    let query = FundAccountQueryParams {
                        account_type: None,
                        status: None,
                        page: Some(1),
                        page_size: Some(100),
                    };
                    match FundManagementService::list_accounts(query).await {
                        Ok(res) => link.send_message(Msg::DataLoaded(res)),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                true
            }
            Msg::DataLoaded(data) => {
                self.accounts = data;
                self.loading = false;
                true
            }
            Msg::LoadError(err) => {
                self.error = Some(err);
                self.loading = false;
                true
            }
            Msg::OpenCreateModal => {
                self.show_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_modal = false;
                true
            }
            Msg::CreateAccount(req) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FundManagementService::create_account(req).await {
                        Ok(_) => link.send_message(Msg::LoadData),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                self.show_modal = false;
                true
            }
            Msg::DeleteAccount(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match FundManagementService::delete_account(id).await {
                        Ok(_) => link.send_message(Msg::LoadData),
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div class="fund-management-page p-6">
                <div class="flex justify-between items-center mb-6">
                    <h1 class="text-2xl font-bold text-gray-800">{"资金账户管理"}</h1>
                    <button 
                        class="bg-indigo-600 text-white px-4 py-2 rounded shadow hover:bg-indigo-700"
                        onclick={ctx.link().callback(|_| Msg::OpenCreateModal)}
                    >
                        {"新建资金账户"}
                    </button>
                </div>
                
                { self.render_content(ctx) }
                
                if self.show_modal {
                    { self.render_modal(ctx) }
                }
            </div>
        }
    }
}

impl FundManagementPage {
    fn render_content(&self, ctx: &Context<Self>) -> Html {
        if self.loading {
            return html! { <div class="text-center p-10">{"加载中..."}</div> };
        }
        if let Some(err) = &self.error {
            return html! { <div class="text-red-500 p-4 bg-red-50 rounded"> { err } </div> };
        }

        html! {
            <div class="bg-white rounded-lg shadow overflow-hidden">
                <table class="min-w-full divide-y divide-gray-200">
                    <thead class="bg-gray-50">
                        <tr>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"ID"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"账户名称"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"账号"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"类型"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"币种"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"余额"}</th>
                            <th class="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">{"操作"}</th>
                        </tr>
                    </thead>
                    <tbody class="bg-white divide-y divide-gray-200">
                        { for self.accounts.iter().map(|a| {
                            let id = a.id;
                            html! {
                                <tr>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ a.id }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-900">{ &a.account_name }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ &a.account_no }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ &a.account_type }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm text-gray-500">{ &a.currency }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm font-medium text-gray-900">{ &a.balance }</td>
                                    <td class="px-6 py-4 whitespace-nowrap text-sm font-medium">
                                        <button class="text-red-600 hover:text-red-900 ml-4" onclick={ctx.link().callback(move |_| Msg::DeleteAccount(id))}>{"删除"}</button>
                                    </td>
                                </tr>
                            }
                        }) }
                    </tbody>
                </table>
            </div>
        }
    }

    fn render_modal(&self, ctx: &Context<Self>) -> Html {
        let onsubmit = ctx.link().batch_callback(|e: SubmitEvent| {
            e.prevent_default();
            let form = e.target_unchecked_into::<web_sys::HtmlFormElement>();
            
            let name = form.elements().named_item("account_name")?.unchecked_into::<web_sys::HtmlInputElement>().value();
            let no = form.elements().named_item("account_no")?.unchecked_into::<web_sys::HtmlInputElement>().value();
            let typ = form.elements().named_item("account_type")?.unchecked_into::<web_sys::HtmlInputElement>().value();
            let curr = form.elements().named_item("currency")?.unchecked_into::<web_sys::HtmlInputElement>().value();
            
            let req = CreateFundAccountRequest {
                account_name: name,
                account_no: no,
                account_type: typ,
                bank_name: None,
                currency: curr,
                opened_date: None,
                remark: None,
            };
            Some(Msg::CreateAccount(req))
        });

        html! {
            <div class="fixed inset-0 z-50 flex items-center justify-center overflow-x-hidden overflow-y-auto outline-none focus:outline-none">
                <div class="fixed inset-0 bg-gray-900 bg-opacity-50 transition-opacity" onclick={ctx.link().callback(|_| Msg::CloseModal)}></div>
                <div class="relative w-full max-w-lg mx-auto my-6 z-50">
                    <div class="relative flex flex-col w-full bg-white border-0 rounded-xl shadow-2xl outline-none focus:outline-none">
                        <div class="flex items-start justify-between p-5 border-b border-solid border-gray-200 rounded-t">
                            <h3 class="text-2xl font-semibold text-gray-800">{"新建资金账户"}</h3>
                        </div>
                        <form onsubmit={onsubmit}>
                            <div class="relative p-6 flex-auto grid grid-cols-1 gap-4">
                                <div>
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"账户名称 *"}</label>
                                    <input name="account_name" type="text" class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none" required=true />
                                </div>
                                <div>
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"账号 *"}</label>
                                    <input name="account_no" type="text" class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none" required=true />
                                </div>
                                <div>
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"类型 *"}</label>
                                    <input name="account_type" type="text" class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none" required=true />
                                </div>
                                <div>
                                    <label class="block text-gray-700 text-sm font-bold mb-2">{"币种 *"}</label>
                                    <input name="currency" type="text" class="w-full px-3 py-2 border border-gray-300 rounded focus:outline-none" value="CNY" required=true />
                                </div>
                            </div>
                            <div class="flex items-center justify-end p-6 border-t border-solid border-gray-200 rounded-b">
                                <button type="button" class="text-gray-500 font-bold px-6 py-2 outline-none mr-2 hover:bg-gray-100 rounded" onclick={ctx.link().callback(|_| Msg::CloseModal)}>{"取消"}</button>
                                <button type="submit" class="bg-indigo-600 text-white font-bold px-6 py-2 rounded shadow hover:bg-indigo-700 outline-none">{"保存"}</button>
                            </div>
                        </form>
                    </div>
                </div>
            </div>
        }
    }
}
