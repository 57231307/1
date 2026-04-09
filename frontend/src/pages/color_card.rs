use crate::components::main_layout::MainLayout;
use crate::services::api::ApiService;
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use chrono::{DateTime, Utc, Local};

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct ColorCardResponse {
    pub id: i32,
    pub card_no: String,
    pub product_id: i32,
    pub season: Option<String>,
    pub description: Option<String>,
    pub stock_quantity: Option<i32>,
    pub created_at: String,
}

#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct ColorCardRecordResponse {
    pub id: i32,
    pub color_card_id: i32,
    pub customer_id: i32,
    pub issue_date: String,
    pub return_date: Option<String>,
    pub status: Option<String>,
    pub notes: Option<String>,
}

#[function_component(ColorCardPage)]
pub fn color_card_page() -> Html {
    let cards = use_state(|| Vec::<ColorCardResponse>::new());
    let records = use_state(|| Vec::<ColorCardRecordResponse>::new());
    let active_tab = use_state(|| "cards".to_string());
    
    {
        let cards = cards.clone();
        let records = records.clone();
        use_effect_with((), move |_| {
            spawn_local(async move {
                if let Ok(res) = ApiService::get::<Vec<ColorCardResponse>>("/api/v1/erp/color-cards").await {
                    cards.set(res);
                }
                if let Ok(res) = ApiService::get::<Vec<ColorCardRecordResponse>>("/api/v1/erp/color-cards/records").await {
                    records.set(res);
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

    html! {
        <MainLayout current_page="色卡管理">
            <div class="flex flex-col gap-4">
                <div class="flex justify-between items-center">
                    <h1 class="text-2xl font-bold text-[#1D2129]">{"色卡管理"}</h1>
                    <div class="flex gap-2">
                        <button class="btn-primary shrink-0">{"新增色卡"}</button>
                        <button class="btn-secondary shrink-0">{"登记发放"}</button>
                    </div>
                </div>

                <div class="card p-0">
                    <div class="flex border-b border-[#E5E6EB] px-4">
                        <button 
                            onclick={let on_tab_change = on_tab_change.clone(); Callback::from(move |_| on_tab_change.emit("cards"))}
                            class={format!("py-3 px-4 font-bold border-b-2 transition-colors {}", if *active_tab == "cards" { "border-[#165DFF] text-[#165DFF]" } else { "border-transparent text-[#4E5969]" })}>
                            {"色卡主数据"}
                        </button>
                        <button 
                            onclick={let on_tab_change = on_tab_change.clone(); Callback::from(move |_| on_tab_change.emit("records"))}
                            class={format!("py-3 px-4 font-bold border-b-2 transition-colors {}", if *active_tab == "records" { "border-[#165DFF] text-[#165DFF]" } else { "border-transparent text-[#4E5969]" })}>
                            {"发放与追踪记录"}
                        </button>
                    </div>
                    
                    <div class="p-4 table-responsive">
                        if *active_tab == "cards" {
                            if cards.is_empty() {
                                <div class="py-12 flex flex-col items-center justify-center text-[#86909C]">
                                    <svg class="w-12 h-12 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 002-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path></svg>
                                    {"暂无色卡数据"}
                                </div>
                            } else {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>{"色卡编号"}</th>
                                            <th>{"关联产品ID"}</th>
                                            <th>{"适用季节"}</th>
                                            <th>{"描述"}</th>
                                            <th>{"当前库存"}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        { for cards.iter().map(|card| html! {
                                            <tr key={card.id}>
                                                <td class="font-bold text-[#1D2129]">{&card.card_no}</td>
                                                <td>{card.product_id}</td>
                                                <td>{card.season.as_deref().unwrap_or("-")}</td>
                                                <td>{card.description.as_deref().unwrap_or("-")}</td>
                                                <td class={if card.stock_quantity.unwrap_or(0) < 5 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129]" }}>{card.stock_quantity.unwrap_or(0)}</td>
                                            </tr>
                                        }) }
                                    </tbody>
                                </table>
                            }
                        } else {
                            if records.is_empty() {
                                <div class="py-12 flex flex-col items-center justify-center text-[#86909C]">
                                    <svg class="w-12 h-12 mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"></path></svg>
                                    {"暂无发放记录"}
                                </div>
                            } else {
                                <table class="data-table">
                                    <thead>
                                        <tr>
                                            <th>{"记录ID"}</th>
                                            <th>{"色卡ID"}</th>
                                            <th>{"客户ID"}</th>
                                            <th>{"发放时间"}</th>
                                            <th>{"状态"}</th>
                                            <th>{"备注"}</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                        { for records.iter().map(|record| html! {
                                            <tr key={record.id}>
                                                <td>{record.id}</td>
                                                <td>{record.color_card_id}</td>
                                                <td>{record.customer_id}</td>
                                                <td>{record.issue_date.split('T').next().unwrap_or("")}</td>
                                                <td>
                                                    <span class={if record.status.as_deref() == Some("ISSUED") { "px-2 py-0.5 rounded text-[12px] bg-orange-100 text-orange-800" } else { "px-2 py-0.5 rounded text-[12px] bg-green-100 text-green-800" }}>
                                                        {record.status.as_deref().unwrap_or("-")}
                                                    </span>
                                                </td>
                                                <td>{record.notes.as_deref().unwrap_or("-")}</td>
                                            </tr>
                                        }) }
                                    </tbody>
                                </table>
                            }
                        }
                    </div>
                </div>
            </div>
        </MainLayout>
    }
}
