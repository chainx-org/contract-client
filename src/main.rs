// Copyright 2018 Chainpool

extern crate futures;
extern crate jsonrpc_client_core;
extern crate jsonrpc_core;
extern crate jsonrpc_ws_server;
extern crate parking_lot;
extern crate serde;
extern crate url;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate hex;
extern crate node_primitives as primitives;
extern crate node_runtime as runtime;
extern crate parity_codec as codec;
extern crate sr_primitives;
extern crate srml_contract as contract;
extern crate srml_support;
extern crate srml_system;
extern crate substrate_primitives;

mod chainx_rpc;
mod ws;

use self::ws::{Rpc, RpcError};
use codec::Encode;
use jsonrpc_core::Notification;
use std::fs::File;
use std::io::Read;
use std::sync::mpsc;

fn read_a_file() -> std::io::Result<Vec<u8>> {
    let mut file = try!(File::open("adder-deployed.wasm"));

    let mut data = Vec::new();
    try!(file.read_to_end(&mut data));

    return Ok(data);
}

fn chainx_thread(send_tx: mpsc::Sender<jsonrpc_ws_server::ws::Message>) -> Result<Rpc, RpcError> {
    let port = 8087;
    Rpc::new(&format!("ws://127.0.0.1:{}", port), send_tx)
}

fn main() {
    let _ = env_logger::try_init();
    let (send_tx, recv_tx) = mpsc::channel();
    let mut chainx_client = chainx_thread(send_tx.clone()).unwrap();
    let chainx_genesis_hash = chainx_rpc::genesis_hash(&mut chainx_client);
    println!("chainx genesis hash: {:?}", chainx_genesis_hash);
    let raw_seed = chainx_rpc::RawSeed::new("Alice");
    let account = raw_seed.account_id();
    let index = chainx_rpc::account_nonce(&mut chainx_client, &account);
    let code = read_a_file().unwrap();
    let tx = chainx_rpc::generate_deploy_contract_tx(
        &raw_seed,
        account,
        index,
        chainx_genesis_hash,
        code.encode(),
    );
    // deploy code
    let sub_deploy_id = chainx_rpc::deploy_contract(&mut chainx_client, tx);
    loop {
        let msg = recv_tx.recv().unwrap();
        let msg = msg.into_text().unwrap();
        let des: Notification = serde_json::from_str(&msg).unwrap();
        let des: serde_json::Map<String, serde_json::Value> = des.params.parse().unwrap();
        let sub_id = &des["subscription"];
        println!("----subscribe extrinsic return sub_id:{:?}----", sub_id);
    }
}
