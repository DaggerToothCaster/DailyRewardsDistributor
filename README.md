# Daily Rewards Distributor

一个基于Rust的自动化服务，用于每天0点调用Solidity合约的`distributeDailyRewards`函数。

## 功能特性

- 🕛 **定时执行**: 每天0点自动执行奖励分发
- 🔗 **以太坊集成**: 使用ethers-rs与智能合约交互
- 📊 **日志记录**: 详细的执行日志和错误处理
- ⚡ **异步处理**: 基于Tokio的高性能异步运行时
- 🛡️ **错误恢复**: 智能的错误处理和重试机制
- 🔧 **配置灵活**: 通过环境变量配置所有参数

## 快速开始

### 1. 安装依赖

确保你已经安装了Rust (1.70+):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. 运行

```bash
cargo run
```

## 部署

### 1. 本地编译（开发机）
```bash
# 在项目目录下执行
cargo build --release
```
编译完成后，可执行文件位于：  
`./target/release/daily-rewards-distributor`

### 2. 上传到服务器
```bash
# 使用scp上传（替换你的信息）
scp ./target/release/daily-rewards-distributor 用户名@服务器IP:/home/用户名/
```

### 3. 服务器上运行
```bash
```
# SSH登录服务器
```
ssh 用户名@服务器IP
```

# 给程序执行权限
```
chmod +x ./daily-rewards-distributor
```
# 直接运行（前台运行，退出终端会停止）
```
./daily-rewards-distributor
```
# 或使用nohup后台运行（退出终端不会停止）
```
nohup ./daily-rewards-distributor > output.log 2>&1 &
```

### 补充说明：
1. **极简依赖**：如果程序是静态链接（用`musl`编译），服务器甚至不需要安装Rust环境
   ```bash
   rustup target add x86_64-unknown-linux-musl
   cargo build --release --target x86_64-unknown-linux-musl
   ```

2. **查看运行状态**：
   ```bash
   # 查看进程
   ps aux | grep daily-rewards-distributor
   
   # 查看输出日志
   tail -f output.log
   ```

3. **停止程序**：
   ```bash
   # 找到进程ID
   ps aux | grep daily-rewards-distributor
   
   # 停止进程
   kill 进程ID
   ```


# 对于MAC编译
> MAC 电脑可能编译不兼容，可以选择下边的方式
你现在遇到的这个错误本质上是：
**在 Mac 上编译 `x86_64-unknown-linux-musl` 目标时，缺少交叉编译器 `x86_64-linux-musl-gcc`。**


### ✅ 方法 1：安装交叉编译工具链（Mac 上）

推荐使用 [`FiloSottile/homebrew-musl-cross`](https://github.com/FiloSottile/homebrew-musl-cross) 来提供 `x86_64-linux-musl-gcc`。

```bash
brew install filosottile/musl-cross/musl-cross
```

安装完成后会得到：

```
/usr/local/bin/x86_64-linux-musl-gcc
```

接着你可以这样设置环境变量让 `cargo` 使用这个交叉编译器：

```bash
export CC_x86_64_unknown_linux_musl=x86_64-linux-musl-gcc
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=x86_64-linux-musl-gcc
```

> ⚠️ 你可以把它加到 `.zshrc` / `.bashrc` 里，或在编译前执行。

然后重新构建：

```bash
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
```

---
