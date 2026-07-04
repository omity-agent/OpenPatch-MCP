# apply_patch 输入输出案例

本文档覆盖 `apply_patch` 工具的主要成功与失败场景。所有案例都假设工具调用参数形如：

```json
{
  "cwd": "C:/work/example",
  "patch": "..."
}
```

`cwd` 可以省略，省略时使用 MCP server 进程当前目录。patch 内的文件路径可以是相对路径或绝对路径；相对路径会基于 `cwd` 解析。

返回文本统一为：

```text
exit_code: <0 或 1>
stdout:
<成功应用的文件摘要>
stderr:
<失败信息>
```

如果 `exit_code` 为 `1`，MCP tool result 会被标记为 error；如果为 `0`，会被标记为 success。

## 1. 新增文件

初始文件：

```text
不存在 hello.txt
```

输入：

```text
*** Begin Patch
*** Add File: hello.txt
+hello
+world
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
A hello.txt

stderr:
```

最终文件：

```text
hello.txt
---------
hello
world
```

## 2. 新增空文件

输入：

```text
*** Begin Patch
*** Add File: empty.txt
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
A empty.txt

stderr:
```

最终文件：

```text
empty.txt 为空文件
```

## 3. 新增嵌套目录中的文件

输入：

```text
*** Begin Patch
*** Add File: docs/example.txt
+created with parent directories
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
A docs/example.txt

stderr:
```

结果：如果 `docs` 目录不存在，会自动创建父目录。

## 4. 更新文件

初始文件：

```text
target.txt
----------
old
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

最终文件：

```text
target.txt
----------
new
```

## 5. 带上下文定位的更新

初始文件：

```text
target.txt
----------
alpha
anchor
old
omega
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@ anchor
-old
+new
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

最终文件：

```text
target.txt
----------
alpha
anchor
new
omega
```

## 6. 单个文件内多个更新块

初始文件：

```text
target.txt
----------
one
two
three
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-one
+1
@@
-three
+3
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

最终文件：

```text
target.txt
----------
1
two
3
```

## 6.1. 单个文件内部分更新块失败

初始文件：

```text
target.txt
----------
one
two
three
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-one
+1
@@
-missing
+changed
@@
-three
+3
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:
M target.txt

stderr:
Failed to find expected lines in C:/work/example/target.txt:
missing
```

最终文件：

```text
target.txt
----------
1
two
3
```

## 7. 只插入内容

初始文件：

```text
target.txt
----------
alpha
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
+beta
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

最终文件：

```text
target.txt
----------
alpha
beta
```

## 8. 删除文件

初始文件：

```text
obsolete.txt 存在
```

输入：

```text
*** Begin Patch
*** Delete File: obsolete.txt
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
D obsolete.txt

stderr:
```

最终结果：`obsolete.txt` 被删除。

## 9. 重命名文件

初始文件：

```text
old-name.txt
------------
content
```

输入：

```text
*** Begin Patch
*** Update File: old-name.txt
*** Move to: new-name.txt
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M new-name.txt

stderr:
```

最终结果：`old-name.txt` 被删除，`new-name.txt` 包含原内容。

## 10. 重命名并更新文件

初始文件：

```text
old-name.txt
------------
old
```

输入：

```text
*** Begin Patch
*** Update File: old-name.txt
*** Move to: new-name.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M new-name.txt

stderr:
```

最终文件：

```text
new-name.txt
------------
new
```

## 11. 一次成功编辑多个文件

初始文件：

```text
a.txt: old
c.txt: old
b.txt 不存在
```

输入：

```text
*** Begin Patch
*** Update File: a.txt
@@
-old
+new
*** Add File: b.txt
+created
*** Update File: c.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
A b.txt
M a.txt
M c.txt

stderr:
```

最终文件：

```text
a.txt: new
b.txt: created
c.txt: new
```

## 12. 多文件中间失败，后续文件继续执行

初始文件：

```text
a.txt: old
b.txt: kept
c.txt: old
```

输入：

```text
*** Begin Patch
*** Update File: a.txt
@@
-old
+new
*** Update File: b.txt
@@
-missing
+changed
*** Update File: c.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:
M a.txt
M c.txt

stderr:
Failed to find expected lines in C:/work/example/b.txt:
missing
```

最终文件：

```text
a.txt: new
b.txt: kept
c.txt: new
```

说明：失败会被记录，但不会阻止后续文件继续处理。

## 13. 多文件中多个失败

初始文件：

```text
a.txt: kept
b.txt: old
c.txt: kept
```

输入：

```text
*** Begin Patch
*** Update File: a.txt
@@
-missing-a
+new-a
*** Update File: b.txt
@@
-old
+new
*** Update File: c.txt
@@
-missing-c
+new-c
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:
M b.txt

stderr:
Failed to find expected lines in C:/work/example/a.txt:
missing-a
Failed to find expected lines in C:/work/example/c.txt:
missing-c
```

