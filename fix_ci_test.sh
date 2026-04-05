          echo "\`\`\`bash" > release_notes.md
          echo "curl -fsSL https://cdn.jsdelivr.net/gh/57231307/1@main/%E5%BF%AB%E9%80%9F%E9%83%A8%E7%BD%B2/install.sh | sudo bash -s install" >> release_notes.md
          echo "\`\`\`" >> release_notes.md
          cat release_notes.md
