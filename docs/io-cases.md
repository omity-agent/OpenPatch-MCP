# `apply_patch`

返回文本由成功与失败两个可选分区组成。存在成功项时输出 `<SUCCEEDED>`，存在失败项时输出 `<FAILED>`；部分成功的结果会同时包含两个分区。成功操作按原 patch 中的顺序排列。

成功的新增、编辑和删除分别使用 `<ADD>`、`<EDIT>` 和 `<DELETE>` 块。每个成功块都在路径后包含一个的 `<UUID>` 块。新增仅包含修改后的统计，编辑同时包含修改前后的统计，删除仅包含修改前的统计。能归属文件操作的失败使用对应的操作块，并在路径后放置 `<REASON>`；解析错误等全局失败在 `<FAILED>` 下放置 `<REASON>`。

这种格式不是标准 XML，路径、原因等内容不会转义。MCP tool result 的 `is_error` 由结果中是否存在失败项决定。

## 1. 新增文件

初始文件：

```text
不存在 hello.txt
```

输入：

```text
*** Begin Patch
*** Add File: C:/work/example/hello.txt
+hello
+world
*** End Patch
```

输出：

```text
<SUCCEEDED>
<ADD>
C:/work/example/hello.txt
<UUID>
019d0000-0000-7000-8000-000000000001
</UUID>
after: 2 lines, 12 chars
</ADD>
</SUCCEEDED>
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
*** Add File: C:/work/example/empty.txt
*** End Patch
```

输出：

```text
<SUCCEEDED>
<ADD>
C:/work/example/empty.txt
<UUID>
019d0000-0000-7000-8000-000000000002
</UUID>
after: 0 lines, 0 chars
</ADD>
</SUCCEEDED>
```

最终文件：

```text
empty.txt 为空文件
```

## 3. 新增嵌套目录中的文件

输入：

```text
*** Begin Patch
*** Add File: C:/work/example/docs/example.txt
+created with parent directories
*** End Patch
```

输出：

```text
<SUCCEEDED>
<ADD>
C:/work/example/docs/example.txt
<UUID>
019d0000-0000-7000-8000-000000000003
</UUID>
after: 1 lines, 32 chars
</ADD>
</SUCCEEDED>
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
*** Update File: C:/work/example/target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000004
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/target.txt
@@ anchor
-old
+new
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000005
</UUID>
before: 4 lines, 23 chars
after: 4 lines, 23 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/target.txt
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
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000006
</UUID>
before: 3 lines, 14 chars
after: 3 lines, 8 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/target.txt
@@
-one
+1
@@
-twx
+changed
@@
-three
+3
*** End Patch
```

输出：

````text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000007
</UUID>
before: 3 lines, 14 chars
after: 3 lines, 8 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find expected lines. Closest match:
```
two
```
</REASON>
</EDIT>
</FAILED>
````

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
*** Update File: C:/work/example/target.txt
@@
+beta
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000008
</UUID>
before: 1 lines, 6 chars
after: 2 lines, 11 chars
</EDIT>
</SUCCEEDED>
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
obsolete.txt
------------
obsolete
```

输入：

```text
*** Begin Patch
*** Delete File: C:/work/example/obsolete.txt
*** End Patch
```

输出：

```text
<SUCCEEDED>
<DELETE>
C:/work/example/obsolete.txt
<UUID>
019d0000-0000-7000-8000-000000000009
</UUID>
before: 1 lines, 9 chars
</DELETE>
</SUCCEEDED>
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
*** Update File: C:/work/example/old-name.txt
*** Move to: C:/work/example/new-name.txt
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/new-name.txt
<UUID>
019d0000-0000-7000-8000-00000000000a
</UUID>
before: 1 lines, 8 chars
after: 1 lines, 8 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/old-name.txt
*** Move to: C:/work/example/new-name.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/new-name.txt
<UUID>
019d0000-0000-7000-8000-00000000000b
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/a.txt
@@
-old
+new
*** Add File: C:/work/example/b.txt
+created
*** Update File: C:/work/example/c.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/a.txt
<UUID>
019d0000-0000-7000-8000-00000000000c
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
<ADD>
C:/work/example/b.txt
<UUID>
019d0000-0000-7000-8000-00000000000d
</UUID>
after: 1 lines, 8 chars
</ADD>
<EDIT>
C:/work/example/c.txt
<UUID>
019d0000-0000-7000-8000-00000000000e
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
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
*** Update File: C:/work/example/a.txt
@@
-old
+new
*** Update File: C:/work/example/b.txt
@@
-kepx
+changed
*** Update File: C:/work/example/c.txt
@@
-old
+new
*** End Patch
```

输出：

````text
<SUCCEEDED>
<EDIT>
C:/work/example/a.txt
<UUID>
019d0000-0000-7000-8000-00000000000f
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
<EDIT>
C:/work/example/c.txt
<UUID>
019d0000-0000-7000-8000-000000000010
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/b.txt
<REASON>
Failed to find expected lines. Closest match:
```
kept
```
</REASON>
</EDIT>
</FAILED>
````

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
*** Update File: C:/work/example/a.txt
@@
-missing-a
+new-a
*** Update File: C:/work/example/b.txt
@@
-old
+new
*** Update File: C:/work/example/c.txt
@@
-missing-c
+new-c
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/b.txt
<UUID>
019d0000-0000-7000-8000-000000000011
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/a.txt
<REASON>
Failed to find expected lines
</REASON>
</EDIT>
<EDIT>
C:/work/example/c.txt
<REASON>
Failed to find expected lines
</REASON>
</EDIT>
</FAILED>
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
{ "patch": "" }
```

