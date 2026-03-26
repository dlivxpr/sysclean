# SysClean

SysClean 是一个面向 Windows 11 的 Rust + `ratatui` TUI 磁盘清理工具，聚焦两类高频场景：

- 清理开发工具全局缓存：`uv`、`npm`、`pnpm`、`docker`、`cargo`
- 分析用户 `Home` 目录下各级子目录的磁盘占用，并支持逐层进入查看

项目当前强调“安全优先”：

- 目录分析区严格只读，不提供删除入口
- 删除能力只存在于预定义缓存目标中
- 清理前必须先扫描、勾选，再进入统一确认

## 功能概览

- 双工作区界面：`Cache Cleanup` 与 `Space Explorer`
- 缓存扫描优先调用官方命令，失败时回退到常见 Windows 路径
- 目录扫描默认按大小降序显示，并支持逐层进入子目录
- 支持 `Home`、`End`、`PgUp`、`PgDn` 快速翻页/跳转
- 支持通过 Windows 资源管理器打开当前选中目录
- 支持目录过滤搜索
- 支持轻量目录扫描缓存，重复进入目录时可更快显示结果
- 支持删除前预览和删除后结果摘要

## 环境要求

- Windows 11
- Rust 1.94+ 与 Cargo
- 建议在 Windows Terminal 或支持 ANSI / alternate screen 的终端中运行

可选但推荐：

- 安装 `uv`、`npm`、`pnpm`、`docker` 中你实际会用到的工具
- 如果要清理 Docker builder cache，需要本机已安装且可正常调用 `docker`

## 快速开始

### 1. 克隆并进入项目

```powershell
git clone <your-repo-url>
cd sysclean
```

### 2. 直接运行

```powershell
cargo run
```

首次启动时，程序会自动：

- 扫描支持的缓存目标
- 以当前用户 `Home` 目录为根开始空间分析

### 3. 发布模式运行

```powershell
cargo run --release
```

如果你的用户目录内容很多，建议优先使用 `--release`，扫描体验会更好一些。

## 使用说明

### 界面结构

- 顶部：应用标题、当前工作区和任务状态
- 中部：主内容区
- 底部：当前页面可用的快捷键提示

### 全局快捷键

| 快捷键 | 作用 |
| --- | --- |
| `Tab` / `Left` / `Right` | 切换工作区 |
| `?` | 打开帮助 |
| `q` | 退出程序 |
| `Esc` | 关闭弹窗，或取消当前输入/任务显示 |

### 缓存清理页

此页面负责发现和清理预定义缓存目标。

#### 支持的缓存目标

- `uv`
- `npm`
- `pnpm`
- `docker`
- `cargo`

#### 快捷键

| 快捷键 | 作用 |
| --- | --- |
| `Up` / `Down` | 选择缓存项 |
| `Space` | 勾选/取消勾选当前项 |
| `a` | 全选或反选 |
| `r` | 重新扫描缓存 |
| `d` | 打开删除确认框 |
| `Enter` | 在确认框中执行删除 |

#### 清理流程

1. 启动后先完成缓存扫描
2. 用方向键选择缓存项
3. 用 `Space` 勾选想清理的项
4. 按 `d` 打开删除确认框
5. 再按 `Enter` 正式执行

#### 安全边界

- 不支持输入任意路径删除
- 仅清理程序内置识别规则发现出来的缓存位置
- Docker 当前仅执行 builder cache 清理，不做激进的系统级全盘清理

### 目录分析页

此页面负责查看用户目录各层级的磁盘占用。

#### 快捷键

| 快捷键 | 作用 |
| --- | --- |
| `Up` / `Down` | 选择目录 |
| `Enter` | 进入当前目录 |
| `Backspace` | 返回上一级 |
| `Home` | 跳到列表首项 |
| `End` | 跳到列表末项 |
| `PgUp` | 向上快速翻页 |
| `PgDn` | 向下快速翻页 |
| `/` | 进入过滤模式 |
| `o` | 用资源管理器打开当前选中目录 |
| `r` | 强制重扫当前目录 |

#### 过滤模式

按 `/` 后进入过滤模式：

- 输入关键字时列表会立即按名称过滤
- `Enter` 应用过滤并退出输入
- `Esc` 退出过滤模式
- 删除已输入内容后再回车，相当于清空过滤条件

