use std::time::Duration;

use alloy_rpc_types_beacon::{BlsPublicKey, BlsSignature};
use cb_common::{
    commit::request::SignRequest,
    config::{load_module_config, StartModuleConfig},
    utils::initialize_tracing_log,
};
// use cb_metrics::sdk::{register_custom_metric, update_custom_metric};
use eyre::OptionExt;
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{error, info};
use tree_hash_derive::TreeHash;

#[derive(TreeHash)]
struct Datagram {
    data: u64,
}

struct DaCommitService {
    config: StartModuleConfig<ExtraConfig>,
}

#[derive(Debug, Deserialize)]
struct ExtraConfig {
    sleep_secs: u64,
}

impl DaCommitService {
    pub async fn run(self) -> eyre::Result<()> {
        let pubkeys = self.config.signer_client.get_pubkeys().await?;
        info!(consensus = pubkeys.consensus.len(), proxy = pubkeys.proxy.len(), "Received pubkeys");

        let pubkey = pubkeys.consensus.first().ok_or_eyre("no key available")?;
        info!("Registered validator {pubkey}");

        let mut data = 0;

        loop {
            self.send_request(data, *pubkey).await?;

            // update_custom_metric("custom_metric", 42.0, vec![(
            //     "label_key".to_string(),
            //     "label_value".to_string(),
            // )])
            // .await
            // .expect("Failed to update custom metric");

            sleep(Duration::from_secs(self.config.extra.sleep_secs)).await;
            data += 1;
        }
    }

    pub async fn send_request(&self, data: u64, pubkey: BlsPublicKey) -> eyre::Result<()> {
        let datagram = Datagram { data };
        let request = SignRequest::builder(&self.config.id, pubkey).with_msg(&datagram);

        let signature = self.config.signer_client.request_signature(&request).await?;

        info!("Proposer commitment: {}", pretty_print_sig(signature));

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    initialize_tracing_log();

    match load_module_config::<ExtraConfig>() {
        Ok(config) => {
            info!(
                module_id = config.id,
                sleep_secs = config.extra.sleep_secs,
                "Starting module with custom data"
            );

            let service = DaCommitService { config };

            // register_custom_metric("custom_metric", "A custom metric for demonstration")
            //     .await
            //     .expect("Failed to register custom metric.");

            if let Err(err) = service.run().await {
                error!(?err, "Service failed");
            }
        }
        Err(err) => {
            error!(?err, "Failed to load module config");
        }
    }
}

fn pretty_print_sig(sig: BlsSignature) -> String {
    format!("{}..", &sig.to_string()[..16])
}