输出：

```text
<FAILED>
<REASON>
patch must not be empty
</REASON>
</FAILED>
```

说明：空输入由 patch runner 统一生成为标准失败结果。

## 15. patch 路径不是绝对路径

输入：

```text
*** Begin Patch
*** Add File: relative.txt
+hello
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch hunk on line 2: patch paths must be absolute
</REASON>
</FAILED>
```

说明：相对路径会在解析 patch 时失败，不会进行文件写入。

## 16. patch 缺少 Begin 标记

输入：

```text
*** Update File: C:/work/example/target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch: The first line of the patch must be '*** Begin Patch'
</REASON>
</FAILED>
```

## 17. patch 缺少 End 标记

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/target.txt
@@
-old
+new
```

输出：

```text
<FAILED>
<REASON>
Invalid patch: The last line of the patch must be '*** End Patch'
</REASON>
</FAILED>
```

## 18. patch 中没有文件操作

输入：

```text
*** Begin Patch
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
No files were modified.
</REASON>
</FAILED>
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
<FAILED>
<REASON>
Invalid patch hunk on line 2: expected file operation marker
</REASON>
</FAILED>
```

## 20. Add File 内容行缺少 `+`

输入：

```text
*** Begin Patch
*** Add File: C:/work/example/target.txt
hello
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch hunk on line 3: add file lines must start with '+'
</REASON>
</FAILED>
```

## 21. Update File 缺少变更块

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/target.txt
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch hunk on line 2: update file hunk has no changes
</REASON>
</FAILED>
```

说明：如果 `Update File` 后面带有 `*** Move to: ...`，则允许没有变更块，表示只重命名。

## 22. Update File 缺少 `@@` 变更标记

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/target.txt
-old
+new
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch hunk on line 3: expected '@@' change marker
</REASON>
</FAILED>
```

## 23. Update File 变更行缺少前缀

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/target.txt
@@
old
*** End Patch
```

输出：

```text
<FAILED>
<REASON>
Invalid patch hunk on line 4: expected change line prefix
</REASON>
</FAILED>
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
*** Update File: C:/work/example/target.txt
@@
-actuel
+new
*** End Patch
```

输出：

````text
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find expected lines. Closest match:
```
actual
```
</REASON>
</EDIT>
</FAILED>
````

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
*** Update File: C:/work/example/target.txt
@@ alphx
-old
+new
*** End Patch
```

输出：

````text
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find context. Closest match:
```
alpha
```
</REASON>
</EDIT>
</FAILED>
````

## 26. 更新不存在的文件

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/missing.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<FAILED>
<EDIT>
C:/work/example/missing.txt
<REASON>
Failed to read file to update: 系统找不到指定的文件。 (os error 2)
</REASON>
</EDIT>
</FAILED>
```

