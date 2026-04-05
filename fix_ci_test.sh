          echo "\`\`\`bash" > release_notes.md
          echo "curl -fsSL https://raw.githubusercontent.com/57231307/1/main/install.sh | sudo bash -s install" >> release_notes.md
          echo "\`\`\`" >> release_notes.md
          cat release_notes.md
