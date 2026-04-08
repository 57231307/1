import os, re
MODELS_DIR = '/workspace/backend/src/models/'
files = [f for f in os.listdir(MODELS_DIR) if f.endswith('.rs')]
print(f"Total models: {len(files)}")
