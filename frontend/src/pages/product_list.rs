use yew::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use crate::components::{
    confirm_dialog::ConfirmDialog,
    search_bar::SearchBar,
    pagination::Pagination,
    page_header::PageHeader,
    empty_state::EmptyState,
    loading_state::LoadingState,
};
use crate::models::product::{Product, CreateProductRequest, UpdateProductRequest};
use crate::services::product_service::ProductService;
use crate::services::crud_service::CrudService;
use crate::utils::toast_helper;

#[allow(unused_imports)]
use crate::services::crud_service::CrudService as _;

pub struct ProductListPage {
    products: Vec<Product>,
    filtered_products: Vec<Product>,
    total: u64,
    page: u64,
    page_size: u64,
    loading: bool,
    error: Option<String>,
    search_keyword: String,
    show_create_modal: bool,
    show_edit_modal: bool,
    editing_product: Option<Product>,
    show_delete_confirm: bool,
    deleting_id: Option<i32>,
    form_name: String,
    form_code: String,
    form_unit: String,
    form_price: String,
    form_description: String,
    form_category_id: String,
    form_error: Option<String>,
}

pub enum Msg {
    LoadData,
    DataLoaded(Vec<Product>),
    Error(String),
    Search(String),
    ResetSearch,
    PageChanged(u64),
    OpenCreateModal,
    CloseModal,
    OpenEditModal(Product),
    OpenDeleteConfirm(i32),
    CancelDelete,
    ConfirmDelete,
    Deleted,
    FormNameChanged(String),
    FormCodeChanged(String),
    FormUnitChanged(String),
    FormPriceChanged(String),
    FormDescriptionChanged(String),
    FormCategoryIdChanged(String),
    SubmitForm,
    FormSubmitted,
}

