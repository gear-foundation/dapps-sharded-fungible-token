#![no_std]
use ft_logic_io::*;
use ft_storage_io::*;
use gstd::{debug, msg, prelude::*, prog::ProgramGenerator, ActorId};
mod instruction;
use instruction::*;
use primitive_types::H256;

const GAS_STORAGE_CREATION: u64 = 3_000_000_000;

#[derive(Default)]
struct FTLogic {
    admin: ActorId,
    ftoken_id: ActorId,
    transaction_status: BTreeMap<H256, TransactionStatus>,
    instructions: BTreeMap<H256, (Instruction, Instruction)>,
    storage_code_hash: H256,
    id_to_storage: BTreeMap<String, ActorId>,
}

static mut FT_LOGIC: Option<FTLogic> = None;

pub enum TransactionStatus {
    InProgress,
    Success,
    Failure,
}

impl FTLogic {
    /// The message received from the main contract.
    ///
    /// Arguments:
    /// * `transaction_hash`: the hash associated with that transaction;
    /// * `account`: the account that sent the message to the main contract;
    /// * `action`: the message payload.
    async fn message(&mut self, transaction_hash: H256, account: &ActorId, action: &Action) {
        self.assert_main_contract();
        let transaction_status = self
            .transaction_status
            .get(&transaction_hash)
            .unwrap_or(&TransactionStatus::InProgress);

        match transaction_status {
            // The transaction has already been made but there wasn't enough gas for a message reply.
            TransactionStatus::Success => reply_ok(),
            TransactionStatus::Failure => reply_err(),
            // The transaction took place for the first time
            // Or there was not enough gas to change the `TransactionStatus`.
            TransactionStatus::InProgress => match action {
                Action::Mint { recipient, amount } => {
                    self.mint(transaction_hash, recipient, amount).await;
                }
                Action::Transfer {
                    sender,
                    recipient,
                    amount,
                } => {
                    self.transfer(transaction_hash, account, sender, recipient, *amount)
                        .await;
                }
            },
        }
    }

    async fn mint(&mut self, transaction_hash: H256, recipient: &ActorId, amount: &u128) {
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::InProgress);
        let recipient_storage = self.get_storage_address(recipient);

        let result =
            increase_balance(transaction_hash, &recipient_storage, recipient, *amount).await;

        match result {
            Ok(()) => {
                self.transaction_status
                    .insert(transaction_hash, TransactionStatus::Success);
                reply_ok()
            }
            Err(()) => {
                self.transaction_status
                    .insert(transaction_hash, TransactionStatus::Failure);
                reply_err();
            }
        }
    }

    async fn transfer(
        &mut self,
        transaction_hash: H256,
        msg_source: &ActorId,
        sender: &ActorId,
        recipient: &ActorId,
        amount: u128,
    ) {
        self.transaction_status
            .insert(transaction_hash, TransactionStatus::InProgress);
        let sender_storage = self.get_storage_address(sender);
        let recipient_storage = self.get_storage_address(recipient);

        self.instructions
            .entry(transaction_hash)
            .or_insert_with(|| {
                let decrease_instruction = create_decrease_instruction(
                    transaction_hash,
                    msg_source,
                    &sender_storage,
                    sender,
                    amount,
                );
                let increase_instruction = create_increase_instruction(
                    transaction_hash,
                    &recipient_storage,
                    recipient,
                    amount,
                );
                (decrease_instruction, increase_instruction)
            });

        let (decrease_instruction, increase_instruction) = self
            .instructions
            .get_mut(&transaction_hash)
            .expect("Can't be `None`: Instructions must exist");

        if decrease_instruction.start().await.is_err() {
            self.transaction_status
                .insert(transaction_hash, TransactionStatus::Failure);
            reply_err();
            return;
        }

        match increase_instruction.start().await {
            Err(_) => {
                if decrease_instruction.abort().await.is_ok() {
                    self.transaction_status
                        .insert(transaction_hash, TransactionStatus::Failure);
                    reply_err();
                }
            }
            Ok(_) => {
                self.transaction_status
                    .insert(transaction_hash, TransactionStatus::Success);
                reply_ok();
            }
        }
    }

    fn update_storage_hash(&mut self, storage_code_hash: H256) {
        self.assert_admin();
        self.storage_code_hash = storage_code_hash;
    }

    fn get_storage_address(&mut self, address: &ActorId) -> ActorId {
        let encoded = hex::encode(address.as_ref());
        let id: String = encoded.chars().next().expect("Can't be None").to_string();
        if let Some(address) = self.id_to_storage.get(&id) {
            *address
        } else {
            let address = ProgramGenerator::create_program_with_gas(
                self.storage_code_hash.into(),
                "",
                GAS_STORAGE_CREATION,
                0,
            )
            .expect("Error in creating Storage program");
            self.id_to_storage.insert(id, address);
            address
        }
    }

    fn assert_main_contract(&self) {
        assert!(
            self.ftoken_id == msg::source(),
            "Only main fungible token contract can send that message"
        );
    }

    fn assert_admin(&self) {
        assert!(
            self.admin == msg::source(),
            "Only admin can send that message"
        );
    }
}

#[gstd::async_main]
async fn main() {
    let action: FTLogicAction = msg::load().expect("Error in loading `StorageAction`");
    let logic: &mut FTLogic = unsafe { FT_LOGIC.get_or_insert(Default::default()) };
    match action {
        FTLogicAction::Message {
            transaction_hash,
            account,
            payload,
        } => logic.message(transaction_hash, &account, &payload).await,
        FTLogicAction::UpdateStorageCodeHash(storage_code_hash) => {
            logic.update_storage_hash(storage_code_hash)
        }
        _ => {}
    }
}

#[no_mangle]
unsafe extern "C" fn init() {
    debug!("INIT FT LOFIC");
    let init_config: InitFTLogic = msg::load().expect("Unable to decode `InitFTLogic`");
    let ft_logic = FTLogic {
        admin: init_config.admin,
        storage_code_hash: init_config.storage_code_hash,
        ftoken_id: msg::source(),
        ..Default::default()
    };
    FT_LOGIC = Some(ft_logic);
}

fn reply_err() {
    msg::reply(FTLogicEvent::Err, 0).expect("Error in sending a reply `FTLogicEvent::Err`");
}

fn reply_ok() {
    msg::reply(FTLogicEvent::Ok, 0).expect("Error in sending a reply `FTLogicEvent::Ok`");
}

async fn increase_balance(
    transaction_hash: H256,
    storage_id: &ActorId,
    account: &ActorId,
    amount: u128,
) -> Result<(), ()> {
    let result = msg::send_for_reply_as::<_, FTStorageEvent>(
        *storage_id,
        FTStorageAction::IncreaseBalance {
            transaction_hash,
            account: *account,
            amount,
        },
        0,
    )
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

#[no_mangle]
unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: FTLogicState = msg::load().expect("Unable to decode `State");
    let logic: &mut FTLogic = FT_LOGIC.get_or_insert(Default::default());

    let encoded = match query {
        FTLogicState::Storages => FTLogicStateReply::Storages(logic.id_to_storage.clone()),
    }
    .encode();
    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "Logic Fungible Token contract",
    handle:
        input: FTLogicAction,
        output: FTLogicEvent,
    state:
        input: FTLogicState,
        output: FTLogicStateReply,
}
