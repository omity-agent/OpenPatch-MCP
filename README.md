OpenPatch 用于向 MCP 客户端提供文件编辑功能。

暴露两个工具：

- `apply_patch`：编辑文件。
- `undo_patch`：撤回补丁。输入 Patch 的 UUID。

输入格式：

- `--style=general`（默认）：`apply_patch` 接收 `path`、`old_string`、`new_string`。
- `--style=openai`：`apply_patch` 接收 `patch`，语法与 OpenAI Apply Patch 工具相同。

OpenPatch 与 OpenAI Apply Patch 工具存在一定差异，包括：

- 更激进的模糊匹配。
- 允许部分成功。
- 经过重新设计的 XML-Like 输出格式。
- `undo_patch` 工具和 UUID 机制。

意在提供更高效和强大的文件编辑能力。

为实现近期历史记录撤回功能，该工具会在本地保存一个数据库。