说明：系统错误文本会随操作系统语言而变化。

## 27. 删除不存在的文件

输入：

```text
*** Begin Patch
*** Delete File: C:/work/example/missing.txt
*** End Patch
```

输出：

```text
<FAILED>
<DELETE>
C:/work/example/missing.txt
<REASON>
Failed to delete file: 系统找不到指定的文件。 (os error 2)
</REASON>
</DELETE>
</FAILED>
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
*** Delete File: C:/work/example/target
*** End Patch
```

输出：

```text
<FAILED>
<DELETE>
C:/work/example/target
<REASON>
Failed to delete file: path is a directory
</REASON>
</DELETE>
</FAILED>
```

## 29. heredoc 包装输入

输入：

```text
<<'EOF'
*** Begin Patch
*** Update File: C:/work/example/target.txt
@@
-old
+new
*** End Patch
EOF
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000012
</UUID>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
```

说明：外层 heredoc 包装会被识别并剥离。

## 30. 匹配时允许的宽松规则

查找待替换旧内容时，会依次尝试：

1. 完全匹配；
2. 忽略行首和行尾空白；
3. 规范化部分 Unicode 标点与空白后再匹配；
4. 忽略空行；
5. 忽略连续空格的数量。

初始文件：

```text
target.txt
----------
  old
```

输入：

```text
*** Begin Patch
*** Update File: C:/work/example/target.txt
@@
-old
+new
*** End Patch
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000013
</UUID>
before: 1 lines, 9 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
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
PATCH_DIR=C:/work/example/docs
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
<SUCCEEDED>
<ADD>
C:/work/example/docs/example.txt
<UUID>
019d0000-0000-7000-8000-000000000014
</UUID>
after: 1 lines, 6 chars
</ADD>
</SUCCEEDED>
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
<FAILED>
<REASON>
Invalid patch hunk on line 2: environment variable 'MISSING_FILE' is not set in path '$MISSING_FILE'
</REASON>
</FAILED>
```

# `undo_patch`

`undo_patch` 接受 UUID 数组：

```json
{
  "uuids": [
    "019d0000-0000-7000-8000-000000000101",
    "019d0000-0000-7000-8000-000000000102"
  ]
}
```

UUID 按数组顺序逐个处理，其中一项失败不会回滚已成功项，也不会阻止后续项。
成功项使用 `<ADD>`、`<EDIT>` 或 `<DELETE>` 表示 Undo 实际产生的文件操作。路径后的 `<UUID>` 是本次 Undo 的新 UUID，`<UNDO_OF>` 是请求撤销的 UUID。新 UUID 本身也可以传给 `undo_patch`，从而撤回这次 Undo。

原操作与成功 Undo 块的对应关系如下：

| 原操作                  | Undo 的实际效果        | 成功块                       |
| ----------------------- | ---------------------- | ---------------------------- |
| 新增原本不存在的文件    | 删除新增文件           | `<DELETE>`                   |
| `Add File` 覆盖已有文件 | 恢复被覆盖内容         | `<EDIT>`                     |
| 编辑文件                | 逆向合并编辑           | `<EDIT>`                     |
| 删除文件                | 恢复已删除文件         | `<ADD>`                      |
| Move（包括覆盖目标）    | 恢复源文件和原目标状态 | `<EDIT>`，显示恢复后的源路径 |

单项失败使用 `<UNDO>` 块，包含请求值和失败原因：

```text
<UNDO>
<UUID>
请求值
</UUID>
<REASON>
失败原因
</REASON>
</UNDO>
```

只要存在失败项，MCP tool result 的 `is_error` 就为 `true`。

## 33. 批量撤销普通新增与删除

初始状态是两个 `apply_patch` 操作已经完成。

输入：

```json
{
  "uuids": [
    "019d0000-0000-7000-8000-000000000101",
    "019d0000-0000-7000-8000-000000000102"
  ]
}
```

输出：

```text
<SUCCEEDED>
<DELETE>
C:/work/example/created.txt
<UUID>
019d0000-0000-7000-8000-000000000201
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000101
</UNDO_OF>
before: 1 lines, 8 chars
</DELETE>
<ADD>
C:/work/example/obsolete.txt
<UUID>
019d0000-0000-7000-8000-000000000202
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000102
</UNDO_OF>
after: 1 lines, 9 chars
</ADD>
</SUCCEEDED>
```

