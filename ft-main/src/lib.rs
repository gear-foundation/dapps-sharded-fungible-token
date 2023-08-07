#![no_std]
use ft_logic_io::{FTLogicAction, FTLogicEvent, InitFTLogic};
use ft_main_io::*;
use gstd_fluent::{
    self as builder,
    gstd::{self, exec, msg, prelude::*, ActorId},
};
use hashbrown::HashMap;
use primitive_types::H256;

const DELAY: u32 = 600_000;

#[derive(Default)]
struct FToken {
    admin: ActorId,
    ft_logic_id: ActorId,
    transactions: HashMap<H256, TransactionStatus>,
}

static mut FTOKEN: Option<FToken> = None;

impl FToken {
    /// Accepts the payload message that will be sent to the logic token contract.
    ///
    /// Arguments:
    /// * `transaction_id`: the id of the transaction indicated by the actor that has sent that message;
    /// * `payload`: the message payload that will be sent to the logic token contract
    async fn message(&mut self, transaction_id: u64, payload: &[u8]) {
        // Get the transaction hash from `msg::source` and `transaction_id`
        // Tracking the trandaction ids is a responsibility of the account or programs that sent that transaction.
        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let transaction = self.transactions.get(&transaction_hash);

        match transaction {
            None => {
                // If transaction took place for the first time we set its status to `InProgress`
                // and send message to the logic contract.
                send_delayed_clear(transaction_hash);
                self.transactions
                    .insert(transaction_hash, TransactionStatus::InProgress);
                self.send_message_then_reply(transaction_hash, payload)
                    .await;
            }
            // The case when there was not enough gas to process the result of the message to the logic contract.
            Some(transaction_status) => match transaction_status {
                TransactionStatus::InProgress => {
                    self.send_message_then_reply(transaction_hash, payload)
                        .await;
                }
                TransactionStatus::Success => {
                    reply_ok();
                }
                TransactionStatus::Failure => {
                    reply_err();
                }
            },
        }
    }

    async fn send_message_then_reply(&mut self, transaction_hash: H256, payload: &[u8]) {
        let result = self.send_message(transaction_hash, payload).await;
        //debug!("REPLY");
        match result {
            Ok(()) => {
                self.transactions
                    .insert(transaction_hash, TransactionStatus::Success);
                reply_ok();
            }
            Err(()) => {
                self.transactions
                    .insert(transaction_hash, TransactionStatus::Failure);
                reply_err();
            }
        };
    }

    async fn send_message(&self, transaction_hash: H256, payload: &[u8]) -> Result<(), ()> {
        let result = builder::send(
            self.ft_logic_id,
            FTLogicAction::Message {
                transaction_hash,
                account: msg::source(),
                payload: payload.to_vec(),
            },
        )
        .for_reply_as::<FTLogicEvent>()
        .execute()
        .expect("Error in sending a message to the fungible logic contract")
        .await;
        match result {
            Ok(FTLogicEvent::Ok) => Ok(()),
            _ => Err(()),
        }
    }

    async fn get_balance(&self, account: &ActorId) {
        let reply = builder::send(self.ft_logic_id, FTLogicAction::GetBalance(*account))
            .for_reply_as::<FTLogicEvent>()
            .execute()
            .expect("Error in sending a message `FTLogicGetBalance")
            .await
            .expect("Unable to decode `FTLogicEvent");
        if let FTLogicEvent::Balance(balance) = reply {
            builder::reply(FTokenEvent::Balance(balance))
                .execute()
                .expect("Error in a reply `FTokenEvent::Balance`");
        }
    }

    async fn get_permit_id(&self, account: &ActorId) {
        let reply = builder::send(self.ft_logic_id, FTLogicAction::GetPermitId(*account))
            .for_reply_as::<FTLogicEvent>()
            .execute()
            .expect("Error in sending a message `FTLogic::GetPermitId")
            .await
            .expect("Unable to decode `FTLogicEvent");
        if let FTLogicEvent::PermitId(permit_id) = reply {
            builder::reply(FTokenEvent::PermitId(permit_id))
                .execute()
                .expect("Error in a reply `FTokenEvent::PermitId`");
        }
    }

