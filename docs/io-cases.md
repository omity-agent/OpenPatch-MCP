# apply_patch 输入输出案例

本文档覆盖 `apply_patch` 工具的主要成功与失败场景。所有案例都假设工具调用参数形如：

```json
{
  "patch": "..."
}
```

返回文本由成功与失败两个可选分区组成。存在成功项时输出 `<SUCCEEDED>`，存在失败项时输出 `<FAILED>`；部分成功的结果会同时包含两个分区。成功操作按原 patch 中的顺序排列。

成功的新增、编辑和删除分别使用 `<ADD>`、`<EDIT>` 和 `<DELETE>` 块。新增仅包含修改后的统计，编辑同时包含修改前后的统计，删除仅包含修改前的统计。能归属文件操作的失败使用对应的操作块，并在路径后放置 `<REASON>`；解析错误等全局失败在 `<FAILED>` 下放置 `<REASON>`。

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
-missing
+changed
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
before: 3 lines, 14 chars
after: 3 lines, 8 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find expected lines:
missing
</REASON>
</EDIT>
</FAILED>
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
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
<ADD>
C:/work/example/b.txt
after: 1 lines, 8 chars
</ADD>
<EDIT>
C:/work/example/c.txt
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
-missing
+changed
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
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
<EDIT>
C:/work/example/c.txt
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/b.txt
<REASON>
Failed to find expected lines:
missing
</REASON>
</EDIT>
</FAILED>
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
before: 1 lines, 4 chars
after: 1 lines, 4 chars
</EDIT>
</SUCCEEDED>
<FAILED>
<EDIT>
C:/work/example/a.txt
<REASON>
Failed to find expected lines:
missing-a
</REASON>
</EDIT>
<EDIT>
C:/work/example/c.txt
<REASON>
Failed to find expected lines:
missing-c
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
{
  "patch": ""
}
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
-expected
+new
*** End Patch
```

输出：

```text
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find expected lines:
expected
</REASON>
</EDIT>
</FAILED>
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
*** Update File: C:/work/example/target.txt
@@ missing-anchor
-old
+new
*** End Patch
```

输出：

```text
<FAILED>
<EDIT>
C:/work/example/target.txt
<REASON>
Failed to find context 'missing-anchor'
</REASON>
</EDIT>
</FAILED>
```

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
