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

use ft_logic_io::Action;
use ft_main_io::*;
use gclient::{EventListener, EventProcessor, GearApi, Result};
use gstd::CodeId;
use gstd::{prelude::*, ActorId};
use primitive_types::H256;
use std::time::Duration;

const PATHS: [&str; 3] = [
    "../target/wasm32-unknown-unknown/release/ft_main.wasm",
    "../target/wasm32-unknown-unknown/release/ft_logic.opt.wasm",
    "../target/wasm32-unknown-unknown/release/ft_storage.opt.wasm",
];

static mut STORAGE_ID: [u8; 32] = [0; 32];
static mut LOGIC_ID: [u8; 32] = [0; 32];
static mut TOKEN_ID: [u8; 32] = [0; 32];
async fn upload_programs_and_check(api: &GearApi, listener: &mut EventListener) -> Result<()> {
    // upload codes for main fungible token contract
    unsafe {
        // Upload ft storage code
        if let Ok((storage_id, _)) = api.upload_code(gclient::code_from_os(PATHS[0])?).await {
            STORAGE_ID = storage_id.into();
            println!("STORAGE ID {:?}", STORAGE_ID);
        }
        // Upload ft logic code
        if let Ok((logic_id, _)) = api.upload_code(gclient::code_from_os(PATHS[1])?).await {
            LOGIC_ID = logic_id.into();
            println!("STORAGE ID {:?}", LOGIC_ID);
        }
    }

    let init_ft_payload = unsafe {
        InitFToken {
            storage_code_hash: STORAGE_ID.into(),
            ft_logic_code_hash: LOGIC_ID.into(),
        }
        .encode()
    };
    let gas_info = api
        .calculate_upload_gas(
            None,
            gclient::code_from_os(PATHS[2])?,
            init_ft_payload.clone(),
            0,
            true,
            None,
        )
        .await?;

    // Program initialization.
    let (mid, _pid, _) = api
        .upload_program_bytes_by_path(
            PATHS[2],
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
async fn upload_codes() -> Result<()> {
    // Creating gear api.
    //
    // By default, login as Alice, than re-login as Bob.
    let api = GearApi::dev().await?.with("//Bob")?;
    let mut listener = api.subscribe().await?;

    upload_programs_and_check(&api, &mut listener).await?;

    let account: u64 = 1;
    let transaction_id: u64 = 0;
    let amount: u128 = 10_000;

    let mint_payload = FTokenAction::Message {
        transaction_id,
        payload: Action::Mint {
            recipient: account.into(),
            amount,
        }
        .encode(),
    };

    let gas_info = unsafe {
        api.calculate_handle_gas(None, TOKEN_ID.into(), mint_payload.encode(), 0, true, None)
            .await?
    };

    let (mid, _) = unsafe {
        api.send_message(TOKEN_ID.into(), mint_payload, gas_info.min_limit * 10  / 11, 0)
            .await?
    };

    // Asserting successful initialization.
    assert!(listener.message_processed(mid).await?.succeed());

    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    Ok(())
}

// #[tokio::test]
// async fn alloc_zero_pages() -> Result<()> {
//     let _ = env_logger::Builder::from_default_env()
//         .format_module_path(false)
//         .format_level(true)
//         .try_init();
//     log::info!("Begin");
//     let wat_code = r#"
//         (module
//             (import "env" "memory" (memory 0))
//             (import "env" "alloc" (func $alloc (param i32) (result i32)))
//             (export "init" (func $init))
//             (func $init
//                 i32.const 0
//                 call $alloc
//                 drop
//             )
//         )"#;
//     let api = GearApi::dev().await?.with("//Bob")?;
//     let codes = vec![wat::parse_str(wat_code).unwrap()];
//     let res = upload_programs_and_check(&api, codes, Some(Duration::from_secs(5))).await;
//     res
// }
