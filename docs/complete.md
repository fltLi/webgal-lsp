# 自动补全

`webgal-ls` 提供了对 WebGAL 脚本丰富的自动补全功能。

## 触发

一条 WebGAL 语句通常可以被划分为下面几个部分:
```
语句类型:主参数 -参数名=参数值; 注释
command:content -name -name=value; comment
```

下面简单介绍处于不同位置输入时 `webgal-ls` 的补全方案。

### 语句类型

语句类型补全包含三种方案：

1. **提示语句类型**  
   例如：`changeBg`，`setTransform`。

   对于需要参数的语句，将生成形如 `command:|;|` 的补全；  
   对于不需要参数的语句，将直接生成 `command;|`，即跳到语句末尾。

2. **展开语句模板**  
   `webgal-ls` 内置了一些方便的语句模板，通常以 `command.feature` 的形式显示。  
   选择模板后，将展开模板对应的补全。

   例如：`changeFigure.motion` -> `changeFigure:| -id=| -motion=| -next|;`。

3. **提示对话人物**
   考虑到对话语句的语法糖，即在语句类型处直接书写对话者，语句类型补全加入了对话者。

### 参数名

参数名补全有两种情况：

1. **普通参数补全**  
   形如 `-name=value`，将生成 `-name=` 格式的补全。

   部分参数拥有参数值模板，其将随着参数名补全自动展开。  
   例如：`intro: -fontColor=rgba(|,|,|,|)|`。

2. **bool 参数补全**  
   值为 bool 类型的参数将仅生成 `-name` 格式的补全。

除此以外，参数名补全还会自动检查当前语句当前的参数，其中**已填充的参数将不再纳入补全**。

### 主参数 / 参数值

主参数 / 参数值是 `webgal-ls` 的一大亮点，具体请看[类型](#类型)章节。

## 类型

对于不同类型的主参数和参数值，`webgal-ls` 也提供了相关的补全功能。

### 枚举

枚举即为一系列可选的值。

例如，`pixiPerform` 支持 `snow`，`heavySnow` 等特效，这些特效就是枚举项。

枚举补全覆盖了：
- **fontSize** - 字体大小
- **ease** - 缓动类型
- **animation** / **enter** / **exit** - 独立动画
- **setComplexAnimation** - 复杂动画
- **pixiPerform** - pixi 特效
- **setTextbox** - 文本显示
- **filmMode** - 电影模式

### 标识符

标识符补全是枚举补全的扩展。  
其收集所有场景的某一类标识符，并在相关参数处呈现。

标识符补全覆盖的内容有：
- **id** - 立绘、背景、舞台、音效等 ID
- **speaker** - 对话人物
- **label** - 标签
- **series** - 鉴赏系列
- **duration** - 所有需要设置时长的主参数和参数

### 资源路径

资源路径补全分目录分层实现。

例如，对于 `changeFigure:anon/` 的输入，会提示 `figure/anon/` 目录下的目录和资源信息。

资源路径补全覆盖了所有值为资源的主参数和参数。

### 立绘动作 / 表情

立绘动作 / 表情补全与[资源路径](#资源路径)补全基本一致。

目前支持 Live2D 和 WMDL 这两种立绘的补全。

### JSON

JSON 补全由 `json-complete` 提供，是作者自己造的轮子，可以宽松解析和补全 JSON 字符串。

例如，对于 `transform` 参数，当键入 `{"pos|` 时，将自动提示出 `position` 这一个键。

JSON 补全覆盖了下列类型（不分主参数 / 参数）：
- **transform** - 变换参数
- **tempAnimation** - 连续动画
- **blink** - Live2D 眨眼参数
- **focus** - Live2D 注视参数
