---
name: clone_repo
description: Clone a git repository into the workspace. Supports automatic folder creation.
---

### Usage
`[CLONE_REPO: "https://github.com/user/repo" --path="workspace/my-project"]`

### Arguments
- **repo_url**: The HTTPS URL of the git repository.
- **--path**: (Optional) The target directory inside the workspace. Defaults to `workspace/repo-clone`.
