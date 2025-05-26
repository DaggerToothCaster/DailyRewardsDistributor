use anyhow::Result;
use ethers::prelude::*;
use std::sync::Arc;
use tracing::{info, warn};

// 定义合约ABI
abigen!(
    RewardsContractABI,
    r#"[
        function distributeDailyRewards() external
        function isActive() external view returns (bool)
        function owner() external view returns (address)
        function lastDistributionTime() external view returns (uint256)
    ]"#
);

#[derive(Clone)]
pub struct RewardsContract {
    contract: RewardsContractABI<SignerMiddleware<Provider<Http>, LocalWallet>>,
    client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
}

impl RewardsContract {
    pub fn new(
        address: Address,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    ) -> Self {
        let contract = RewardsContractABI::new(address, client.clone());
        
        Self { contract, client }
    }
    
    pub async fn distribute_daily_rewards(&self) -> Result<H256> {
        info!("调用 distributeDailyRewards 函数...");
        
        // 估算Gas
        let gas_estimate = self.contract
            .distribute_daily_rewards()
            .estimate_gas()
            .await
            .unwrap_or(U256::from(500_000));
        
        info!("估算Gas用量: {}", gas_estimate);
        
        // 发送交易
        let mut call = self.contract.distribute_daily_rewards();
        let gas_with_buffer = gas_estimate * 120 / 100; // 增加20%的Gas缓冲
        let call_with_gas = call.gas(gas_with_buffer);
        let tx = call_with_gas
            .send()
            .await?;
        
        info!("交易已发送，哈希: {:?}", tx.tx_hash());
        
        Ok(tx.tx_hash())
    }
    
    pub async fn is_contract_active(&self) -> Result<bool> {
        match self.contract.is_active().call().await {
            Ok(active) => {
                info!("合约活跃状态: {}", active);
                Ok(active)
            }
            Err(e) => {
                warn!("无法检查合约状态，假设为活跃: {}", e);
                Ok(true) // 如果无法检查状态，假设合约是活跃的
            }
        }
    }
    
    pub async fn get_last_distribution_time(&self) -> Result<U256> {
        let time = self.contract.last_distribution_time().call().await?;
        info!("上次分发时间: {}", time);
        Ok(time)
    }
    
    pub async fn wait_for_confirmation(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        info!("等待交易确认: {:?}", tx_hash);
        
        let receipt = self.client
            .get_transaction_receipt(tx_hash)
            .await?
            .ok_or_else(|| anyhow::anyhow!("交易收据未找到"))?;
        
        if receipt.status == Some(U64::from(1)) {
            info!("交易执行成功");
        } else {
            warn!("交易执行失败");
        }
        
        Ok(receipt)
    }
    
    pub async fn get_balance(&self) -> Result<U256> {
        let balance = self.client.get_balance(self.client.address(), None).await?;
        info!("账户余额: {} ETH", ethers::utils::format_ether(balance));
        Ok(balance)
    }
}