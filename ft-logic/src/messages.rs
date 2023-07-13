use crate::H256;
use ft_storage_io::{FTStorageAction, FTStorageEvent};
use gstd_fluent::{self as builder, gstd::ActorId};

pub async fn increase_balance(
    transaction_hash: H256,
    storage_id: &ActorId,
    account: &ActorId,
    amount: u128,
) -> Result<(), ()> {
    let result = builder::send(
        *storage_id,
        FTStorageAction::IncreaseBalance {
            transaction_hash,
            account: *account,
            amount,
        },
    )
    .for_reply_as::<FTStorageEvent>()
    .execute()
    .expect("Error in sending a message `FTStorageAction::IncreaseBalance`")
    .await;
    match result {
        Ok(storage_event) => match storage_event {
            FTStorageEvent::Ok => Ok(()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

pub async fn decrease_balance(
    transaction_hash: H256,
    storage_id: &ActorId,
    msg_source: &ActorId,
    account: &ActorId,
    amount: u128,
) -> Result<(), ()> {
    let result = builder::send(
        *storage_id,
        FTStorageAction::DecreaseBalance {
            transaction_hash,
            msg_source: *msg_source,
            account: *account,
            amount,
        },
    )
    .for_reply_as::<FTStorageEvent>()
    .execute()
    .expect("Error in sending a message `FTStorageAction::DecreaseBalance`")
    .await;
    match result {
        Ok(storage_event) => match storage_event {
            FTStorageEvent::Ok => Ok(()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

pub async fn approve(
    transaction_hash: H256,
    storage_id: &ActorId,
    msg_source: &ActorId,
    account: &ActorId,
    amount: u128,
) -> Result<(), ()> {
    let result = builder::send(
        *storage_id,
        FTStorageAction::Approve {
            transaction_hash,
            msg_source: *msg_source,
            account: *account,
            amount,
        },
    )
    .for_reply_as::<FTStorageEvent>()
    .execute()
    .expect("Error in sending a message `FTStorageAction::DecreaseBalance`")
    .await;
    match result {
        Ok(storage_event) => match storage_event {
            FTStorageEvent::Ok => Ok(()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

pub async fn transfer(
    transaction_hash: H256,
    storage_id: &ActorId,
    msg_source: &ActorId,
    sender: &ActorId,
    recipient: &ActorId,
    amount: u128,
) -> Result<(), ()> {
    let result = builder::send(
        *storage_id,
        FTStorageAction::Transfer {
            transaction_hash,
            msg_source: *msg_source,
            sender: *sender,
            recipient: *recipient,
            amount,
        },
    )
    .for_reply_as::<FTStorageEvent>()
    .execute()
    .expect("Error in sending a message `FTStorageAction::Transfer`")
    .await;
    match result {
        Ok(storage_event) => match storage_event {
            FTStorageEvent::Ok => Ok(()),
            _ => Err(()),
        },
        Err(_) => Err(()),
    }
}

pub async fn get_permit_id(storage_id: &ActorId, account: &ActorId) -> u128 {
    let reply = builder::send(*storage_id, FTStorageAction::GetPermitId(*account))
        .for_reply_as::<FTStorageEvent>()
        .execute()
        .expect("Error in sending a message `FTStorageAction::GetPermitId")
        .await
        .expect("Unable to decode `FTStorageEvent");
    if let FTStorageEvent::PermitId(permit_id) = reply {
        permit_id
    } else {
        0
    }
}

pub async fn check_and_increment_permit_id(
    storage_id: &ActorId,
    transaction_hash: H256,
    account: &ActorId,
    expected_permit_id: u128,
) -> bool {
    let reply = builder::send(
        *storage_id,
        FTStorageAction::IncrementPermitId {
            transaction_hash,
            account: *account,
            expected_permit_id,
        },
    )
    .for_reply_as::<FTStorageEvent>()
    .execute()
    .expect("Error in sending a message `FTStorageAction::IncrementPermitId")
    .await
    .expect("Unable to decode `FTStorageEvent");
    if let FTStorageEvent::Ok = reply {
        return true;
    }
    false
}

pub async fn get_balance(storage_id: &ActorId, account: &ActorId) -> u128 {
    let reply = builder::send(*storage_id, FTStorageAction::GetBalance(*account))
        .for_reply_as::<FTStorageEvent>()
        .execute()
        .expect("Error in sending a message `FTStorageAction::GetBalance")
        .await
        .expect("Unable to decode `FTStorageEvent");
    if let FTStorageEvent::Balance(balance) = reply {
        balance
    } else {
        0
    }
}
