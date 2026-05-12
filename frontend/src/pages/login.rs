use yew::prelude::*;
use gloo_net::http::Request;
use serde_json::json;
use web_sys::HtmlInputElement;

#[function_component(Login)]
pub fn login() -> Html {
    let username = use_state(|| String::new());
    let password = use_state(|| String::new());
    let error = use_state(|| Option::<String>::None);
    let loading = use_state(|| false);

    let onsubmit = {
        let username = username.clone();
        let password = password.clone();
        let error = error.clone();
        let loading = loading.clone();

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if username.is_empty() || password.is_empty() {
                error.set(Some("用户名和密码不能为空".to_string()));
                return;
            }

            let username_val = (*username).clone();
            let password_val = (*password).clone();
            let error = error.clone();
            let loading = loading.clone();

            loading.set(true);
            error.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let response = Request::post("/api/auth/login")
                    .json(&json!({
                        "username": username_val,
                        "password": password_val
                    }))
                    .unwrap()
                    .send()
                    .await;

                match response {
                    Ok(resp) => {
                        if resp.ok() {
                            match resp.json::<serde_json::Value>().await {
                                Ok(data) => {
                                    if let Some(token) = data.get("token").and_then(|t| t.as_str()) {
                                        let window = web_sys::window().unwrap();
                                        let storage = window.local_storage().unwrap().unwrap();
                                        storage.set_item("token", token).unwrap();

                                        let location = window.location();
                                        let _ = location.set_href("/");
                                    } else {
                                        error.set(Some("登录响应中缺少 token".to_string()));
                                    }
                                }
                                Err(e) => {
                                    error.set(Some(format!("解析响应失败: {}", e)));
                                }
                            }
                        } else {
                            let status = resp.status();
                            let text = resp.text().await.unwrap_or_default();
                            error.set(Some(format!("登录失败 ({}): {}", status, text)));
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("网络请求失败: {}", e)));
                    }
                }

                loading.set(false);
            });
        })
    };

    let on_username_change = {
        let username = username.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            username.set(input.value());
        })
    };

    let on_password_change = {
        let password = password.clone();
        Callback::from(move |e: Event| {
            let input: HtmlInputElement = e.target_unchecked_into();
            password.set(input.value());
        })
    };

    html! {
        <div class="login-page">
            <div class="login-container">
                <div class="login-header">
                    <h1>{ "秉羲面料管理系统" }</h1>
                    <p>{ "请登录以继续" }</p>
                </div>

                <form class="login-form" {onsubmit}>
                    if let Some(err) = (*error).as_ref() {
                        <div class="error-message">
                            { err }
                        </div>
                    }

                    <div class="form-group">
                        <label for="username">{ "用户名" }</label>
                        <input
                            type="text"
                            id="username"
                            name="username"
                            placeholder="请输入用户名"
                            value={(*username).clone()}
                            onchange={on_username_change}
                            required={true}
                        />
                    </div>

                    <div class="form-group">
                        <label for="password">{ "密码" }</label>
                        <input
                            type="password"
                            id="password"
                            name="password"
                            placeholder="请输入密码"
                            value={(*password).clone()}
                            onchange={on_password_change}
                            required={true}
                        />
                    </div>

                    <button
                        type="submit"
                        class="login-button"
                        disabled={*loading}
                    >
                        if *loading {
                            <span class="loading-spinner"></span>
                            { "登录中..." }
                        } else {
                            { "登录" }
                        }
                    </button>
                </form>

                <div class="login-footer">
                    <p>{ "忘记密码？请联系管理员" }</p>
                </div>
            </div>
        </div>
    }
}
