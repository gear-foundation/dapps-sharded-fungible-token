// This file is part of Gear.
//
// Copyright (C) 2021-2023 Gear Technologies Inc.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

#![cfg(test)]

use ft_main_io::{FTokenAction, InitFToken, LogicAction};
use gclient::{EventProcessor, GearApi, Result};
use gear_core::ids::{MessageId, ProgramId};
use gstd::{vec, Encode, Vec};
use rand::Rng;
use statrs::statistics::Statistics;
use std::fs;

pub mod utils;

/// This constant defines the number of messages in the batch.
/// It is calculated empirically, and 25 is considered the optimal value for
/// messages in this test. If the value were larger, transactions would
/// exhaust the block limits.
const BATCH_CHUNK_SIZE: usize = 25;
const MAX_GAS_LIMIT: u64 = 250_000_000_000;

async fn send_messages_in_parallel(
    api: &GearApi,
    batch_size: usize,
    treads_number: usize,
    messages: &[(ProgramId, Vec<u8>, u64, u128)],
) -> Result<Vec<MessageId>> {
    // TODO: currently have problem with transaction priorities from one user.
    // Fix this after loader become a lib #2781
    assert_eq!(treads_number, 1);

    let step_size = treads_number * batch_size;
    let mut message_ids = vec![];

    let mut step_no = 0;
    let steps_amount = messages.len() / step_size;

    for step in messages.chunks(step_size) {
        dbg!("send messages step {}/{}", step_no, steps_amount);
        step_no += 1;

        let tasks: Vec<_> = step
            .chunks(batch_size)
            .map(|batch| api.send_message_bytes_batch(batch.to_vec()))
            .collect();
        for res in futures::future::join_all(tasks).await {
            let (results, _) = res?;
            for res in results {
                let (msg_id, _) = res?;
                message_ids.push(msg_id);
            }
        }
    }

    Ok(message_ids)
}

#[tokio::test]
async fn stress_transfer() -> Result<()> {
    let mut rng = rand::thread_rng();

    // let api = GearApi::dev_from_path(GEAR_PATH).await?;
    // Use this code in comment for custom node run:
    let api = GearApi::dev().await?.with("//Alice")?;

    // Subscribing for events.
    let mut listener = api.subscribe().await?;

    // Checking that blocks still running.
    assert!(listener.blocks_running().await?);

    let ft_main_wasm = utils::current_wasm();
    let ft_storage_wasm =
        fs::read("../target/wasm32-unknown-unknown/release/ft_storage.opt.wasm").unwrap();
    let ft_logic_wasm =
        fs::read("../target/wasm32-unknown-unknown/release/ft_logic.opt.wasm").unwrap();

    dbg!("Upload ft storage code");
    let (storage_code_id, _) = api.upload_code(ft_storage_wasm).await?;
    dbg!("Upload ft logic code");
    let (ft_logic_code_id, _) = api.upload_code(ft_logic_wasm).await?;

    dbg!("Upload ft main program");
    let salt: u32 = rng.gen();
    let (message_id, program_id, _) = api
        .upload_program(
            ft_main_wasm,
            salt.to_le_bytes(),
            InitFToken {
                storage_code_hash: storage_code_id.into_bytes().into(),
                ft_logic_code_hash: ft_logic_code_id.into_bytes().into(),
            }
            .encode(),
            MAX_GAS_LIMIT,
            0,
        )
        .await?;

    assert!(listener.message_processed(message_id).await?.succeed());

    // Fill program with test users balances
    dbg!("Fill with test users balances");
    let users_amount = 200_000;

    let mut actions: Vec<LogicAction> = vec![];
    for user_id in 0u64..users_amount {
        actions.push(LogicAction::Mint {
            recipient: user_id.into(),
            amount: u64::MAX as u128,
        });
    }

    let messages: Vec<(_, Vec<u8>, u64, _)> = actions
        .into_iter()
        .enumerate()
        .map(|(id, action)| {
            (
                program_id,
                FTokenAction::Message {
                    transaction_id: id as u64,
                    payload: action,
                }
                .encode(),
                MAX_GAS_LIMIT,
                0,
            )
        })
        .collect();

    let message_ids = send_messages_in_parallel(&api, BATCH_CHUNK_SIZE, 1, &messages).await?;

    // Wait until messages are not processed
    if let Some((msg_id, status)) = listener
        .message_processed_batch(message_ids)
        .await?
        .into_iter()
        .find(|(_, status)| !status.succeed())
    {
        panic!(
            "{msg_id:?} ended with error status: {status:?}, may be need to decrease `step_size`"
        );
    };

    // Estimate gas for one transfer action
    let mut gas_burned = Vec::new();
    for id in 0u64..100 {
        let from: u64 = rng.gen_range(1..=users_amount);
        let to: u64 = rng.gen_range(1..=users_amount);
        let amount: u128 = rng.gen_range(1..=100);
        let action = FTokenAction::Message {
            transaction_id: id + 1_000_000,
            payload: LogicAction::Transfer {
                sender: from.into(),
                recipient: to.into(),
                amount,
            },
        };

        let burned = api
            .calculate_handle_gas(None, program_id, action.encode(), 0, false)
            .await
            .unwrap()
            .burned;
        gas_burned.push(burned as f64);
    }

    println!(
        "\n===================\n
        Gas burned for one transfer operation = {} * 10^9. \
        Calculated as geometric mean from 100 transfer operations.\n",
        gas_burned.geometric_mean() / 1_000_000_000f64
    );

    Ok(())
}
