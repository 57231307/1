//! 会计科目管理页面

use crate::components::main_layout::MainLayout;
use crate::models::account_subject::{AccountSubject, SubjectQueryParams};
use crate::services::account_subject_service::AccountSubjectService;
use yew::prelude::*;

#[function_component(AccountSubjectPage)]
pub fn account_subject_page() -> Html {
    let subjects = use_state(|| Vec::<AccountSubject>::new());

    {
        let subjects = subjects.clone();
        use_effect_with((), move |_| {
            wasm_bindgen_futures::spawn_local(async move {
                let params = SubjectQueryParams {
                    level: None,
                    parent_id: None,
                    status: None,
                    keyword: None,
                };
                if let Ok(res) = AccountSubjectService::list_subjects(params).await {
                    subjects.set(res);
                }
            });
            || ()
        });
    }

    html! {
        <MainLayout current_page={"account_subject"}>
<div class="account-subject-page p-4">
            <div class="header mb-4">
                <h1 class="text-2xl font-bold">{"会计科目管理"}</h1>
            </div>
            <div class="content">
                <div class="table-responsive">
                    <table class="data-table w-full">
                        <thead>
                            <tr>
                                <th>{"ID"}</th>
                                <th>{"代码"}</th>
                                <th>{"名称"}</th>
                                <th>{"级别"}</th>
                                <th>{"状态"}</th>
                                <th>{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            {
                                if subjects.is_empty() {
                                    html! {
                                        <tr><td colspan="6" class="text-center py-4">{"暂无数据"}</td></tr>
                                    }
                                } else {
                                    html! {
                                        for subjects.iter().map(|subject| html! {
                                            <tr key={subject.id}>
                                                <td>{ subject.id }</td>
                                                <td>{ &subject.code }</td>
                                                <td>{ &subject.name }</td>
                                                <td>{ subject.level }</td>
                                                <td>
                                                    <span class="status-badge">{ &subject.status }</span>
                                                </td>
                                                <td>
                                                    <button class="text-blue-500 hover:text-blue-700">{"查看"}</button>
                                                </td>
                                            </tr>
                                        })
                                    }
                                }
                            }
                        </tbody>
                    </table>
                </div>
            </div>
        </div>
    
</MainLayout>}
}
