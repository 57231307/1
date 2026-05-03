with open("/home/root0/桌面/121/1/frontend/src/app/mod.rs", "r") as f:
    content = f.read()

# Add import
if "use crate::utils::permissions;" not in content:
    content = content.replace("use crate::utils::storage::Storage;", "use crate::utils::storage::Storage;\nuse crate::utils::permissions;")

new_fn = """fn protected_route_with_permission<F>(component: F, resource: &str, action: &str) -> Html
where
    F: FnOnce() -> Html,
{
    if Storage::get_token().is_some() {
        if permissions::has_permission(resource, action) {
            component()
        } else {
            html! {
                <div class="error-page" style="padding: 20px; text-align: center;">
                    <h1>{"无权访问"}</h1>
                    <p>{"您没有权限访问此页面"}</p>
                </div>
            }
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn protected_route<F>(component: F) -> Html"""

content = content.replace("fn protected_route<F>(component: F) -> Html", new_fn)

# Change Sales to use protected_route_with_permission
content = content.replace("Route::Sales => protected_route(|| html! { <SalesOrderPage /> }),", "Route::Sales => protected_route_with_permission(|| html! { <SalesOrderPage /> }, \"sales_order\", \"read\"),")

with open("/home/root0/桌面/121/1/frontend/src/app/mod.rs", "w") as f:
    f.write(content)
