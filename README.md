OpenPatch 用于向 MCP 客户端提供文件编辑功能。

只适用于 GPT 系列模型。

暴露两个工具：

- `apply_patch`：应用补丁。输入语法与 OpenAI Apply Patch 工具完全相同。
- `undo_patch`：撤回补丁。输入 Patch 的 UUID。

OpenPatch 与 OpenAI Apply Patch 工具存在一定差异，包括：

- 更激进的模糊匹配。
- 允许部分成功。
- 经过重新设计的 XML-Like 输出格式。
- `undo_patch` 工具和 UUID 机制。

意在提供更高效和强大的文件编辑能力。

为实现近期历史记录撤回功能，该工具会在本地保存一个数据库。