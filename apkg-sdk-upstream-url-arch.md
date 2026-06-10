# APKG SDK 需求书: UpstreamUrl 架构感知

## 背景

Ubuntu 官方 APT 仓库存在经典的架构分裂：

| 域名 | 服务架构 |
|---|---|
| `archive.ubuntu.com` | amd64, i386 |
| `ports.ubuntu.com` | arm64, armhf, riscv64, ppc64el, s390x |

两个域名互补，万不包含对方架构。

当前 APKG SDK 的 `UpstreamUrl` 是一个静态字符串（DebBuilder.cs:468）：

```csharp
var uri = project.UpstreamUrl.TrimEnd('/');
```

不走 `ResolveVariables`，不支持 `Condition`。当 `.aosproj` 声明 `TargetArchitectures>amd64 arm64` 且 `UpstreamUrl` 指向单架构镜像时，arm64 构建必然失败：

```
E: Failed to fetch https://mirror.aiursoft.com/ubuntu/dists/noble/main/binary-arm64/Packages  404 Not Found
```

## 场景分析

以 `plymouth-anduinos` 为例：

```xml
<TargetArchitectures>amd64 arm64</TargetArchitectures>
<UpstreamUrl>https://mirror.aiursoft.com/ubuntu</UpstreamUrl>
<UpstreamPackage>plymouth-theme-spinner</UpstreamPackage>
<UpstreamArch>$(Arch)</UpstreamArch>
```

- amd64 构建 → `[arch=amd64] https://mirror.aiursoft.com/ubuntu` → ✅
- arm64 构建 → `[arch=arm64] https://mirror.aiursoft.com/ubuntu` → ❌ 404

同样的问题也存在于任何使用 Ubuntu 上游、且未来需要扩展 arm64 的包。

## 影响范围

当前只有 `plymouth-anduinos` 受影响（唯一下水 arm64 的派生包）。但这是 SDK 层面的基础设施缺陷——随着 arm64 支持扩展，更多包会遇到。

## 需求

`UpstreamUrl` 需要支持按架构动态解析，使同一个 `.aosproj` 可以为不同目标架构从不同上游 URL 拉包。

### 最小可行方案: `ResolveVariables` 解析

在 DebBuilder.cs 中对 `UpstreamUrl` 执行 `ResolveVariables`，使其支持 `$(Arch)` 等宏变量：

```diff
- var uri = project.UpstreamUrl.TrimEnd('/');
+ var uri = ResolveVariables(project.UpstreamUrl, project, distro, suite, arch).TrimEnd('/');
```

然后 .aosproj 中使用模板 URL：

```xml
<UpstreamUrl>https://mirror.aiursoft.com/ubuntu-ports</UpstreamUrl>
```

**优点**：一行改动，与 `UpstreamArch` / `UpstreamSuite` / `UpstreamComponent` 一致  
**局限**：无法表达"完全不同的域名"（如 archive  vs ports），需要内部镜像同时承载两个上游

### 完整方案: PropertyGroup Condition 支持

允许 PropertyGroup 中同一属性出现多次，通过 `Condition` 区分：

```xml
<UpstreamUrl Condition="'$(Arch)' == 'amd64'">https://mirror.aiursoft.com/ubuntu</UpstreamUrl>
<UpstreamUrl Condition="'$(Arch)' == 'arm64'">http://ports.ubuntu.com</UpstreamUrl>
```

**实现要点**：
1. `AosprojProject.UpstreamUrl` 从 `string` 改为条件选取（类似 `ConditionalValue`）
2. `AosprojSerializer` 支持 PropertyGroup 元素的条件累加
3. `DebBuilder` 在 `BuildContext` 中选取匹配条件的值

**优点**：完全灵活，适配任何上下游架构命名差异  
**缺点**：改动量中等，涉及 Model / Serializer / Builder 三层

### 建议

- **先做最小方案**（`ResolveVariables`），将 arm64 的端口镜像需求交给运维侧（内部 mirror 同时拉 archive + ports）
- **完整方案**作为 backlog，等下一个上游 URL 分裂场景出现时再做

## 验收标准

### 最小方案

1. `apkg build --arch amd64` 时，`UpstreamUrl` 中的 `$(Arch)` 解析为 `amd64`
2. `apkg build --arch arm64` 时，`UpstreamUrl` 中的 `$(Arch)` 解析为 `arm64`
3. 未使用 `$(Arch)` 的字面量 URL 行为不变（向后兼容）
4. 单元测试覆盖：`$(Arch)` 解析、字面量透传

### 完整方案（如实施）

1. 两个带不同 `Condition` 的 `UpstreamUrl` 分别匹配对应架构
2. 未写 `Condition` 的 `UpstreamUrl` 作为默认值（向后兼容）
3. 无匹配条件的架构构建时报错（不应静默回退）

## 临时 workaround

在 SDK 修复前，plymouth-anduinos 的 arm64 构建只能绕道：

1. 运维侧让 `mirror.aiursoft.com` 同步 arm64（推荐）
2. 或手动切 `.aosproj` 的 `UpstreamUrl` 后构建 arm64，再改回去