#### 扫描行为说明

- 只展示“当前目录的直接子目录”
- 但每个子目录的体积是递归统计出来的
- 默认按大小降序排列
- 对符号链接、junction 等路径会跳过并标记
- 对无权限或读取失败的目录会显示失败状态

## 缓存发现规则

当前实现大致如下：

- `uv`
  - 优先：`uv cache dir`
  - 回退：`%LOCALAPPDATA%\uv\cache`
- `npm`
  - 优先：`npm config get cache`
  - 回退：`%LOCALAPPDATA%\npm-cache`
- `pnpm`
  - 优先：`pnpm store path`
  - 回退：常见 `pnpm store` Windows 路径
- `cargo`
  - `%USERPROFILE%\.cargo\registry`
  - `%USERPROFILE%\.cargo\git`
- `docker`
  - 优先读取 `docker system df --format json`
  - 执行清理时使用 `docker builder prune -a -f`

## 扫描缓存

目录分析结果会写入本地 JSON 缓存，目标是加快重复进入目录时的显示速度。

- 缓存命中窗口：24 小时
- 手动按 `r` 可强制忽略缓存并重新扫描
- 缓存结果会在界面中显示为“缓存”状态

当前缓存文件位置由 `dirs` crate 决定，Windows 下通常类似：

```text
C:\Users\<用户名>\AppData\Local\sysclean\scan-cache.json
```

## 开发指南

### 常用命令

```powershell
cargo run
cargo run --release
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

### 代码结构

```text
src/
  app.rs             # 应用状态机、页面状态、交互状态
  cache_cleaner.rs   # 缓存发现、预览、删除逻辑
  models.rs          # 通用数据模型
  persistence.rs     # 目录扫描缓存读写
  platform.rs        # Windows 平台相关帮助函数
  space_explorer.rs  # 目录扫描与缓存复用
  ui.rs              # ratatui 绘制逻辑
  main.rs            # 终端初始化、事件循环、后台任务调度
tests/
  *.rs               # 缓存发现、状态机、分页、过滤、持久化等回归测试
```

### 开发流程建议

1. 先写测试，再补实现
2. 改动核心逻辑后优先运行 `cargo test`
3. 准备提交前运行：
   - `cargo fmt`
   - `cargo clippy --all-targets -- -D warnings`
   - `cargo test`
   - `cargo build`

### 扩展新缓存目标

如果后续要新增缓存清理目标，建议按下面步骤改：

1. 在 `src/cache_cleaner.rs` 的 `CacheTargetKind` 中新增枚举项
2. 补充名称、说明和发现规则
3. 明确删除策略
4. 为发现逻辑和回退逻辑补测试
5. 确认该目标不会突破“只删预定义缓存路径”的安全边界

## 当前实现边界

这版是 v1，已经可用，但仍有一些明确边界：

- 仅支持 Windows 11，不兼容 Linux/macOS
- 目录分析区只读，不支持删除、移动或重命名
- “取消任务”当前是 UI 层忽略旧结果，不是底层线程的硬中断
- Docker 只做较保守的 builder cache 清理，不做更激进的 `system prune -a`
- 目录扫描目前使用同步文件遍历 + 后台线程，不是 async I/O

## 常见问题

### 启动后某个缓存显示“不可用”

这通常表示：

- 对应工具没有安装
- 官方命令不可调用
- 默认回退路径不存在

如果这符合你的机器实际情况，属于正常表现。

### 目录扫描比较慢

常见原因：

- Home 目录内容很多
- 某些子目录文件数非常大
- 你在 `debug` 模式运行

建议尝试：

- 使用 `cargo run --release`
- 先等待首次扫描完成
- 之后重复进入相同目录时，会优先复用扫描缓存

### Docker 清理没有释放很多空间

当前版本只清理较保守的 builder cache，不会主动删除镜像、命名卷或其他更高风险资源。

## 验证状态

当前代码已通过以下命令验证：

```powershell
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

## 后续可继续演进的方向

- 真正可取消的扫描/清理任务
- 更细粒度的 Docker 空间展示
- 首页概览统计卡片
- 扫描结果导出
- 更多受控缓存目标
- 历史清理记录
