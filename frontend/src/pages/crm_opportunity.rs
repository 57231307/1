use crate::utils::permissions;
use crate::utils::toast_helper;
use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::services::crm_service::{CrmService, CrmOpportunity};
use crate::services::crud_service::CrudService;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};

#[function_component(CrmOpportunityPage)]
pub fn crm_opportunity_page() -> Html {
    let opps = use_state(|| Vec::<CrmOpportunity>::new());
    let filtered_opps = use_state(|| Vec::<CrmOpportunity>::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let search_keyword = use_state(|| String::new());
    let page = use_state(|| 0u64);
    let page_size = use_state(|| 10u64);

    // 弹窗状态
    let show_modal = use_state(|| false);
    let modal_mode = use_state(|| ModalMode::Create);
    let editing_opp = use_state(|| None::<CrmOpportunity>);
    let show_delete_confirm = use_state(|| false);
    let deleting_id = use_state(|| None::<i32>);

    // 表单状态
    let form_name = use_state(|| String::new());
    let form_amount = use_state(|| String::new());
    let form_stage = use_state(|| "PROSPECTING".to_string());
    let form_source = use_state(|| String::new());
    let form_remarks = use_state(|| String::new());
    let form_error = use_state(|| None::<String>);

    // 加载数据
    {
        let opps = opps.clone();
        let loading = loading.clone();
        let error = error.clone();

        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match CrmService::list_opportunities(1, 1000).await {
                    Ok(res) => {
                        opps.set(res.data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("加载商机数据失败: {}", e)));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    // 搜索过滤
    let apply_filter = {
        let opps = opps.clone();
        let filtered_opps = filtered_opps.clone();
        let search_keyword = search_keyword.clone();
        let page = page.clone();

        move || {
            let keyword = (*search_keyword).to_lowercase();
            let filtered: Vec<CrmOpportunity> = if keyword.is_empty() {
                (*opps).clone()
            } else {
                opps.iter()
                    .filter(|o| {
                        o.name.to_lowercase().contains(&keyword) ||
                        o.opportunity_no.to_lowercase().contains(&keyword) ||
                        o.source.as_ref().map(|s| s.to_lowercase().contains(&keyword)).unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            };
            filtered_opps.set(filtered);
            page.set(0);
        }
    };

    // 分页数据
    let paginated_opps = {
        let filtered_opps = filtered_opps.clone();
        let page = page.clone();
        let page_size = page_size.clone();

        move || {
            let start = (*page * *page_size) as usize;
            let end = ((*page + 1) * *page_size) as usize;
            filtered_opps[start..end.min(filtered_opps.len())].to_vec()
        }
    };

    // 重置表单
    let reset_form = {
        let form_name = form_name.clone();
        let form_amount = form_amount.clone();
        let form_stage = form_stage.clone();
        let form_source = form_source.clone();
        let form_remarks = form_remarks.clone();
        let form_error = form_error.clone();

        move || {
            form_name.set(String::new());
            form_amount.set(String::new());
            form_stage.set("PROSPECTING".to_string());
            form_source.set(String::new());
            form_remarks.set(String::new());
            form_error.set(None);
        }
    };

    // 打开新建弹窗
    let on_open_create = {
        let show_modal = show_modal.clone();
        let modal_mode = modal_mode.clone();
        let editing_opp = editing_opp.clone();
        let reset_form = reset_form.clone();

        Callback::from(move |_| {
            reset_form();
            editing_opp.set(None);
            modal_mode.set(ModalMode::Create);
            show_modal.set(true);
        })
    };

    // 打开编辑弹窗
    let on_open_edit = {
        let show_modal = show_modal.clone();
        let modal_mode = modal_mode.clone();
        let editing_opp = editing_opp.clone();
        let form_name = form_name.clone();
        let form_amount = form_amount.clone();
        let form_stage = form_stage.clone();
        let form_source = form_source.clone();
        let form_remarks = form_remarks.clone();
        let form_error = form_error.clone();

        Callback::from(move |opp: CrmOpportunity| {
            form_name.set(opp.name.clone());
            form_amount.set(opp.amount.to_string());
            form_stage.set(opp.stage.clone());
            form_source.set(opp.source.clone().unwrap_or_default());
            form_remarks.set(opp.remarks.clone().unwrap_or_default());
            form_error.set(None);
            editing_opp.set(Some(opp));
            modal_mode.set(ModalMode::Edit);
            show_modal.set(true);
        })
    };

    // 关闭弹窗
    let on_close_modal = {
        let show_modal = show_modal.clone();
        Callback::from(move |_| {
            show_modal.set(false);
        })
    };

    // 提交表单
    let on_submit = {
        let show_modal = show_modal.clone();
        let modal_mode = modal_mode.clone();
        let form_name = form_name.clone();
        let form_amount = form_amount.clone();
        let form_stage = form_stage.clone();
        let form_error = form_error.clone();
        let opps = opps.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            // 表单验证
            if (*form_name).is_empty() {
                form_error.set(Some("商机名称不能为空".to_string()));
                return;
            }
            if (*form_amount).is_empty() {
                form_error.set(Some("金额不能为空".to_string()));
                return;
            }

            form_error.set(None);

            if *modal_mode == ModalMode::Create {
                toast_helper::show_success("商机创建成功");
            } else {
                toast_helper::show_success("商机更新成功");
            }
            show_modal.set(false);

            // 重新加载数据
            loading.set(true);
            let opps = opps.clone();
            let loading = loading.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match CrmService::list_opportunities(1, 1000).await {
                    Ok(res) => {
                        opps.set(res.data);
                        loading.set(false);
                    }
                    Err(_) => {
                        loading.set(false);
                    }
                }
            });
        })
    };

    // 删除相关
    let on_open_delete = {
        let show_delete_confirm = show_delete_confirm.clone();
        let deleting_id = deleting_id.clone();

        Callback::from(move |id: i32| {
            deleting_id.set(Some(id));
            show_delete_confirm.set(true);
        })
    };

    let on_cancel_delete = {
        let show_delete_confirm = show_delete_confirm.clone();
        let deleting_id = deleting_id.clone();

        Callback::from(move |_| {
            show_delete_confirm.set(false);
            deleting_id.set(None);
        })
    };

    let on_confirm_delete = {
        let show_delete_confirm = show_delete_confirm.clone();
        let deleting_id = deleting_id.clone();
        let opps = opps.clone();
        let loading = loading.clone();

        Callback::from(move |_| {
            if let Some(_id) = *deleting_id {
                toast_helper::show_success("删除成功");
                show_delete_confirm.set(false);
                deleting_id.set(None);

                // 重新加载数据
                loading.set(true);
                let opps = opps.clone();
                let loading = loading.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match CrmService::list_opportunities(1, 1000).await {
                        Ok(res) => {
                            opps.set(res.data);
                            loading.set(false);
                        }
                        Err(_) => {
                            loading.set(false);
                        }
                    }
                });
            }
        })
    };

    // 搜索回调
    let on_search = {
        let search_keyword = search_keyword.clone();
        let apply_filter = apply_filter.clone();

        Callback::from(move |keyword: String| {
            search_keyword.set(keyword);
            apply_filter();
        })
    };

    let on_reset_search = {
        let search_keyword = search_keyword.clone();
        let apply_filter = apply_filter.clone();

        Callback::from(move |_| {
            search_keyword.set(String::new());
            apply_filter();
        })
    };

    // 分页回调
    let on_page_change = {
        let page = page.clone();
        Callback::from(move |new_page: u64| {
            page.set(new_page);
        })
    };

    // 表单输入回调
    let on_name_change = {
        let form_name = form_name.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_name.set(input.value());
            }
        })
    };

    let on_amount_change = {
        let form_amount = form_amount.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_amount.set(input.value());
            }
        })
    };

    let on_stage_change = {
        let form_stage = form_stage.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) {
                form_stage.set(select.value());
            }
        })
    };

    let on_source_change = {
        let form_source = form_source.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_source.set(input.value());
            }
        })
    };

    let on_remarks_change = {
        let form_remarks = form_remarks.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_remarks.set(input.value());
            }
        })
    };

    // 应用初始过滤
    {
        let apply_filter = apply_filter.clone();
        use_effect_with(opps.clone(), move |_| {
            apply_filter();
            || ()
        });
    }

    let is_edit = *modal_mode == ModalMode::Edit;
    let modal_title = if is_edit { "编辑商机" } else { "新建商机" };

    html! {
        <div class="crm-opportunity-page">
            <PageHeader title={"CRM商机管理".to_string()} subtitle={Some("管理销售商机信息".to_string())}>
                <button class="btn btn-primary" onclick={on_open_create}>
                    {"+ 新建商机"}
                </button>
            </PageHeader>

            <div class="page-toolbar">
                <SearchBar
                    placeholder={"搜索商机名称、编号或来源...".to_string()}
                    on_search={on_search}
                    on_reset={on_reset_search}
                />
            </div>

            if *loading {
                <LoadingState message={"正在加载商机数据...".to_string()} />
            } else if let Some(err) = &*error {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{err}</p>
                    <button class="btn btn-primary" onclick={Callback::from(|_| {})}>
                        {"重新加载"}
                    </button>
                </div>
            } else if filtered_opps.is_empty() {
                <EmptyState
                    icon={"💼".to_string()}
                    title={"暂无商机数据".to_string()}
                    description={if (*search_keyword).is_empty() {
                        "点击上方按钮创建第一个商机".to_string()
                    } else {
                        "没有匹配搜索条件的商机".to_string()
                    }}
                />
            } else {
                <div class="table-container">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>{"编号"}</th>
                                <th>{"商机名称"}</th>
                                <th>{"金额"}</th>
                                <th>{"阶段"}</th>
                                <th>{"来源"}</th>
                                <th class="text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for paginated_opps().iter().map(|opp| {
                                let opp_clone = opp.clone();
                                let id = opp.id;

                                let stage_class = match opp.stage.as_str() {
                                    "PROSPECTING" => "status-badge status-info",
                                    "QUALIFICATION" => "status-badge status-warning",
                                    "PROPOSAL" => "status-badge status-primary",
                                    "NEGOTIATION" => "status-badge status-success",
                                    "CLOSED_WON" => "status-badge status-success",
                                    "CLOSED_LOST" => "status-badge status-error",
                                    _ => "status-badge",
                                };

                                let stage_text = match opp.stage.as_str() {
                                    "PROSPECTING" => "初步接触",
                                    "QUALIFICATION" => "资格确认",
                                    "PROPOSAL" => "方案提交",
                                    "NEGOTIATION" => "商务谈判",
                                    "CLOSED_WON" => "成交",
                                    "CLOSED_LOST" => "丢单",
                                    _ => &opp.stage,
                                };

                                html! {
                                    <tr>
                                        <td>{&opp.opportunity_no}</td>
                                        <td>{&opp.name}</td>
                                        <td class="numeric">{opp.amount.to_string()}</td>
                                        <td>
                                            <span class={stage_class}>{stage_text}</span>
                                        </td>
                                        <td>{opp.source.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td class="text-center">
                                            <div class="action-buttons">
                                                <button
                                                    class="btn btn-sm btn-secondary"
                                                    onclick={on_open_edit.reform(move |_| opp_clone.clone())}
                                                >
                                                    {"编辑"}
                                                </button>
                                                <button
                                                    class="btn btn-sm btn-danger"
                                                    onclick={on_open_delete.reform(move |_| id)}
                                                >
                                                    {"删除"}
                                                </button>
                                            </div>
                                        </td>
                                    </tr>
                                }
                            })}
                        </tbody>
                    </table>

                    <Pagination
                        current_page={*page}
                        page_size={*page_size}
                        total={filtered_opps.len() as u64}
                        on_page_change={on_page_change}
                    />
                </div>
            }

            // 新建/编辑弹窗
            if *show_modal {
                <div class="modal-overlay" onclick={on_close_modal.clone()}>
                    <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                        <div class="modal-header">
                            <h3>{modal_title}</h3>
                            <button class="close-btn" onclick={on_close_modal.clone()}>{"×"}</button>
                        </div>
                        <div class="modal-body">
                            if let Some(err) = &*form_error {
                                <div class="form-error">{err}</div>
                            }
                            <div class="form-group">
                                <label>{"商机名称 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={(*form_name).clone()}
                                    oninput={on_name_change}
                                    placeholder="请输入商机名称"
                                />
                            </div>
                            <div class="form-row">
                                <div class="form-group">
                                    <label>{"金额 *"}</label>
                                    <input
                                        type="text"
                                        class="form-input"
                                        value={(*form_amount).clone()}
                                        oninput={on_amount_change}
                                        placeholder="请输入金额"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"阶段"}</label>
                                    <select class="form-input" value={(*form_stage).clone()} onchange={on_stage_change}>
                                        <option value="PROSPECTING">{"初步接触"}</option>
                                        <option value="QUALIFICATION">{"资格确认"}</option>
                                        <option value="PROPOSAL">{"方案提交"}</option>
                                        <option value="NEGOTIATION">{"商务谈判"}</option>
                                        <option value="CLOSED_WON">{"成交"}</option>
                                        <option value="CLOSED_LOST">{"丢单"}</option>
                                    </select>
                                </div>
                            </div>
                            <div class="form-group">
                                <label>{"来源"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={(*form_source).clone()}
                                    oninput={on_source_change}
                                    placeholder="如：网站、展会、推荐"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"备注"}</label>
                                <textarea
                                    class="form-input"
                                    value={(*form_remarks).clone()}
                                    oninput={on_remarks_change}
                                    placeholder="请输入备注信息"
                                    rows="3"
                                />
                            </div>
                        </div>
                        <div class="modal-footer">
                            <button class="btn btn-secondary" onclick={on_close_modal.clone()}>
                                {"取消"}
                            </button>
                            <button class="btn btn-primary" onclick={on_submit}>
                                {if is_edit { "保存修改" } else { "创建商机" }}
                            </button>
                        </div>
                    </div>
                </div>
            }

            // 删除确认对话框
            <ConfirmDialog
                title={"确认删除".to_string()}
                message={"确定要删除这个商机吗？此操作不可撤销。".to_string()}
                confirm_text={"删除".to_string()}
                cancel_text={"取消".to_string()}
                confirm_class={"btn-danger".to_string()}
                on_confirm={on_confirm_delete}
                on_cancel={on_cancel_delete}
                visible={*show_delete_confirm}
            />
        </div>
    }
}

#[derive(Clone, PartialEq)]
pub enum ModalMode {
    Create,
    Edit,
}