impl Component for ProductListPage {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        ctx.link().send_message(Msg::LoadData);
        Self {
            products: Vec::new(),
            filtered_products: Vec::new(),
            total: 0,
            page: 0,
            page_size: 10,
            loading: true,
            error: None,
            search_keyword: String::new(),
            show_create_modal: false,
            show_edit_modal: false,
            editing_product: None,
            show_delete_confirm: false,
            deleting_id: None,
            form_name: String::new(),
            form_code: String::new(),
            form_unit: String::new(),
            form_price: String::new(),
            form_description: String::new(),
            form_category_id: String::new(),
            form_error: None,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::LoadData => {
                self.loading = true;
                self.error = None;
                let link = ctx.link().clone();
                spawn_local(async move {
                    match ProductService::list().await {
                        Ok(data) => link.send_message(Msg::DataLoaded(data)),
                        Err(e) => link.send_message(Msg::Error(e)),
                    }
                });
                false
            }
            Msg::DataLoaded(products) => {
                self.loading = false;
                self.products = products.clone();
                self.apply_filter();
                true
            }
            Msg::Error(e) => {
                self.loading = false;
                self.error = Some(e);
                true
            }
            Msg::Search(keyword) => {
                self.search_keyword = keyword;
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::ResetSearch => {
                self.search_keyword = String::new();
                self.page = 0;
                self.apply_filter();
                true
            }
            Msg::PageChanged(page) => {
                self.page = page;
                true
            }
            Msg::OpenCreateModal => {
                self.reset_form();
                self.show_create_modal = true;
                true
            }
            Msg::CloseModal => {
                self.show_create_modal = false;
                self.show_edit_modal = false;
                self.editing_product = None;
                self.form_error = None;
                true
            }
            Msg::OpenEditModal(product) => {
                self.form_name = product.name.clone();
                self.form_code = product.code.clone();
                self.form_unit = product.unit.clone();
                self.form_price = product.price.clone().unwrap_or_default();
                self.form_description = product.description.clone().unwrap_or_default();
                self.form_category_id = product.category_id.map(|id| id.to_string()).unwrap_or_default();
                self.editing_product = Some(product);
                self.show_edit_modal = true;
                true
            }
            Msg::OpenDeleteConfirm(id) => {
                self.deleting_id = Some(id);
                self.show_delete_confirm = true;
                true
            }
            Msg::CancelDelete => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                true
            }
            Msg::ConfirmDelete => {
                if let Some(id) = self.deleting_id {
                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match ProductService::delete(id).await {
                            Ok(_) => {
                                toast_helper::show_success("删除成功");
                                link.send_message(Msg::Deleted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("删除失败: {}", e));
                                link.send_message(Msg::CancelDelete);
                            }
                        }
                    });
                }
                false
            }
            Msg::Deleted => {
                self.show_delete_confirm = false;
                self.deleting_id = None;
                ctx.link().send_message(Msg::LoadData);
                false
            }
            Msg::FormNameChanged(v) => { self.form_name = v; true }
            Msg::FormCodeChanged(v) => { self.form_code = v; true }
            Msg::FormUnitChanged(v) => { self.form_unit = v; true }
            Msg::FormPriceChanged(v) => { self.form_price = v; true }
            Msg::FormDescriptionChanged(v) => { self.form_description = v; true }
            Msg::FormCategoryIdChanged(v) => { self.form_category_id = v; true }
            Msg::SubmitForm => {
                if self.form_name.is_empty() {
                    self.form_error = Some("产品名称不能为空".to_string());
                    return true;
                }
                if self.form_code.is_empty() {
                    self.form_error = Some("产品编码不能为空".to_string());
                    return true;
                }
                if self.form_unit.is_empty() {
                    self.form_error = Some("单位不能为空".to_string());
                    return true;
                }

                let name = self.form_name.clone();
                let code = self.form_code.clone();
                let unit = self.form_unit.clone();
                let price = if self.form_price.is_empty() { None } else { Some(self.form_price.clone()) };
                let description = if self.form_description.is_empty() { None } else { Some(self.form_description.clone()) };
                let category_id = self.form_category_id.parse().ok();

                let link = ctx.link().clone();

                if self.show_edit_modal {
                    if let Some(product) = &self.editing_product {
                        let id = product.id;
                        spawn_local(async move {
                            let req = UpdateProductRequest {
                                name: Some(name),
                                code: Some(code),
                                unit: Some(unit),
                                price,
                                description,
                                category_id,
                            };
                            match ProductService::update(id, req).await {
                                Ok(_) => {
                                    toast_helper::show_success("更新成功");
                                    link.send_message(Msg::FormSubmitted);
                                }
                                Err(e) => {
                                    toast_helper::show_error(&format!("更新失败: {}", e));
                                }
                            }
                        });
                    }
                } else {
                    spawn_local(async move {
                        let req = CreateProductRequest {
                            name,
                            code,
                            unit,
                            price,
                            description,
                            category_id,
                        };
                        match ProductService::create(req).await {
                            Ok(_) => {
                                toast_helper::show_success("创建成功");
                                link.send_message(Msg::FormSubmitted);
                            }
                            Err(e) => {
                                toast_helper::show_error(&format!("创建失败: {}", e));
                            }
                        }
                    });
                }
                false
            }
            Msg::FormSubmitted => {
                self.show_create_modal = false;
                self.show_edit_modal = false;
                self.editing_product = None;
                self.reset_form();
                ctx.link().send_message(Msg::LoadData);
                false
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();

        html! {
            <div class="product-list-page">
                <PageHeader title={"产品管理".to_string()} subtitle={Some("管理所有产品信息".to_string())}>
                    <button class="btn btn-primary" onclick={link.callback(|_| Msg::OpenCreateModal)}>
                        {"+ 新建产品"}
                    </button>
                </PageHeader>

                <div class="page-toolbar">
                    <SearchBar
                        placeholder={"搜索产品名称或编码...".to_string()}
                        on_search={link.callback(|keyword| Msg::Search(keyword))}
                        on_reset={link.callback(|_| Msg::ResetSearch)}
                    />
                </div>

                if self.loading {
                    <LoadingState message={"正在加载产品数据...".to_string()} />
                } else if let Some(error) = &self.error {
                    <div class="error-container">
                        <div class="error-icon">{"⚠️"}</div>
                        <p class="error-message">{error}</p>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::LoadData)}>
                            {"重新加载"}
                        </button>
                    </div>
                } else if self.filtered_products.is_empty() {
                    <EmptyState
                        icon={"📦".to_string()}
                        title={"暂无产品数据".to_string()}
                        description={if self.search_keyword.is_empty() {
                            "点击上方按钮创建第一个产品".to_string()
                        } else {
                            "没有匹配搜索条件的产品".to_string()
                        }}
                    />
                } else {
                    <div class="table-container">
                        <table class="data-table">
                            <thead>
                                <tr>
                                    <th>{"ID"}</th>
                                    <th>{"产品编码"}</th>
                                    <th>{"产品名称"}</th>
                                    <th>{"类别"}</th>
                                    <th>{"单位"}</th>
                                    <th>{"价格"}</th>
                                    <th class="text-center">{"操作"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {for self.paginated_products().iter().map(|product| {
                                    let product_clone = product.clone();
                                    let product_clone2 = product.clone();
                                    let id = product.id;
                                    html! {
                                        <tr>
                                            <td>{product.id}</td>
                                            <td>{&product.code}</td>
                                            <td>{&product.name}</td>
                                            <td>{product.category_id.map(|id| id.to_string()).unwrap_or_else(|| "-".to_string())}</td>
                                            <td>{&product.unit}</td>
                                            <td class="numeric">{product.price.clone().unwrap_or_else(|| "-".to_string())}</td>
                                            <td class="text-center">
                                                <div class="action-buttons">
                                                    <button
                                                        class="btn btn-sm btn-secondary"
                                                        onclick={link.callback(move |_| Msg::OpenEditModal(product_clone.clone()))}
                                                    >
                                                        {"编辑"}
                                                    </button>
                                                    <button
                                                        class="btn btn-sm btn-danger"
                                                        onclick={link.callback(move |_| Msg::OpenDeleteConfirm(id))}
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
                            current_page={self.page}
                            page_size={self.page_size}
                            total={self.filtered_products.len() as u64}
                            on_page_change={link.callback(|page| Msg::PageChanged(page))}
                        />
                    </div>
                }

                // 新建/编辑弹窗
                if self.show_create_modal || self.show_edit_modal {
                    {self.render_form_modal(ctx)}
                }

                // 删除确认对话框
                <ConfirmDialog
                    title={"确认删除".to_string()}
                    message={"确定要删除这个产品吗？此操作不可撤销。".to_string()}
                    confirm_text={"删除".to_string()}
                    cancel_text={"取消".to_string()}
                    confirm_class={"btn-danger".to_string()}
                    on_confirm={link.callback(|_| Msg::ConfirmDelete)}
                    on_cancel={link.callback(|_| Msg::CancelDelete)}
                    visible={self.show_delete_confirm}
                />
            </div>
        }
    }
}

impl ProductListPage {
    fn apply_filter(&mut self) {
        if self.search_keyword.is_empty() {
            self.filtered_products = self.products.clone();
        } else {
            let keyword = self.search_keyword.to_lowercase();
            self.filtered_products = self.products.iter()
                .filter(|p| {
                    p.name.to_lowercase().contains(&keyword) ||
                    p.code.to_lowercase().contains(&keyword)
                })
                .cloned()
                .collect();
        }
        self.total = self.filtered_products.len() as u64;
    }

    fn paginated_products(&self) -> Vec<Product> {
        let start = (self.page * self.page_size) as usize;
        let end = ((self.page + 1) * self.page_size) as usize;
        self.filtered_products[start..end.min(self.filtered_products.len())].to_vec()
    }

    fn reset_form(&mut self) {
        self.form_name = String::new();
        self.form_code = String::new();
        self.form_unit = String::new();
        self.form_price = String::new();
        self.form_description = String::new();
        self.form_category_id = String::new();
        self.form_error = None;
    }

    fn render_form_modal(&self, ctx: &Context<Self>) -> Html {
        let link = ctx.link();
        let is_edit = self.show_edit_modal;
        let title = if is_edit { "编辑产品" } else { "新建产品" };

        let on_name_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormNameChanged(input.value()))
        });
        let on_code_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCodeChanged(input.value()))
        });
        let on_unit_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormUnitChanged(input.value()))
        });
        let on_price_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormPriceChanged(input.value()))
        });
        let on_desc_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormDescriptionChanged(input.value()))
        });
        let on_category_change = link.batch_callback(|e: InputEvent| {
            e.target().and_then(|t| t.dyn_into::<HtmlInputElement>().ok()).map(|input| Msg::FormCategoryIdChanged(input.value()))
        });

        html! {
            <div class="modal-overlay" onclick={link.callback(|_| Msg::CloseModal)}>
                <div class="modal-content" onclick={Callback::from(|e: MouseEvent| e.stop_propagation())}>
                    <div class="modal-header">
                        <h3>{title}</h3>
                        <button class="close-btn" onclick={link.callback(|_| Msg::CloseModal)}>{"×"}</button>
                    </div>
                    <div class="modal-body">
                        if let Some(error) = &self.form_error {
                            <div class="form-error">{error}</div>
                        }
                        <div class="form-group">
                            <label>{"产品名称 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_name.clone()}
                                oninput={on_name_change}
                                placeholder="请输入产品名称"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"产品编码 *"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_code.clone()}
                                oninput={on_code_change}
                                placeholder="请输入产品编码"
                            />
                        </div>
                        <div class="form-row">
                            <div class="form-group">
                                <label>{"单位 *"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_unit.clone()}
                                    oninput={on_unit_change}
                                    placeholder="如：米、千克"
                                />
                            </div>
                            <div class="form-group">
                                <label>{"价格"}</label>
                                <input
                                    type="text"
                                    class="form-input"
                                    value={self.form_price.clone()}
                                    oninput={on_price_change}
                                    placeholder="请输入价格"
                                />
                            </div>
                        </div>
                        <div class="form-group">
                            <label>{"类别ID"}</label>
                            <input
                                type="text"
                                class="form-input"
                                value={self.form_category_id.clone()}
                                oninput={on_category_change}
                                placeholder="请输入类别ID"
                            />
                        </div>
                        <div class="form-group">
                            <label>{"描述"}</label>
                            <textarea
                                class="form-input"
                                value={self.form_description.clone()}
                                oninput={on_desc_change}
                                placeholder="请输入产品描述"
                                rows="3"
                            />
                        </div>
                    </div>
                    <div class="modal-footer">
                        <button class="btn btn-secondary" onclick={link.callback(|_| Msg::CloseModal)}>
                            {"取消"}
                        </button>
                        <button class="btn btn-primary" onclick={link.callback(|_| Msg::SubmitForm)}>
                            {if is_edit { "保存修改" } else { "创建产品" }}
                        </button>
                    </div>
                </div>
            </div>
        }
    }
}