最终状态：`created.txt` 被删除，`obsolete.txt` 恢复为删除前的内容。

## 34. 批量撤销允许部分成功

`a.txt` 的一次编辑操作 UUID 为 `019d0000-0000-7000-8000-000000000103`，文件当前内容仍是该操作产生的 `new`。第二个 UUID 不存在。

输入：

```json
{
  "uuids": [
    "019d0000-0000-7000-8000-000000000103",
    "019d0000-0000-7000-8000-0000000001ff"
  ]
}
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/a.txt
<UUID>
019d0000-0000-7000-8000-000000000203
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000103
</UNDO_OF>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<UNDO>
<UUID>
019d0000-0000-7000-8000-0000000001ff
</UUID>
<REASON>
unknown operation UUID: 019d0000-0000-7000-8000-0000000001ff
</REASON>
</UNDO>
</FAILED>
```

最终结果：`a.txt` 已恢复为 `old`；未知或已因数量限制被清理的 UUID 失败，但不影响第一项。

## 35. 撤销覆盖式 Add 恢复旧文件

初始文件原本包含 `old`，随后被一次 `Add File` 操作覆盖为 `new`。

输入：

```json
{ "uuids": ["019d0000-0000-7000-8000-000000000104"] }
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000204
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000104
</UNDO_OF>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
```

最终文件恢复为 `old`。覆盖式 Add 的 Undo 是编辑，不会删除原本就存在的文件。

## 36. 撤销覆盖式 Move 同时恢复源和目标

Move 前：

```text
C:/work/example/source.txt: from
C:/work/example/destination.txt: covered
```

Move 将 `source.txt` 移到 `destination.txt` 并覆盖目标。此时源路径不存在，目标内容为 `from`。

输入：

```json
{ "uuids": ["019d0000-0000-7000-8000-000000000105"] }
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/source.txt
<UUID>
019d0000-0000-7000-8000-000000000205
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000105
</UNDO_OF>
before: 1 lines, 5 chars
after: 1 lines, 5 chars
</EDIT>
</SUCCEEDED>
```

最终结果：`source.txt` 恢复为 `from`，`destination.txt` 恢复为被覆盖前的 `covered`。成功块显示恢复后的源路径；目标路径的恢复属于同一个 Undo，不会额外生成 UUID。

## 37. 撤销编辑时保留不相交改动

操作把第二行的 `old` 改成了 `new`。之后文件末尾又加入了与该编辑不相交的 `tail`：

```text
header
new
footer
tail
```

输入：

```json
{ "uuids": ["019d0000-0000-7000-8000-000000000106"] }
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000206
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000106
</UNDO_OF>
before: 4 lines, 23 chars
after: 4 lines, 23 chars
</EDIT>
</SUCCEEDED>
```

最终文件：

```text
header
old
footer
tail
```

Undo 恢复目标操作，同时保留不相交的后续编辑。

## 38. 与后续改动冲突时失败

操作把 `old` 改成了 `new`，之后同一位置又被修改成 `custom`。

输入：

```json
{ "uuids": ["019d0000-0000-7000-8000-000000000107"] }
```

输出：

```text
<FAILED>
<UNDO>
<UUID>
019d0000-0000-7000-8000-000000000107
</UUID>
<REASON>
current file changes conflict with the operation being undone
</REASON>
</UNDO>
</FAILED>
```

最终文件仍为 `custom`。冲突项不修改文件，也不会生成新的 UUID。

## 39. 撤销一次 Undo

继续第 35 节的状态，将那次 Undo 生成的 UUID 作为新输入：

```json
{ "uuids": ["019d0000-0000-7000-8000-000000000204"] }
```

输出：

```text
<SUCCEEDED>
<EDIT>
C:/work/example/target.txt
<UUID>
019d0000-0000-7000-8000-000000000207
</UUID>
<UNDO_OF>
019d0000-0000-7000-8000-000000000204
</UNDO_OF>
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
```

最终文件再次变为 `new`。新生成的 `019d0000-0000-7000-8000-000000000207` 仍可继续撤销。
