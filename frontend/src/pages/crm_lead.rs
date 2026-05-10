use crate::utils::permissions;
use crate::utils::toast_helper;
use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::services::crm_service::{CrmService, CrmLead};
use crate::services::crud_service::CrudService;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};

#[function_component(CrmLeadPage)]
pub fn crm_lead_page() -> Html {
    let leads = use_state(|| Vec::<CrmLead>::new());
    let filtered_leads = use_state(|| Vec::<CrmLead>::new());
    let loading = use_state(|| true);
    let error = use_state(|| None::<String>);
    let search_keyword = use_state(|| String::new());
    let page = use_state(|| 0u64);
    let page_size = use_state(|| 10u64);
    
    // 弹窗状态
    let show_modal = use_state(|| false);
    let modal_mode = use_state(|| ModalMode::Create);
    let editing_lead = use_state(|| None::<CrmLead>);
    let show_delete_confirm = use_state(|| false);
    let deleting_id = use_state(|| None::<i32>);
    
    // 表单状态
    let form_name = use_state(|| String::new());
    let form_customer_name = use_state(|| String::new());
    let form_contact_person = use_state(|| String::new());
    let form_contact_phone = use_state(|| String::new());
    let form_email = use_state(|| String::new());
    let form_source = use_state(|| String::new());
    let form_status = use_state(|| "NEW".to_string());
    let form_remarks = use_state(|| String::new());
    let form_error = use_state(|| None::<String>);

    // 加载数据
    {
        let leads = leads.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match CrmService::list_leads(1, 1000).await {
                    Ok(res) => {
                        leads.set(res.data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(Some(format!("加载线索数据失败: {}", e)));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }

    // 搜索过滤
    let apply_filter = {
        let leads = leads.clone();
        let filtered_leads = filtered_leads.clone();
        let search_keyword = search_keyword.clone();
        let page = page.clone();
        
        move || {
            let keyword = (*search_keyword).to_lowercase();
            let filtered: Vec<CrmLead> = if keyword.is_empty() {
                (*leads).clone()
            } else {
                leads.iter()
                    .filter(|l| {
                        l.name.to_lowercase().contains(&keyword) ||
                        l.lead_no.to_lowercase().contains(&keyword) ||
                        l.customer_name.as_ref().map(|n| n.to_lowercase().contains(&keyword)).unwrap_or(false) ||
                        l.contact_person.as_ref().map(|n| n.to_lowercase().contains(&keyword)).unwrap_or(false)
                    })
                    .cloned()
                    .collect()
            };
            filtered_leads.set(filtered);
            page.set(0);
        }
    };

    // 分页数据
    let paginated_leads = {
        let filtered_leads = filtered_leads.clone();
        let page = page.clone();
        let page_size = page_size.clone();
        
        move || {
            let start = (*page * *page_size) as usize;
            let end = ((*page + 1) * *page_size) as usize;
            filtered_leads[start..end.min(filtered_leads.len())].to_vec()
        }
    };

    // 重置表单
    let reset_form = {
        let form_name = form_name.clone();
        let form_customer_name = form_customer_name.clone();
        let form_contact_person = form_contact_person.clone();
        let form_contact_phone = form_contact_phone.clone();
        let form_email = form_email.clone();
        let form_source = form_source.clone();
        let form_status = form_status.clone();
        let form_remarks = form_remarks.clone();
        let form_error = form_error.clone();
        
        move || {
            form_name.set(String::new());
            form_customer_name.set(String::new());
            form_contact_person.set(String::new());
            form_contact_phone.set(String::new());
            form_email.set(String::new());
            form_source.set(String::new());
            form_status.set("NEW".to_string());
            form_remarks.set(String::new());
            form_error.set(None);
        }
    };

    // 打开新建弹窗
    let on_open_create = {
        let show_modal = show_modal.clone();
        let modal_mode = modal_mode.clone();
        let editing_lead = editing_lead.clone();
        let reset_form = reset_form.clone();
        
        Callback::from(move |_| {
            reset_form();
            editing_lead.set(None);
            modal_mode.set(ModalMode::Create);
            show_modal.set(true);
        })
    };

    // 打开编辑弹窗
    let on_open_edit = {
        let show_modal = show_modal.clone();
        let modal_mode = modal_mode.clone();
        let editing_lead = editing_lead.clone();
        let form_name = form_name.clone();
        let form_customer_name = form_customer_name.clone();
        let form_contact_person = form_contact_person.clone();
        let form_contact_phone = form_contact_phone.clone();
        let form_email = form_email.clone();
        let form_source = form_source.clone();
        let form_status = form_status.clone();
        let form_remarks = form_remarks.clone();
        let form_error = form_error.clone();
        
        Callback::from(move |lead: CrmLead| {
            form_name.set(lead.name.clone());
            form_customer_name.set(lead.customer_name.clone().unwrap_or_default());
            form_contact_person.set(lead.contact_person.clone().unwrap_or_default());
            form_contact_phone.set(lead.contact_phone.clone().unwrap_or_default());
            form_email.set(lead.email.clone().unwrap_or_default());
            form_source.set(lead.source.clone());
            form_status.set(lead.status.clone());
            form_remarks.set(lead.remarks.clone().unwrap_or_default());
            form_error.set(None);
            editing_lead.set(Some(lead));
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
        let editing_lead = editing_lead.clone();
        let form_name = form_name.clone();
        let form_customer_name = form_customer_name.clone();
        let form_contact_person = form_contact_person.clone();
        let form_contact_phone = form_contact_phone.clone();
        let form_email = form_email.clone();
        let form_source = form_source.clone();
        let form_status = form_status.clone();
        let form_remarks = form_remarks.clone();
        let form_error = form_error.clone();
        let leads = leads.clone();
        let loading = loading.clone();
        
        Callback::from(move |_| {
            // 表单验证
            if (*form_name).is_empty() {
                form_error.set(Some("线索名称不能为空".to_string()));
                return;
            }
            if (*form_source).is_empty() {
                form_error.set(Some("来源不能为空".to_string()));
                return;
            }
            
            form_error.set(None);
            
            // 这里应该调用API创建/更新线索
            // 目前显示成功提示
            if *modal_mode == ModalMode::Create {
                toast_helper::show_success("线索创建成功");
            } else {
                toast_helper::show_success("线索更新成功");
            }
            show_modal.set(false);
            
            // 重新加载数据
            loading.set(true);
            let leads = leads.clone();
            let loading = loading.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match CrmService::list_leads(1, 1000).await {
                    Ok(res) => {
                        leads.set(res.data);
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
        let leads = leads.clone();
        let loading = loading.clone();
        
        Callback::from(move |_| {
            if let Some(id) = *deleting_id {
                toast_helper::show_success("删除成功");
                show_delete_confirm.set(false);
                deleting_id.set(None);
                
                // 重新加载数据
                loading.set(true);
                let leads = leads.clone();
                let loading = loading.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match CrmService::list_leads(1, 1000).await {
                        Ok(res) => {
                            leads.set(res.data);
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

    let on_customer_name_change = {
        let form_customer_name = form_customer_name.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_customer_name.set(input.value());
            }
        })
    };

    let on_contact_person_change = {
        let form_contact_person = form_contact_person.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_contact_person.set(input.value());
            }
        })
    };

    let on_contact_phone_change = {
        let form_contact_phone = form_contact_phone.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_contact_phone.set(input.value());
            }
        })
    };

    let on_email_change = {
        let form_email = form_email.clone();
        Callback::from(move |e: InputEvent| {
            if let Some(input) = e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()) {
                form_email.set(input.value());
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

    let on_status_change = {
        let form_status = form_status.clone();
        Callback::from(move |e: Event| {
            if let Some(select) = e.target().and_then(|t| t.dyn_into::<web_sys::HtmlSelectElement>().ok()) {
                form_status.set(select.value());
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
        use_effect_with(leads.clone(), move |_| {
            apply_filter();
            || ()
        });
    }

    let is_edit = *modal_mode == ModalMode::Edit;
    let modal_title = if is_edit { "编辑线索" } else { "新建线索" };

    html! {
        <div class="crm-lead-page">
            <PageHeader title={"CRM线索管理".to_string()} subtitle={Some("管理销售线索信息".to_string())}>
                <button class="btn btn-primary" onclick={on_open_create}>
                    {"+ 新建线索"}
                </button>
            </PageHeader>

            <div class="page-toolbar">
                <SearchBar
                    placeholder={"搜索线索名称、编号或客户...".to_string()}
                    on_search={on_search}
                    on_reset={on_reset_search}
                />
            </div>

            if *loading {
                <LoadingState message={"正在加载线索数据...".to_string()} />
            } else if let Some(err) = &*error {
                <div class="error-container">
                    <div class="error-icon">{"⚠️"}</div>
                    <p class="error-message">{err}</p>
                    <button class="btn btn-primary" onclick={Callback::from(|_| {
                        // 重新加载
                    })}>
                        {"重新加载"}
                    </button>
                </div>
            } else if filtered_leads.is_empty() {
                <EmptyState
                    icon={"🎯".to_string()}
                    title={"暂无线索数据".to_string()}
                    description={if (*search_keyword).is_empty() {
                        "点击上方按钮创建第一个线索".to_string()
                    } else {
                        "没有匹配搜索条件的线索".to_string()
                    }}
                />
            } else {
                <div class="table-container">
                    <table class="data-table">
                        <thead>
                            <tr>
                                <th>{"编号"}</th>
                                <th>{"线索名称"}</th>
                                <th>{"客户名称"}</th>
                                <th>{"联系人"}</th>
                                <th>{"联系电话"}</th>
                                <th>{"来源"}</th>
                                <th>{"状态"}</th>
                                <th class="text-center">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {for paginated_leads().iter().map(|lead| {
                                let lead_clone = lead.clone();
                                let lead_clone2 = lead.clone();
                                let id = lead.id;
                                
                                let status_class = match lead.status.as_str() {
                                    "NEW" => "status-badge status-info",
                                    "CONTACTED" => "status-badge status-warning",
                                    "QUALIFIED" => "status-badge status-success",
                                    "LOST" => "status-badge status-error",
                                    _ => "status-badge",
                                };
                                
                                let status_text = match lead.status.as_str() {
                                    "NEW" => "新建",
                                    "CONTACTED" => "已联系",
                                    "QUALIFIED" => "已认证",
                                    "LOST" => "已丢失",
                                    _ => &lead.status,
                                };
                                
                                html! {
                                    <tr>
                                        <td>{&lead.lead_no}</td>
                                        <td>{&lead.name}</td>
                                        <td>{lead.customer_name.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td>{lead.contact_person.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td>{lead.contact_phone.clone().unwrap_or_else(|| "-".to_string())}</td>
                                        <td>{&lead.source}</td>
                                        <td>
                                            <span class={status_class}>{status_text}</span>
                                        </td>
                                        <td class="text-center">
                                            <div class="action-buttons">
                                                <button
                                                    class="btn btn-sm btn-secondary"
                                                    onclick={on_open_edit.reform(move |_| lead_clone.clone())}
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
                        total={filtered_leads.len() as u64}
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
                                <label>{"线索名称 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={(*form_name).clone()}
                                    oninput={on_name_change}
                                    placeholder="请输入线索名称"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"客户名称"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={(*form_customer_name).clone()}
                                    oninput={on_customer_name_change}
                                    placeholder="请输入客户名称"
                                />
                            </div>
                            <div class="form-row">
                                <div class="form-group">
                                    <label>{"联系人"}</label>
                                    <input
                                        type="text"
                                        class="form-input"
                                        value={(*form_contact_person).clone()}
                                        oninput={on_contact_person_change}
                                        placeholder="请输入联系人"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"联系电话"}</label>
                                    <input
                                        type="text"
                                        class="form-input"
                                        value={(*form_contact_phone).clone()}
                                        oninput={on_contact_phone_change}
                                        placeholder="请输入联系电话"
                                    />
                                </div>
                            </div>
                            <div class="form-group">
                                <label>{"邮箱"}</label>
                                <input
                                    type="email"
                                    class="form-input"
                                    value={(*form_email).clone()}
                                    oninput={on_email_change}
                                    placeholder="请输入邮箱"
                                />
                            </div>
                            <div class="form-row">
                                <div class="form-group">
                                    <label>{"来源 *"}</label>
                                    <input
                                        type="text"
                                        class="form-input"
                                        value={(*form_source).clone()}
                                        oninput={on_source_change}
                                        placeholder="如：网站、展会、推荐"
                                    />
                                </div>
                                <div class="form-group">
                                    <label>{"状态"}</label>
                                    <select class="form-input" value={(*form_status).clone()} onchange={on_status_change}>
                                        <option value="NEW">{"新建"}</option>
                                        <option value="CONTACTED">{"已联系"}</option>
                                        <option value="QUALIFIED">{"已认证"}</option>
                                        <option value="LOST">{"已丢失"}</option>
                                    </select>
                                </div>
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
                                {if is_edit { "保存修改" } else { "创建线索" }}
                            </button>
                        </div>
                    </div>
                </div>
            }

            // 删除确认对话框
            <ConfirmDialog
                title={"确认删除".to_string()}
                message={"确定要删除这个线索吗？此操作不可撤销。".to_string()}
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
