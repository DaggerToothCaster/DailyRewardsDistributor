use anyhow::Result;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::info;
use crate::contract::RewardsContract;

pub struct ContractDebugger {
    contract: RewardsContract,
}

impl ContractDebugger {
    pub fn new(contract: RewardsContract) -> Self {
        Self { contract }
    }
    
    /// 执行完整的合约诊断
    pub async fn diagnose(&self) -> Result<()> {
        info!("=== 开始合约诊断 ===");
        
        // 1. 基本连接测试
        info!("1. 测试合约连接...");
        self.contract.test_connection().await?;
        
        // 2. 网络信息
        info!("2. 获取网络信息...");
        let network_info = self.contract.get_network_info().await?;
        info!("网络信息: {:#?}", network_info);
        
        // 3. 账户信息
        info!("3. 检查账户信息...");
        let balance = self.contract.get_balance().await?;
        info!("账户余额: {} ETH", ethers::utils::format_ether(balance));
        
        // 4. 合约状态
        info!("4. 获取合约状态...");
        let status = self.contract.get_contract_status().await?;
        info!("合约状态: {:#?}", status);
        
        // 5. 权限检查
        info!("5. 检查调用权限...");
        self.check_permissions(&status).await;
        
        // 6. 时间检查
        info!("6. 检查时间条件...");
        self.check_timing(&status).await;
        
        // 7. 余额检查
        info!("7. 检查奖励池余额...");
        self.check_rewards_pool(&status).await;
        
        // 8. 模拟执行
        info!("8. 模拟交易执行...");
        if let Err(e) = self.simulate_transaction().await {
            info!("模拟执行失败: {}", e);
        } else {
            info!("模拟执行成功");
        }
        
        info!("=== 诊断完成 ===");
        Ok(())
    }
    
    async fn check_permissions(&self, status: &crate::contract::ContractStatus) {
        let caller = self.contract.client_address();
        let owner = status.owner;
        
        if caller == owner {
            info!("✅ 权限检查通过: 调用者是合约所有者");
        } else {
            info!("❌ 权限检查失败: 调用者 {:?} 不是所有者 {:?}", caller, owner);
        }
    }
    
    async fn check_timing(&self, status: &crate::contract::ContractStatus) {
        if let Some(last_time) = status.last_distribution_time {
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            
            let time_diff = current_time - last_time.as_u64();
            let hours_since_last = time_diff / 3600;
            
            info!("上次分发时间: {} ({}小时前)", last_time, hours_since_last);
            
            if time_diff >= 23 * 3600 {
                info!("✅ 时间检查通过: 距离上次分发超过23小时");
            } else {
                let hours_remaining = (23 * 3600 - time_diff) / 3600;
                info!("❌ 时间检查失败: 还需等待 {} 小时", hours_remaining);
            }
        } else {
            info!("⚠️  无法获取上次分发时间");
        }
    }
    
    async fn check_rewards_pool(&self, status: &crate::contract::ContractStatus) {
        if status.rewards_pool > U256::zero() {
            info!("✅ 奖励池检查通过: {} wei", status.rewards_pool);
            
            if let Some(rewards_per_day) = status.rewards_per_day {
                if status.rewards_pool >= rewards_per_day {
                    info!("✅ 奖励池余额足够分发");
                } else {
                    info!("❌ 奖励池余额不足: 需要 {} wei，当前 {} wei", 
                          rewards_per_day, status.rewards_pool);
                }
            }
        } else {
            info!("❌ 奖励池检查失败: 余额为零");
        }
    }
    
    async fn simulate_transaction(&self) -> Result<()> {
        info!("尝试模拟distributeDailyRewards调用...");
        
        // 模拟调用
        let call_data = self.contract.inner_contract()
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;
        
        let tx_request = TransactionRequest {
            to: Some(self.contract.contract_address().into()),
            data: Some(call_data),
            from: Some(self.contract.client_address()),
            gas: Some(self.contract.gas_limit()),
            ..Default::default()
        };

        // Convert TransactionRequest to TypedTransaction
        let typed_tx: TypedTransaction = tx_request.into();

        match self.contract.client.call(&typed_tx, None).await {
            Ok(_) => {
                info!("✅ 模拟调用成功");
                Ok(())
            }
            Err(e) => {
                info!("❌ 模拟调用失败: {}", e);
                Err(anyhow::anyhow!("模拟失败: {}", e))
            }
        }
    }
    
    /// 手动执行一次分发（用于测试）
    pub async fn manual_distribute(&self) -> Result<()> {
        info!("=== 手动执行分发 ===");
        
        match self.contract.distribute_daily_rewards().await {
            Ok(tx_hash) => {
                info!("交易发送成功: {:?}", tx_hash);
                
                // 等待确认
                match self.contract.wait_for_confirmation(tx_hash).await {
                    Ok(receipt) => {
                        info!("交易确认成功: 区块 {:?}", receipt.block_number);
                    }
                    Err(e) => {
                        info!("等待确认失败: {}", e);
                    }
                }
            }
            Err(e) => {
                info!("交易发送失败: {}", e);
                return Err(e);
            }
        }
        
        Ok(())
    }
}
