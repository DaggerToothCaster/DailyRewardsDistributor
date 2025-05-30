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

### 2. 编译

```bash
cargo compile
```

### 3. 运行

```bash
cargo run

