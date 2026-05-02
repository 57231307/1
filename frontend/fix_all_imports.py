import os
import glob

def fix_all_crud_imports():
    pages_dir = '/home/root0/桌面/121/1/frontend/src/pages'
    for rs_file in glob.glob(os.path.join(pages_dir, '*.rs')):
        with open(rs_file, 'r') as f:
            content = f.read()
        if "use crate::services::crud_service::CrudService;" not in content:
            lines = content.split('\n')
            for i, line in enumerate(lines):
                if "use crate::services::" in line:
                    lines.insert(i+1, "use crate::services::crud_service::CrudService;")
                    break
            with open(rs_file, 'w') as f:
                f.write('\n'.join(lines))

if __name__ == "__main__":
    fix_all_crud_imports()
    print("Fixed imports in pages.")
