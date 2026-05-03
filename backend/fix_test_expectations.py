import re
import glob

test_files = glob.glob('tests/*.rs') + glob.glob('src/services/tests/*.rs') + glob.glob('src/services/*test*.rs')

for filepath in test_files:
    with open(filepath, 'r') as f:
        content = f.read()
    
    # replace unwrap() with expect() in tests
    content = re.sub(r'\.unwrap\(\)', '.expect("操作应该成功")', content)
    
    with open(filepath, 'w') as f:
        f.write(content)
    
    print(f"Fixed: {filepath}")
