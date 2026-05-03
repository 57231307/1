import os
import re

filepath = 'frontend/src/app/mod.rs'

with open(filepath, 'r', encoding='utf-8') as f:
    content = f.read()

# Fix the invalid Route enum
content = content.replace('''    #[not_found]
    #[at("/404")]
    #[at("/my-tasks")]
    MyTasks,
    #[not_found]
    NotFound,''', '''    #[at("/my-tasks")]
    MyTasks,
    #[not_found]
    #[at("/404")]
    NotFound,''')

with open(filepath, 'w', encoding='utf-8') as f:
    f.write(content)