最终文件：

```text
a.txt: kept
b.txt: new
c.txt: kept
```

## 14. 空 patch 参数

输入参数：

```json
{
  "patch": "",
  "cwd": "C:/work/example"
}
```

输出：

```text
patch must not be empty
```

说明：这个场景在 MCP 请求层直接失败，不会进入 patch runner，因此没有 `exit_code` 包装。

## 15. cwd 不存在或不是目录

输入参数：

```json
{
  "cwd": "C:/work/missing-directory",
  "patch": "*** Begin Patch\n*** End Patch"
}
```

输出：

```text
cwd is not a directory: C:/work/missing-directory
```

说明：这是 invalid params 错误，不会进入 patch runner。

## 16. patch 缺少 Begin 标记

输入：

```text
*** Update File: target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch: The first line of the patch must be '*** Begin Patch'
```

## 17. patch 缺少 End 标记

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-old
+new
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch: The last line of the patch must be '*** End Patch'
```

## 18. patch 中没有文件操作

输入：

```text
*** Begin Patch
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
No files were modified.
```

## 19. 未知文件操作标记

输入：

```text
*** Begin Patch
*** Rename File: a.txt
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 2: expected file operation marker
```

## 20. Add File 内容行缺少 `+`

输入：

```text
*** Begin Patch
*** Add File: target.txt
hello
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 3: add file lines must start with '+'
```

## 21. Update File 缺少变更块

输入：

```text
*** Begin Patch
*** Update File: target.txt
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 2: update file hunk has no changes
```

说明：如果 `Update File` 后面带有 `*** Move to: ...`，则允许没有变更块，表示只重命名。

## 22. Update File 缺少 `@@` 变更标记

输入：

```text
*** Begin Patch
*** Update File: target.txt
-old
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 3: expected '@@' change marker
```

## 23. Update File 变更行缺少前缀

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
old
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 4: expected change line prefix
```

## 24. 更新的旧内容不匹配

初始文件：

```text
target.txt
----------
actual
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-expected
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:

stderr:
Failed to find expected lines in C:/work/example/target.txt:
expected
```

最终文件保持不变：

```text
target.txt
----------
actual
```

## 25. 上下文行不匹配

初始文件：

```text
target.txt
----------
alpha
old
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@ missing-anchor
-old
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:

stderr:
Failed to find context 'missing-anchor' in C:/work/example/target.txt
```

## 26. 更新不存在的文件

输入：

```text
*** Begin Patch
*** Update File: missing.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:

stderr:
Failed to read file to update C:/work/example/missing.txt: 系统找不到指定的文件。 (os error 2)
```

说明：系统错误文本会随操作系统语言而变化。

## 27. 删除不存在的文件

输入：

```text
*** Begin Patch
*** Delete File: missing.txt
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:

stderr:
Failed to delete file C:/work/example/missing.txt: 系统找不到指定的文件。 (os error 2)
```

说明：系统错误文本会随操作系统语言而变化。

## 28. 删除目录

初始状态：

```text
target 是目录
```

输入：

```text
*** Begin Patch
*** Delete File: target
*** End Patch
```

输出：

```text
exit_code: 1
stdout:
Updated the following files:

stderr:
Failed to delete file C:/work/example/target: path is a directory
```

## 29. heredoc 包装输入

输入：

```text
<<'EOF'
*** Begin Patch
*** Update File: target.txt
@@
-old
+new
*** End Patch
EOF
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

说明：外层 heredoc 包装会被识别并剥离。

## 30. 匹配时允许的宽松规则

查找待替换旧内容时，会依次尝试：

1. 完全匹配；
2. 忽略行尾空白；
3. 忽略行首和行尾空白；
4. 规范化部分 Unicode 标点与空白后再匹配。

初始文件：

```text
target.txt
----------
  old   
```

输入：

```text
*** Begin Patch
*** Update File: target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
M target.txt

stderr:
```

最终文件：

```text
target.txt
----------
new
```

## 31. 路径变量展开

环境变量：

```text
PATCH_DIR=docs
PATCH_FILE=example.txt
```

输入：

```text
*** Begin Patch
*** Add File: $PATCH_DIR/%PATCH_FILE%
+hello
*** End Patch
```

输出：

```text
exit_code: 0
stdout:
Success. Updated the following files:
A docs/example.txt

stderr:
```

说明：路径支持 Unix 风格 `$VAR`、`${VAR}`，Windows 风格 `%VAR%`，以及位于路径开头的 `~`。

## 32. 路径变量不存在

输入：

```text
*** Begin Patch
*** Add File: $MISSING_FILE
+hello
*** End Patch
```

输出：

```text
exit_code: 1
stdout:

stderr:
Invalid patch hunk on line 2: environment variable 'MISSING_FILE' is not set in path '$MISSING_FILE'
```