    fn update_logic_contract(&mut self, ft_logic_code_hash: H256, storage_code_hash: H256) {
        self.assert_admin();
        let (_message_id, ft_logic_id) = builder::create_program(
            ft_logic_code_hash.into(),
            InitFTLogic {
                admin: msg::source(),
                storage_code_hash,
            }
            .encode(),
        )
        .execute()
        .expect("Error in creating FToken Logic program");
        self.ft_logic_id = ft_logic_id;
    }

    fn assert_admin(&self) {
        assert!(
            msg::source() == self.admin,
            "Only admin can send that message"
        );
    }

    fn clear(&mut self, transaction_hash: H256) {
        self.transactions.remove(&transaction_hash);
    }
}

#[gstd::async_main]
async fn main() {
    let bytes = msg::load_bytes().expect("Unable to load bytes");
    let ftoken: &mut FToken = unsafe { FTOKEN.as_mut().expect("The contract is not initialized") };

    if bytes[0] == 0 {
        let array: [u8; 8] = bytes[1..=8]
            .try_into()
            .expect("Unable to get an array from slice");
        let transaction_id = u64::from_ne_bytes(array);
        let payload: Vec<u8> = bytes[9..].to_vec();
        ftoken.message(transaction_id, &payload).await;
    } else {
        let action = FTokenInnerAction::decode(&mut &bytes[..])
            .expect("Unable to decode `FTokenInnerAction`");
        match action {
            FTokenInnerAction::UpdateLogicContract {
                ft_logic_code_hash,
                storage_code_hash,
            } => ftoken.update_logic_contract(ft_logic_code_hash, storage_code_hash),
            FTokenInnerAction::Clear(transaction_hash) => ftoken.clear(transaction_hash),
            FTokenInnerAction::GetBalance(account) => ftoken.get_balance(&account).await,
            FTokenInnerAction::GetPermitId(account) => ftoken.get_permit_id(&account).await,
            _ => {}
        }
    }
}

#[no_mangle]
unsafe extern "C" fn init() {
    let init_config: InitFToken = msg::load().expect("Unable to decode `InitFToken`");
    let (_message_id, ft_logic_id) = builder::create_program(
        init_config.ft_logic_code_hash.into(),
        InitFTLogic {
            admin: msg::source(),
            storage_code_hash: init_config.storage_code_hash,
        }
        .encode(),
    )
    .execute()
    .expect("Error in creating FToken Logic program");
    let ftoken = FToken {
        admin: msg::source(),
        ft_logic_id,
        ..Default::default()
    };

    FTOKEN = Some(ftoken);
}

fn reply_ok() {
    builder::reply(FTokenEvent::Ok)
        .execute()
        .expect("Error in a reply `FTokenEvent::Ok`");
}

fn reply_err() {
    builder::reply(FTokenEvent::Err)
        .execute()
        .expect("Error in a reply `FTokenEvent::Ok`");
}

pub fn get_hash(account: &ActorId, transaction_id: u64) -> H256 {
    let account: [u8; 32] = (*account).into();
    let transaction_id = transaction_id.to_be_bytes();
    sp_core_hashing::blake2_256(&[account.as_slice(), transaction_id.as_slice()].concat()).into()
}

fn send_delayed_clear(transaction_hash: H256) {
    builder::send(exec::program_id(), FTokenAction::Clear(transaction_hash))
        .with_delay(DELAY)
        .execute()
        .expect("Error in sending a delayled message `FTStorageAction::Clear`");
}

#[no_mangle]
extern "C" fn state() {
    let token = unsafe { FTOKEN.as_ref().expect("FToken is not initialized") };
    let token_state = FTokenState {
        admin: token.admin,
        ft_logic_id: token.ft_logic_id,
        transactions: token
            .transactions
            .iter()
            .map(|(key, value)| (*key, *value))
            .collect(),
    };
    builder::reply(token_state)
        .execute()
        .expect("Failed to share state");
}
