use bitcoin::{BlockHash, Network, bip32::Xpriv, constants::ChainHash};
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
use std::{env, fs, path::Path, sync::Arc};
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

fn main() {
    let args: Vec<String> = env::args().collect();
    let ldk_dir = args.get(1).unwrap();

    let ldk_store = SqliteStore::new(
        ldk_dir.into(),
        Some(SQLITE_DB_FILE_NAME.to_string()),
        Some(KV_TABLE_NAME.to_string()),
    )
    .unwrap();

    let res = ldk_store
        .read(
            CHANNEL_MANAGER_PERSISTENCE_PRIMARY_NAMESPACE,
            CHANNEL_MANAGER_PERSISTENCE_SECONDARY_NAMESPACE,
            CHANNEL_MANAGER_PERSISTENCE_KEY,
        )
        .unwrap();

    let seed = read_or_generate_seed_file(format!("{}/keys_seed", ldk_dir)).unwrap();
    let cur_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let xprv = Xpriv::new_master(Network::Regtest, &seed).unwrap();
    let keys_manager = &KeysManager::new(
        &xprv.private_key.secret_bytes(),
        cur_time.as_secs(),
        cur_time.subsec_nanos(),
    );

    let mut reader = Cursor::new(res);

    let _v: u8 = Readable::read(&mut reader).unwrap();
    let _v: u8 = Readable::read(&mut reader).unwrap();

    let _c: ChainHash = Readable::read(&mut reader).unwrap();
    let _bh: u32 = Readable::read(&mut reader).unwrap();
    let _bh: BlockHash = Readable::read(&mut reader).unwrap();

    let mut user_conf = UserConfig::default();
    user_conf
        .channel_handshake_config
        .negotiate_anchors_zero_fee_htlc_tx = true;

    let channel_count: u64 = Readable::read(&mut reader).unwrap();
    for _ in 0..channel_count {
        let chan = FundedChannel::read(
            &mut reader,
            (
                &keys_manager,
                &keys_manager,
                &provided_channel_type_features(&user_conf),
            ),
        )
        .unwrap();

        let logger = Arc::new(NLogger {});

        let current_commitment = chan.context.build_commitment_transaction(
            &chan.funding,
            chan.holder_commitment_point.transaction_number(),
            &chan.holder_commitment_point.current_point(),
            true,
            true,
            &logger,
        );
        println!("channel {}", chan.context.channel_id);

        println!(
            "commitment tx {:?}",
            current_commitment.tx.built.transaction
        )
    }
}
