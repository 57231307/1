import os

filepath = 'frontend/src/app/mod.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Add MyTasks to enum Route
if 'MyTasks,' not in content:
    content = content.replace('    NotFound,\n}', '    #[at("/my-tasks")]\n    MyTasks,\n    #[not_found]\n    NotFound,\n}')

# Add MyTasksPage to switch
if 'Route::MyTasks =>' not in content:
    content = content.replace('Route::NotFound =>', 'Route::MyTasks => protected_route(|| html! { <crate::pages::my_tasks::MyTasksPage /> }),\n        Route::NotFound =>')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)
