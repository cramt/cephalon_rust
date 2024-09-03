release VERSION:
    cargo set-version {{ VERSION }}
    git tag {{ VERSION }}
    git push --tags
