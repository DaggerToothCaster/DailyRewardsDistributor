use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::{error, info, warn};

mod config;
mod contract;
mod scheduler;

use config::Config;
use contract::RewardsContract;

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
    let wallet: LocalWallet = config.private_key.parse::<LocalWallet>()?
        .with_chain_id(config.chain_id);
    let client = SignerMiddleware::new(provider, wallet);
    let client = Arc::new(client);
    
    // 创建合约实例
    let rewards_contract = RewardsContract::new(config.contract_address, client.clone());
    
    // 创建调度器
    let mut sched = JobScheduler::new().await?;
    
    // 添加每日0点执行的任务
    let contract_clone = rewards_contract.clone();
    let job = Job::new_async("0 0 0 * * *", move |_uuid, _l| {
        let contract = contract_clone.clone();
        Box::pin(async move {
            if let Err(e) = distribute_daily_rewards(contract).await {
                error!("分发每日奖励失败: {}", e);
            }
        })
    })?;
    
    sched.add(job).await?;
    
    // 启动调度器
    sched.start().await?;
    
    info!("调度器已启动，等待每日0点执行任务...");
    
    // 保持程序运行
    tokio::signal::ctrl_c().await?;
    info!("收到退出信号，正在关闭服务...");
    
    sched.shutdown().await?;
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
            }
        }
        Err(e) => {
            error!("分发每日奖励失败: {}", e);
            return Err(e);
        }
    }
    
    Ok(())
}