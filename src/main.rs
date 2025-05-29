use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{error, info};

mod config;
mod contract;
mod scheduler;
mod debug;

use config::Config;
use contract::RewardsContract;
use scheduler::DailyScheduler;
use debug::ContractDebugger;

#[tokio::main]
async fn main() -> Result<()> {
    // 初始化日志
    tracing_subscriber::fmt::init();

    // 加载配置
    dotenv::dotenv().ok();
    let config = Config::from_env()?;

    info!("启动每日奖励分发服务...");
    info!("合约地址: {}", config.contract_address);
    info!("RPC URL: {}", config.rpc_url);

    // 创建以太坊客户端
    let provider = Provider::<Http>::try_from(&config.rpc_url)?;
    let wallet: LocalWallet = config
        .private_key
        .parse::<LocalWallet>()?
        .with_chain_id(config.chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);

    // 创建合约实例
    let rewards_contract = RewardsContract::new(config.contract_address, client.clone(),config.gas_limit,
        config.gas_price,config.chain_id);

    // 创建调度器
    let mut scheduler = DailyScheduler::new().await?;

    // 添加每日任务
    let contract_clone = rewards_contract.clone();
    scheduler
        .add_daily_job(move || {
            let contract = contract_clone.clone();
            async move { distribute_daily_rewards(contract).await }
        })
        .await?;

    // 可选：添加测试任务（每分钟执行一次，用于测试）
    // 在生产环境中可以注释掉这部分
    #[cfg(debug_assertions)]
    {
        let contract_test = rewards_contract.clone();
        scheduler
            .add_test_job(move || {
                let contract = contract_test.clone();
                async move {
                    info!("执行测试任务 - 检查合约状态");
                    let _ = distribute_daily_rewards(contract).await;
                    Ok(())
                }
            })
            .await?;
    }

    // 启动调度器
    scheduler.start().await?;

    info!(
        "调度器已启动，下次执行时间: {}",
        DailyScheduler::next_midnight()
    );
    info!("按 Ctrl+C 退出服务");

    // 保持程序运行
    tokio::signal::ctrl_c().await?;
    info!("收到退出信号，正在关闭服务...");

    scheduler.shutdown().await?;
    info!("服务已关闭");

    Ok(())
}

async fn distribute_daily_rewards(contract: RewardsContract) -> Result<()> {
    info!("开始分发每日奖励...");

    // 调用分发奖励函数
    match contract.distribute_daily_rewards().await {
        Ok(tx_hash) => {
            info!("每日奖励分发成功! 交易哈希: {:?}", tx_hash);

            // 等待交易确认
            if let Ok(receipt) = contract.wait_for_confirmation(tx_hash).await {
                info!("交易已确认，区块号: {:?}", receipt.block_number);
                info!("Gas使用量: {:?}", receipt.gas_used);
            }
        }
        Err(e) => {
            error!("分发每日奖励失败: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
