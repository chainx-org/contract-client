// Copyright 2019 Chainpool

use codec::{Decode, Encode, Compact};
use futures::Future;
use hex;
use primitives::{AccountId, Hash, Index};
use runtime::{Address, Call, Runtime};
use serde_json::Value;
use sr_primitives::generic::Era;
use srml_support::storage::StorageMap;
use substrate_primitives::ed25519::Pair;
use substrate_primitives::hexdisplay::HexDisplay;
use substrate_primitives::twox_128;
use Rpc;

pub struct RawSeed<'a>(&'a str);

impl<'a> RawSeed<'a> {
    pub fn new(seed: &'a str) -> Self {
        RawSeed(seed)
    }

    // Unsafe, for test only
    pub fn pair(&self) -> Pair {
        let seed = &self.0;
        let mut s: [u8; 32] = [' ' as u8; 32];
        let len = ::std::cmp::min(32, seed.len());
        &mut s[..len].copy_from_slice(&seed.as_bytes()[..len]);
        Pair::from_seed(&s)
    }

    pub fn account_id(&self) -> AccountId {
        let pair = Self::pair(self);
        AccountId::from(pair.public().0)
    }
}

pub fn genesis_hash(client: &mut Rpc) -> Hash {
    client
        .request::<Hash>("chain_getBlockHash", vec![json!(0 as u64)])
        .wait()
        .unwrap()
        .unwrap()
}

pub fn account_nonce(client: &mut Rpc, account_id: &AccountId) -> Index{
    let key = <srml_system::AccountNonce<Runtime>>::key_for(account_id);
    let key = twox_128(&key);
    let key = format!("0x{:}", HexDisplay::from(&key));
    let nonce = client
        .request::<Value>("state_getStorage", vec![json!(key)])
        .wait()
        .unwrap()
        .unwrap();
    if nonce.is_string() {
        let nonce = nonce.as_str().unwrap();
        let blob = hex::decode(&nonce[2..]).unwrap();
        let index: Option<Index> = Decode::decode(&mut blob.as_slice());
        index.unwrap()
    } else {
        0
    }
}

pub fn sudo(client: &mut Rpc, tx: String) -> u64 {
    client
        .request::<u64>("author_submitAndWatchExtrinsic", vec![json!(tx)])
        .wait()
        .unwrap()
        .unwrap()
}

pub fn generate_sudo_tx(
    seed: &RawSeed,
    from: AccountId,
    index: Index,
    hash: Hash,
) -> String {
    let func = runtime::Call::Sudo(sudo::Call::sudo::<runtime::Runtime>(
        Box::new(runtime::Call::Consensus(consensus::Call::set_heap_pages::<runtime::Runtime>(64)))
    ));

    generate_tx(seed, from, func, index, (Era::Immortal, hash))
}

fn generate_tx(
    raw_seed: &RawSeed,
    sender: AccountId,
    function: Call,
    index: Index,
    e: (Era, Hash),
) -> String {
    let era = e.0;
    let hash: Hash = e.1;
    let sign_index: Compact<Index> = index.into();
    let payload = (sign_index, function.clone(), era, hash, Compact::<u32>::from(1));
    let signed: Address = sender.into();
    let pair = raw_seed.pair();
    let s = pair.sign(&payload.encode());

    let signature = sr_primitives::Ed25519Signature(s);
    assert_eq!(
        sr_primitives::verify_encoded_lazy(&signature, &payload, &sender),
        true
    );

    // 编码字段 1 元组(发送人，签名)，func | 签名：(index,func, era, h)
    let uxt = runtime::UncheckedExtrinsic::new_signed(index, function, signed, signature, era, 1);
    let t: Vec<u8> = uxt.encode();
    //format!("0x{:}", HexDisplay::from(&t))
    format!("0x{:}", hex::encode(&t))
}
