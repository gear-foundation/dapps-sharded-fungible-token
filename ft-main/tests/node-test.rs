// This file is part of Gear.

// Copyright (C) 2021-2022 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Test for harmful demos, checking their init can't brake the chain.

use blake2_rfc::blake2b;
use ft_logic_io::Action;
use ft_main_io::*;
use gclient::{EventListener, EventProcessor, GearApi, Result};
use gear_core::ids::ProgramId;
use gstd::CodeId;
use gstd::{prelude::*, ActorId};
use primitive_types::H256;
use std::time::Duration;

const HASH_LENGTH: usize = 32;
type Hash = [u8; HASH_LENGTH];

const PATHS: [&str; 3] = [
    "../target/wasm32-unknown-unknown/release/ft_main.opt.wasm",
    "../target/wasm32-unknown-unknown/release/ft_logic.opt.wasm",
    "../target/wasm32-unknown-unknown/release/ft_storage.opt.wasm",
];

static mut TOKEN_ID: [u8; 32] = [0; 32];
const FTOKEN: [u8; 32] =
    hex_literal::hex!("ab240e729b911b9e16b72dfd4e65d22979678aab9049082de9ad030c81fd31f7");
// 0x4c6eedf8f825338c8ab309729c53dfa7de7f639181111cc850dd06c3496de976
async fn upload_programs_and_check(api: &GearApi, listener: &mut EventListener) -> Result<()> {
    // upload codes for main fungible token contract
    let mut storage_code_id: Hash = Default::default();
    let storage_code = gclient::code_from_os(PATHS[2])?;
    storage_code_id[..]
        .copy_from_slice(blake2b::blake2b(HASH_LENGTH, &[], &storage_code).as_bytes());
    api.upload_code(storage_code).await;

    let mut logic_code_id: Hash = Default::default();
    let logic_code = gclient::code_from_os(PATHS[1])?;
    logic_code_id[..].copy_from_slice(blake2b::blake2b(HASH_LENGTH, &[], &logic_code).as_bytes());
    api.upload_code(logic_code).await;

    let init_ft_payload = unsafe {
        InitFToken {
            storage_code_hash: storage_code_id.into(),
            ft_logic_code_hash: logic_code_id.into(),
        }
        .encode()
    };

    println!("payload {:?}", init_ft_payload);
    let gas_info = api
        .calculate_upload_gas(
            None,
            gclient::code_from_os(PATHS[0])?,
            init_ft_payload.clone(),
            0,
            true,
            None,
        )
        .await?;

    // Program initialization.
    let (mid, _pid, _) = api
        .upload_program_bytes_by_path(
            PATHS[0],
            gclient::bytes_now(),
            init_ft_payload,
            gas_info.min_limit,
            0,
        )
        .await?;
    // Asserting successful initialization.
    assert!(listener.message_processed(mid).await?.succeed());

    println!("FT MAIN ID {:?}", _pid);
    unsafe {
        TOKEN_ID = _pid.into();
    }
    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    Ok(())
}

#[tokio::test]
async fn mint_message() -> Result<()> {
    // Creating gear api.
    //
    // By default, login as Alice, than re-login as Bob.
    let api = GearApi::dev().await?.with("//Bob")?;
    let mut listener = api.subscribe().await?;

    upload_programs_and_check(&api, &mut listener).await?;

    let mut account: u64 = 1;
    let mut transaction_id: u64 = 0;
    let amount: u128 = 10_000;

    let mut mint_payload = FTokenAction::Message {
        transaction_id,
        payload: Action::Mint {
            recipient: account.into(),
            amount,
        }
        .encode(),
    };

    // let gas_info = unsafe {
    //     api.calculate_handle_gas(None, TOKEN_ID.into(), mint_payload.encode(), 0, true, None)
    //         .await?
    // };

    // println!("GAS {:?}", gas_info.min_limit);
    // println!("GAS {:?}", gas_info.min_limit * 10 / 11);
    // let (mid, _) = unsafe {
    //     api.send_message(
    //         TOKEN_ID.into(),
    //         mint_payload,
    //         gas_info.min_limit * 10 / 11,
    //         0,
    //     )
    //     .await?
    // };

    for i in 0..10000 {
        let gas_info = unsafe {
            api.calculate_handle_gas(None, TOKEN_ID.into(), mint_payload.encode(), 0, true, None)
                .await?
        };

        println!("GAS {:?}", gas_info.min_limit);
        println!("iteration {:?}", i);
        let (mid, _) = unsafe {
            api.send_message(
                TOKEN_ID.into(),
                mint_payload.clone(),
                250000000000,
                0,
            )
            .await?
        };
        account = account + 1;
        transaction_id = transaction_id + 1;

        mint_payload = FTokenAction::Message {
            transaction_id,
            payload: Action::Mint {
                recipient: account.into(),
                amount,
            }
            .encode(),
        };

        // Asserting successful initialization.
        assert!(listener.message_processed(mid).await?.succeed());
    }

    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    Ok(())
}

