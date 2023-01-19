#![no_std]
use gmeta::{InOut, Metadata};
use gstd::{prelude::*, ActorId};
use primitive_types::H256;

pub struct FStorageMetadata;

impl Metadata for FStorageMetadata {
    type Init = ();
    type Handle = InOut<FTStorageAction, FTStorageEvent>;
    type Others = ();
    type Reply = ();
    type Signal = ();
    type State = FTStorageState;
}

#[derive(Default, Encode, Decode, TypeInfo)]
pub struct FTStorageState {
    pub ft_logic_id: ActorId,
    pub transaction_status: Vec<(H256, bool)>,
    pub balances: Vec<(ActorId, u128)>,
    pub approvals: Vec<(ActorId, Vec<(ActorId, u128)>)>,
}

#[derive(Encode, Decode, Debug, Copy, Clone, TypeInfo)]
pub enum FTStorageAction {
    GetBalance(ActorId),
    IncreaseBalance {
        transaction_hash: H256,
        account: ActorId,
        amount: u128,
    },
    DecreaseBalance {
        transaction_hash: H256,
        msg_source: ActorId,
        account: ActorId,
        amount: u128,
    },
    Approve {
        transaction_hash: H256,
        msg_source: ActorId,
        account: ActorId,
        amount: u128,
    },
    Transfer {
        transaction_hash: H256,
        msg_source: ActorId,
        sender: ActorId,
        recipient: ActorId,
        amount: u128,
    },
}

#[derive(Encode, Decode, Clone, Debug, TypeInfo)]
pub enum FTStorageEvent {
    Ok,
    Err,
    Balance(u128),
}
