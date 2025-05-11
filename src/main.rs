use anyhow::anyhow;
use bitcoin::{
    Address, BlockHash, FeeRate, Network, bip32::Xpriv, constants::ChainHash, params::Params,
};
use ldk_node::{
    config::WALLET_KEYS_SEED_LEN,
    io::sqlite_store::{KV_TABLE_NAME, SQLITE_DB_FILE_NAME, SqliteStore},
    lightning::util::persist::{
        CHANNEL_MANAGER_PERSISTENCE_KEY, CHANNEL_MANAGER_PERSISTENCE_PRIMARY_NAMESPACE,
        CHANNEL_MANAGER_PERSISTENCE_SECONDARY_NAMESPACE, KVStore,
    },
};
use lightning::{
    io::Cursor,
    ln::{channel::FundedChannel, channelmanager::provided_channel_type_features},
    sign::KeysManager,
    util::{
        config::UserConfig,
        logger::{Logger, Record},
        ser::{Readable, ReadableArgs},
    },
};
use rand::{RngCore, thread_rng};
use std::{env, fs, path::Path, process, sync::Arc};
use std::{io::Write, time::SystemTime};

fn read_or_generate_seed_file(
    keys_seed_path: String,
) -> std::io::Result<[u8; WALLET_KEYS_SEED_LEN]> {
    if Path::new(&keys_seed_path).exists() {
        let seed = fs::read(keys_seed_path)?;

        if seed.len() != WALLET_KEYS_SEED_LEN {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to read keys seed file due to invalid length",
            ));
        }

        let mut key = [0; WALLET_KEYS_SEED_LEN];
        key.copy_from_slice(&seed);
        Ok(key)
    } else {
        let mut key = [0; WALLET_KEYS_SEED_LEN];
        thread_rng().fill_bytes(&mut key);

        if let Some(parent_dir) = Path::new(&keys_seed_path).parent() {
            fs::create_dir_all(parent_dir)?;
        }

        let mut f = fs::File::create(keys_seed_path)?;
        f.write_all(&key)?;
        f.sync_all()?;
        Ok(key)
    }
}

struct NLogger {}

impl Logger for NLogger {
    fn log(&self, _record: Record) {}
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let ldk_dir = match args.get(1) {
        Some(dir) => dir,
        None => {
            println!("specify directory where ldk-node storage is");
            process::exit(1);
        }
    };

    let ldk_store = SqliteStore::new(
        ldk_dir.into(),
        Some(SQLITE_DB_FILE_NAME.to_string()),
        Some(KV_TABLE_NAME.to_string()),
    )?;

    let res = ldk_store.read(
        CHANNEL_MANAGER_PERSISTENCE_PRIMARY_NAMESPACE,
        CHANNEL_MANAGER_PERSISTENCE_SECONDARY_NAMESPACE,
        CHANNEL_MANAGER_PERSISTENCE_KEY,
    )?;

    let seed = read_or_generate_seed_file(format!("{}/keys_seed", ldk_dir))?;
    let cur_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

    let xprv = Xpriv::new_master(Network::Regtest, &seed)?;
    let keys_manager = &KeysManager::new(
        &xprv.private_key.secret_bytes(),
        cur_time.as_secs(),
        cur_time.subsec_nanos(),
    );

    let mut reader = Cursor::new(res);

    // stuff to read before reading the channel count
    let _: u8 = Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;
    let _: u8 = Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;
    let _: ChainHash = Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;
    let _: u32 = Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;
    let _: BlockHash = Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;

    let mut user_conf = UserConfig::default();
    user_conf
        .channel_handshake_config
        .negotiate_anchors_zero_fee_htlc_tx = true;