#[tokio::test]
async fn continue_mint() -> Result<()> {
    // Creating gear api.
    //
    // By default, login as Alice, than re-login as Bob.
    let api = GearApi::dev().await?.with("//Bob")?;
    let mut listener = api.subscribe().await?;

    let mut account: u64 = 1000;
    let mut transaction_id: u64 = 1000;
    let amount: u128 = 10_000;

    let mut mint_payload = FTokenAction::Message {
        transaction_id,
        payload: Action::Mint {
            recipient: account.into(),
            amount,
        }
        .encode(),
    };

    for i in 1000..10000 {
        let gas_info = unsafe {
            api.calculate_handle_gas(None, FTOKEN.into(), mint_payload.encode(), 0, true, None)
                .await?
        };

        println!("GAS {:?}", gas_info.min_limit);
        println!("GAS {:?}", gas_info.min_limit + gas_info.min_limit / 5);
        println!("iteration {:?}", i);
        let (mid, _) = unsafe {
            api.send_message(
                FTOKEN.into(),
                mint_payload.clone(),
                gas_info.min_limit + gas_info.min_limit / 5,
                0,
            )
            .await?
        };
        account = account + 1;
        transaction_id = transaction_id + 1;

        mint_payload = FTokenAction::Message {
            transaction_id,
            payload: Action::Mint {
                recipient: account.into(),
                amount,
            }
            .encode(),
        };

        // Asserting successful initialization.
        assert!(listener.message_processed(mid).await?.succeed());
    }

    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    Ok(())
}

#[tokio::test]
async fn batch_mint() -> Result<()> {
    // Creating gear api.
    //
    // By default, login as Alice, than re-login as Bob.
    let api = GearApi::dev().await?.with("//Bob")?;
    let mut listener = api.subscribe().await?;

    let mut account: u64 = 1004;
    let mut transaction_id: u64 = 1004;
    let amount: u128 = 10_000;

    let mut payloads: Vec<Vec<u8>> = Vec::new();

    for i in 1..40 {
        let mint_payload = FTokenAction::Message {
            transaction_id,
            payload: Action::Mint {
                recipient: account.into(),
                amount,
            }
            .encode(),
        }.encode();
        account = account + 1;
        transaction_id = transaction_id + 1;
        payloads.push(mint_payload);
    }

    let payloads_len = payloads.len();

    // Sending batch.
    let args: Vec<_> = payloads
        .into_iter()
        .map(|payload| ( FTOKEN.into(), payload, 250000000000, 0))
        .collect();

    let (ex_res, _)  =
        api.send_message_bytes_batch(args).await?;

    
    // Ids of initial messages.
    let mids: Vec<_> = ex_res
    .into_iter()
    .filter_map(|v| v.ok().map(|(mid, _pid)| mid))
    .collect();

    // Checking that all upload program calls succeed in batch.
    assert_eq!(payloads_len, mids.len());

    // Checking that all batch got processed.
    assert_eq!(
    payloads_len,
    listener.message_processed_batch(mids).await?.len(),
    );
    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    Ok(())
}
