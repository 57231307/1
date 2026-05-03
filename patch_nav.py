import os
import re

filepath = 'frontend/src/components/navigation.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Add on_my_tasks
if 'let on_my_tasks =' not in content:
    content = content.replace('let on_dashboard = {', 
    '''let on_my_tasks = {
        let navigator = navigator.clone();
        Callback::from(move |_| {
            if let Some(nav) = &navigator {
                nav.push(&Route::MyTasks);
            }
        })
    };
    
    let on_dashboard = {''')

# Add to menu
if 'render_item("我的待办"' not in content:
    content = content.replace('{render_item("首页", "dashboard", on_dashboard)}',
    '{render_item("首页", "dashboard", on_dashboard)}\n                    {render_item("我的待办", "my-tasks", on_my_tasks)}')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)
