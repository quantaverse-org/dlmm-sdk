use crate::*;
use anchor_lang::prelude::Pubkey;
use async_trait::async_trait;
use solana_account::Account;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcProgramAccountsConfig;

#[async_trait]
pub trait RpcClientExtension {
    async fn get_program_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>>;

    async fn get_account_and_deserialize<T>(
        &self,
        pubkey: &Pubkey,
        deserialize_fn: fn(Account) -> Result<T>,
    ) -> Result<T>;
}

#[async_trait]
impl RpcClientExtension for RpcClient {
    async fn get_program_accounts_with_config(
        &self,
        program_id: &Pubkey,
        config: RpcProgramAccountsConfig,
    ) -> Result<Vec<(Pubkey, Account)>> {
        self.get_program_ui_accounts_with_config(program_id, config)
            .await?
            .into_iter()
            .map(|(key, ui_account)| {
                let account = ui_account
                    .to_account()
                    .with_context(|| format!("Could not decode program account {key}"))?;
                Ok((key, account))
            })
            .collect()
    }

    async fn get_account_and_deserialize<T>(
        &self,
        pubkey: &Pubkey,
        deserialize_fn: fn(Account) -> Result<T>,
    ) -> Result<T> {
        let account = self.get_account(pubkey).await?;
        let data = deserialize_fn(account)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use solana_rpc_client_api::request::RpcRequest;

    #[tokio::test]
    async fn decodes_program_accounts_and_reports_invalid_data() {
        let program_id = Pubkey::new_unique();
        let key = Pubkey::new_unique();
        for (data, valid) in [("AQID", true), ("!", false)] {
            let response = json!([{
                "pubkey": key.to_string(),
                "account": {
                    "data": [data, "base64"],
                    "owner": program_id.to_string(),
                    "lamports": 10,
                    "executable": false,
                    "rentEpoch": 0,
                },
            }]);
            let client = RpcClient::new_mock_with_mocks(
                "succeeds".to_string(),
                [(RpcRequest::GetProgramAccounts, response)].into(),
            );
            let result = client
                .get_program_accounts_with_config(&program_id, RpcProgramAccountsConfig::default())
                .await;

            if valid {
                let accounts = result.unwrap();
                assert_eq!(accounts.len(), 1);
                assert_eq!(accounts[0].0, key);
                assert_eq!(accounts[0].1.data, [1, 2, 3]);
            } else {
                assert!(result.unwrap_err().to_string().contains(&key.to_string()));
            }
        }
    }
}
