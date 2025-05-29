use anyhow::Result;
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

abigen!(
    RewardsContractABI,
    r#"[
        function distributeDailyRewards() external
    ]"#
);

#[derive(Clone)]
pub struct RewardsContract {
    contract: RewardsContractABI<SignerMiddleware<Provider<Http>, LocalWallet>>,
    pub client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
    gas_limit: U256,
    gas_price: Option<U256>,
    chain_id: u64,
}

impl RewardsContract {
    pub fn new(
        address: Address,
        client: Arc<SignerMiddleware<Provider<Http>, LocalWallet>>,
        gas_limit: U256,
        gas_price: Option<U256>,
        chain_id: u64,
    ) -> Self {
        let contract = RewardsContractABI::new(address, client.clone());

        Self {
            contract,
            client,
            gas_limit,
            gas_price,
            chain_id,
        }
    }

    /// 简化的每日奖励分发函数
    pub async fn distribute_daily_rewards(&self) -> Result<H256> {
        info!("开始分发每日奖励...");

        // 估算Gas
        let gas_estimate = self.estimate_gas().await.unwrap_or(self.gas_limit);
        let gas_with_buffer = gas_estimate * 120 / 100; // 20% buffer

        info!("使用Gas限制: {}", gas_with_buffer);

        // 构建并发送交易
        let tx_request = self.build_transaction(gas_with_buffer).await?;

        info!("发送交易到网络...");
        let pending_tx = self.client.send_transaction(tx_request, None).await?;
        let tx_hash = pending_tx.tx_hash();

        info!("交易已发送，哈希: {:?}", tx_hash);

        Ok(tx_hash)
    }

    /// 构建交易
    async fn build_transaction(&self, gas_limit: U256) -> Result<TransactionRequest> {
        let call_data = self
            .contract
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;

        let nonce = self
            .client
            .get_transaction_count(self.client.address(), None)
            .await?;

        let gas_price = self.get_gas_price().await?;

        let tx_request = TransactionRequest {
            to: Some(self.contract.address().into()),
            value: Some(U256::zero()),
            gas: Some(gas_limit),
            gas_price: Some(gas_price),
            data: Some(call_data),
            nonce: Some(nonce),
            chain_id: Some(self.chain_id.into()),
            ..Default::default()
        };

        Ok(tx_request)
    }

    /// Gas估算
    async fn estimate_gas(&self) -> Result<U256> {
        let call_data = self
            .contract
            .distribute_daily_rewards()
            .calldata()
            .ok_or_else(|| anyhow::anyhow!("无法生成调用数据"))?;

        let tx_request = TransactionRequest {
            to: Some(self.contract.address().into()),
            data: Some(call_data),
            from: Some(self.client.address()),
            ..Default::default()
        };

        let typed_tx: TypedTransaction = tx_request.into();
        match self.client.estimate_gas(&typed_tx, None).await {
            Ok(gas) => Ok(gas),
            Err(e) => {
                warn!("Gas估算失败: {}", e);
                Ok(self.gas_limit)
            }
        }
    }

    /// 获取Gas价格
    async fn get_gas_price(&self) -> Result<U256> {
        if let Some(price) = self.gas_price {
            return Ok(price);
        }

        match self.client.get_gas_price().await {
            Ok(network_price) => Ok(network_price),
            Err(_) => {
                let default_price = ethers::utils::parse_units("30", "gwei")?;
                Ok(default_price
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Gas价格转换失败"))?)
            }
        }
    }

    /// 等待交易确认
    pub async fn wait_for_confirmation(&self, tx_hash: H256) -> Result<TransactionReceipt> {
        info!("等待交易确认: {:?}", tx_hash);

        let timeout = std::time::Duration::from_secs(300); // 5分钟超时
        let start_time = std::time::Instant::now();

        loop {
            if start_time.elapsed() > timeout {
                return Err(anyhow::anyhow!("交易确认超时"));
            }

            match self.client.get_transaction_receipt(tx_hash).await? {
                Some(receipt) => {
                    if receipt.status == Some(U64::from(1)) {
                        info!("交易执行成功");
                    } else {
                        warn!("交易执行失败");
                    }
                    return Ok(receipt);
                }
                None => {
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                }
            }
        }
    }

    /// 访问器方法
    pub fn client_address(&self) -> Address {
        self.client.address()
    }

    pub fn contract_address(&self) -> Address {
        self.contract.address()
    }

    pub fn inner_contract(
        &self,
    ) -> &RewardsContractABI<SignerMiddleware<Provider<Http>, LocalWallet>> {
        &self.contract
    }

    pub fn gas_limit(&self) -> U256 {
        self.gas_limit
    }
}
