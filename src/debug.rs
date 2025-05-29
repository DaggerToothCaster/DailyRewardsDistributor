use crate::contract::RewardsContract;
use anyhow::Result;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::info;

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

    async fn simulate_transaction(&self) -> Result<()> {
        info!("尝试模拟distributeDailyRewards调用...");

        // 模拟调用
        let call_data = self
            .contract
            .inner_contract()
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