    let channel_count: u64 =
        Readable::read(&mut reader).map_err(|e| anyhow!("error reading {e}"))?;
    for _ in 0..channel_count {
        let chan = FundedChannel::read(
            &mut reader,
            (
                &keys_manager,
                &keys_manager,
                &provided_channel_type_features(&user_conf),
            ),
        )
        .map_err(|e| anyhow!("error reading channel {e}"))?;

        let logger = Arc::new(NLogger {});

        println!(
            "---------------------------------------------------------------------------------------------------"
        );
        println!("Channel ID: {}", chan.context.channel_id);
        println!(
            "Channel Size (sats): {}\n",
            chan.funding.get_value_satoshis()
        );

        // funding tx
        let funding_tx = chan.funding.funding_transaction.clone().unwrap();
        println!("Funding Transaction: ");
        println!("Transaction ID: {}", funding_tx.compute_txid());
        println!("Inputs:");
        for (index, input) in funding_tx.input.iter().enumerate() {
            println!("  Input {}: {:?}", index, input.previous_output);
        }
        println!("\nOutputs:");
        for (index, output) in funding_tx.output.iter().enumerate() {
            println!("  Output {}:", index);
            println!("    Value: {} satoshis", output.value);
            println!("    Script PubKey: {}", output.script_pubkey);
            println!(
                "    Address: {}\n",
                Address::from_script(&output.script_pubkey, Params::new(Network::Regtest))?
            );
        }

        let current_commitment = chan.context.build_commitment_transaction(
            &chan.funding,
            chan.holder_commitment_point.transaction_number(),
            &chan.holder_commitment_point.current_point(),
            chan.funding.is_outbound(),
            false,
            &logger,
        );

        let pending_outgoing: u64 = current_commitment
            .htlcs_included
            .iter()
            .filter(|htlc| htlc.0.offered)
            .map(|htlc| htlc.0.amount_msat / 1000)
            .sum();

        let local_commitment_tx = current_commitment.tx;
        let local_balance = local_commitment_tx.to_broadcaster_value_sat();
        let remote_balance = local_commitment_tx.to_countersignatory_value_sat();

        println!("Channel Information: ");
        let (to_remote_reserve, to_self_reserve) = chan
            .funding
            .get_holder_counterparty_selected_channel_reserve_satoshis();
        println!("Channel Reserve: {}", to_self_reserve.unwrap());

        println!("Inbound Capacity: {}", (remote_balance - to_remote_reserve));
        println!(
            "Outbound Capacity: {}",
            (local_balance - to_self_reserve.unwrap())
        );

        println!("Pending Outgoing HTLCs Amount: {}", pending_outgoing);

        // info about the current commitment tx
        println!("\nCurrent Commitment Transaction: ");

        println!(
            "Balance to local node: {}",
            local_commitment_tx.to_broadcaster_value_sat()
        );
        println!(
            "Balance to remote node: {}",
            local_commitment_tx.to_countersignatory_value_sat()
        );

        println!(
            "Feerate: {} sat/vB",
            FeeRate::from_sat_per_kwu(local_commitment_tx.feerate_per_kw.into())
                .to_sat_per_vb_ceil()
        );
        println!(
            "Transaction fee: {}\n",
            current_commitment.stats.total_fee_sat
        );

        let htlc_idxs: Vec<u32> = current_commitment
            .htlcs_included
            .iter()
            .filter_map(|htlc| {
                if htlc.0.transaction_output_index.is_some() {
                    Some(htlc.0.transaction_output_index.unwrap())
                } else {
                    None
                }
            })
            .collect();

        println!("Inputs:");
        for (index, input) in local_commitment_tx
            .built
            .transaction
            .input
            .iter()
            .enumerate()
        {
            println!("  Input {}: {:?}", index, input.previous_output);
        }
        println!("\nOutputs:");
        for (index, output) in local_commitment_tx
            .built
            .transaction
            .output
            .iter()
            .enumerate()
        {
            if htlc_idxs.contains(&(index as u32)) {
                println!("  Output {} (HTLC output):", index);
            } else {
                println!("  Output {}:", index);
            }
            println!("    Value: {} sats", output.value);
            println!("    Script Pubkey: {}", output.script_pubkey);
            println!(
                "    Address: {}\n",
                Address::from_script(&output.script_pubkey, Params::new(Network::Regtest))?
            );
        }

        println!(
            "---------------------------------------------------------------------------------------------------"
        );
    }
    Ok(())
}
