#!/bin/bash

# Find all rust files in pages directory
FILES=$(find src/pages -name "*.rs" -not -name "mod.rs" -not -name "login.rs" -not -name "init.rs")

for file in $FILES; do
    # Skip if already has MainLayout
    if grep -q "MainLayout" "$file"; then
        continue
    fi

    echo "Processing $file"

    # Add the use statement if not present
    if ! grep -q "use crate::components::main_layout::MainLayout;" "$file"; then
        # Insert after the last use crate:: statement, or at the top of other use statements
        sed -i '0,/use crate::/s/use crate::/use crate::components::main_layout::MainLayout;\nuse crate::/' "$file"
    fi

    # Find the first html! { ... } block in view() or the function component
    # We will use awk to wrap the content inside html! { ... } with <MainLayout> ... </MainLayout>
    
    # Simple heuristic:
    # Most pages have `html! { <div class="some-page"> ... </div> }`
    # Let's write a python script to parse and wrap the outermost element inside html! { ... }
done
