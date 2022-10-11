#![no_std]
use ft_logic_io::*;
use ft_main_io::*;
use gstd::{msg, prelude::*, prog::ProgramGenerator, ActorId};
use primitive_types::H256;

#[derive(Default)]
struct FToken {
    admin: ActorId,
    ft_logic_id: ActorId,
    transactions: BTreeMap<H256, TransactionStatus>,
}

static mut FTOKEN: Option<FToken> = None;

impl FToken {
    /// Accepts the payload message that will be sent to the logic token contract.
    ///
    /// Arguments:
    /// * `transaction_id`: the id of the transaction indicated by the actor that has sent that message;
    /// * `payload`: the message payload that will be sent to the logic token contract
    async fn message(&mut self, transaction_id: u64, payload: &Action) {
        // Get the transaction hash from `msg::source` and `transaction_id`
        // Tracking the trandaction ids is a responsibility of the account or programs that sent that transaction.
        let transaction_hash = get_hash(&msg::source(), transaction_id);
        let transaction = self.transactions.get(&transaction_hash);

        match transaction {
            None => {
                // If transaction took place for the first time we set its status to `InProgress`
                // and send message to the logic contract.
                self.transactions
                    .insert(transaction_hash, TransactionStatus::InProgress);
                let result = self.send_message(transaction_hash, payload).await;
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
                }
            }
            // The case when there was not enough gas to process the result of the message to the logic contract.
            Some(transaction_status) => match transaction_status {
                TransactionStatus::InProgress => {
                    let result = self.send_message(transaction_hash, payload).await;
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
                TransactionStatus::Success => {
                    reply_ok();
                }
                TransactionStatus::Failure => {
                    reply_err();
                }
            },
        }
    }

    async fn send_message(&self, transaction_hash: H256, payload: &Action) -> Result<(), ()> {
        let result = msg::send_for_reply_as::<_, FTLogicEvent>(
            self.ft_logic_id,
            FTLogicAction::Message {
                transaction_hash,
                account: msg::source(),
                payload: *payload,
            },
            0,
        )
        .expect("Error in sending a message to the fungible logic contract")
        .await;
        match result {
            Ok(reply) => match reply {
                FTLogicEvent::Ok => Ok(()),
                FTLogicEvent::Err => Err(()),
            },
            Err(_) => Err(()),
        }
    }

    fn update_logic_contract(&mut self, ft_logic_code_hash: H256, storage_code_hash: H256) {
        self.assert_admin();
        let ft_logic_id = ProgramGenerator::create_program(
            ft_logic_code_hash.into(),
            InitFTLogic {
                admin: msg::source(),
                storage_code_hash,
            }
            .encode(),
            0,
        )
        .expect("Error in creating FToken Logic program");
        self.ft_logic_id = ft_logic_id;
    }

    fn assert_admin(&self) {
        assert!(
            msg::source() == self.admin,
            "Only admin can send that message"
        );
    }
}

#[gstd::async_main]
async fn main() {
    let action: FTokenAction = msg::load().expect("Unable to decode `FTokenAction");
    let ftoken: &mut FToken = unsafe { FTOKEN.get_or_insert(Default::default()) };
    match action {
        FTokenAction::Message {
            transaction_id,
            payload,
        } => ftoken.message(transaction_id, &payload).await,
        FTokenAction::UpdateLogicContract {
            ft_logic_code_hash,
            storage_code_hash,
        } => ftoken.update_logic_contract(ft_logic_code_hash, storage_code_hash),
        _ => {}
    };
}

#[no_mangle]
unsafe extern "C" fn init() {
    let init_config: InitFToken = msg::load().expect("Unable to decode `InitFToken`");
    let ft_logic_id = ProgramGenerator::create_program(
        init_config.ft_logic_code_hash.into(),
        InitFTLogic {
            admin: msg::source(),
            storage_code_hash: init_config.storage_code_hash,
        }
        .encode(),
        0,
    )
    .expect("Error in creating FToken Logic program");
    let ftoken = FToken {
        admin: msg::source(),
        ft_logic_id,
        ..Default::default()
    };

    FTOKEN = Some(ftoken);
}

#[no_mangle]
unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let ftoken = FTOKEN.get_or_insert(Default::default());
    let query: FTokenState = msg::load().expect("Unable to decode `FTokenState`");
    let encoded = match query {
        FTokenState::TransactionStatus(account, transaction_id) => {
            let transaction_hash = get_hash(&account, transaction_id);
            let transaction = ftoken.transactions.get(&transaction_hash);
            FTokenStateReply::TransactionStatus(transaction.copied())
        }
        FTokenState::FTLogicId => FTokenStateReply::FTLogicId(ftoken.ft_logic_id),
    }
    .encode();
    gstd::util::to_leak_ptr(encoded)
}

fn reply_ok() {
    msg::reply(FTokenEvent::Ok, 0).expect("Error in a reply `FTokenEvent::Ok`");
}

fn reply_err() {
    msg::reply(FTokenEvent::Err, 0).expect("Error in a reply `FTokenEvent::Ok`");
}

gstd::metadata! {
    title: "Main Fungible Token contract",
    init:
        input: InitFToken,
    handle:
        input: FTokenAction,
        output: FTokenEvent,
    state:
        input: FTokenState,
        output: FTokenStateReply,
}

pub fn get_hash(account: &ActorId, transaction_id: u64) -> H256 {
    let account: Vec<u8> = <[u8; 32]>::from(*account).into();
    let transaction_id = transaction_id.to_be_bytes();
    sp_core_hashing::blake2_256(&[&account[..], &transaction_id[..]].concat()).into()
}
