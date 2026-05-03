use crate::utils::permissions;
use yew::prelude::*;
use crate::services::account_subject_service::AccountSubjectService;
use crate::services::crud_service::CrudService;
use crate::models::account_subject::{SubjectTreeNode, SubjectQueryParams};

#[function_component(AccountSubjectPage)]
pub fn account_subject_page() -> Html {
    let tree = use_state(Vec::new);
    let loading = use_state(|| false);
    let error = use_state(|| String::new());
    
    {
        let tree = tree.clone();
        let loading = loading.clone();
        let error = error.clone();
        
        use_effect_with((), move |_| {
            loading.set(true);
            wasm_bindgen_futures::spawn_local(async move {
                match AccountSubjectService::get_subject_tree().await {
                    Ok(data) => {
                        tree.set(data);
                        loading.set(false);
                    }
                    Err(e) => {
                        error.set(format!("加载失败: {}", e));
                        loading.set(false);
                    }
                }
            });
            || ()
        });
    }
    
    let on_delete = {
        let tree = tree.clone();
        let error = error.clone();
        Callback::from(move |id: i32| {
            let tree = tree.clone();
            let error = error.clone();
            wasm_bindgen_futures::spawn_local(async move {
                match AccountSubjectService::delete_subject(id).await {
                    Ok(_) => {
                        if let Ok(data) = AccountSubjectService::get_subject_tree().await {
                            tree.set(data);
                        }
                    }
                    Err(e) => {
                        error.set(format!("删除失败: {}", e));
                    }
                }
            });
        })
    };

    html! {
        <div class="account-subject-page">
            <div class="header" style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 20px;">
                <h1>{"会计科目管理"}</h1>
                <button class="btn btn-primary">{"新建科目"}</button>
            </div>
            
            if *loading {
                <div class="loading">{"加载中..."}</div>
            } else if !(*error).is_empty() {
                <div class="error" style="color: red; margin-bottom: 10px;">{ (*error).clone() }</div>
            } else {
                <div class="tree-container" style="background: #fff; padding: 20px; border-radius: 4px; box-shadow: 0 1px 3px rgba(0,0,0,0.1);">
                    <table class="table" style="width: 100%; border-collapse: collapse;">
                        <thead>
                            <tr style="background-color: #f5f5f5; text-align: left;">
                                <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"科目代码"}</th>
                                <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"科目名称"}</th>
                                <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"层级"}</th>
                                <th style="padding: 10px; border-bottom: 1px solid #ddd;">{"操作"}</th>
                            </tr>
                        </thead>
                        <tbody>
                            { for tree.iter().map(|node| render_tree_node(node, 0, on_delete.clone())) }
                        </tbody>
                    </table>
                </div>
            }
        </div>
    }
}

fn render_tree_node(node: &SubjectTreeNode, depth: usize, on_delete: Callback<i32>) -> Html {
    let padding = depth * 20;
    html! {
        <>
            <tr style="border-bottom: 1px solid #eee;">
                <td style={format!("padding: 10px; padding-left: {}px;", padding + 10)}>
                    if !node.children.is_empty() {
                        <span style="margin-right: 5px;">{"📂"}</span>
                    } else {
                        <span style="margin-right: 5px;">{"📄"}</span>
                    }
                    { &node.code }
                </td>
                <td style="padding: 10px;">{ &node.name }</td>
                <td style="padding: 10px;">{ node.level }</td>
                <td style="padding: 10px;">
                    <button class="btn btn-sm" style="margin-right: 5px;">{"编辑"}</button>
                    <button class="btn btn-sm btn-danger" onclick={{ let id = node.id; on_delete.reform(move |_| id) }}>{"删除"}</button>
                </td>
            </tr>
            { for node.children.iter().map(|child| render_tree_node(child, depth + 1, on_delete.clone())) }
        </>
    }
}
