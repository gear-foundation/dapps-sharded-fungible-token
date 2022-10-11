#![no_std]
use ft_storage_io::*;
use gstd::{msg, prelude::*, ActorId};
use primitive_types::H256;

#[derive(Default)]
struct FTStorage {
    ft_logic_id: ActorId,
    transaction_status: BTreeMap<H256, bool>,
    balances: BTreeMap<ActorId, u128>,
    approvals: BTreeMap<ActorId, BTreeMap<ActorId, u128>>,
}

static mut FT_STORAGE: Option<FTStorage> = None;

impl FTStorage {
    fn get_balance(&self, account: &ActorId) {
        let balance = self.balances.get(account).unwrap_or(&0);
        msg::reply(FTStorageEvent::Balance(*balance), 0).expect("");
    }

    fn increase_balance(&mut self, transaction_hash: H256, account: &ActorId, amount: u128) {
        self.assert_ft_contract();

        // check transaction status
        if let Some(status) = self.transaction_status.get(&transaction_hash) {
            match status {
                true => reply_ok(),
                false => reply_err(),
            };
            return;
        }

        // increase balance
        self.balances
            .entry(*account)
            .and_modify(|balance| *balance = (*balance).saturating_add(amount))
            .or_insert(amount);

        self.transaction_status.insert(transaction_hash, true);
        msg::reply(FTStorageEvent::Ok, 0).expect("error in sending a reply `FTStorageEvent::Ok");
    }

    fn decrease_balance(
        &mut self,
        transaction_hash: H256,
        msg_source: &ActorId,
        account: &ActorId,
        amount: u128,
    ) {
        self.assert_ft_contract();
        // check transaction status
        if let Some(status) = self.transaction_status.get(&transaction_hash) {
            match status {
                true => reply_ok(),
                false => reply_err(),
            };
            return;
        }

        // decrease balance
        if let Some(balance) = self.balances.get_mut(account) {
            if *balance >= amount {
                if msg_source == account {
                    *balance -= amount;
                    self.transaction_status.insert(transaction_hash, true);
                    reply_ok();
                    return;
                } else if let Some(allowed_amount) = self
                    .approvals
                    .get_mut(account)
                    .and_then(|m| m.get_mut(msg_source))
                {
                    *balance -= amount;
                    *allowed_amount -= amount;
                    self.transaction_status.insert(transaction_hash, true);
                    reply_ok();
                    return;
                }
            }
        }

        self.transaction_status.insert(transaction_hash, false);
        reply_err();
    }

    fn assert_ft_contract(&self) {
        assert!(
            msg::source() == self.ft_logic_id,
            "Only fungible logic token contract is allowed to call that action"
        );
    }
}

#[no_mangle]
unsafe extern "C" fn handle() {
    let action: FTStorageAction = msg::load().expect("Error in loading `StorageAction`");
    let storage: &mut FTStorage = FT_STORAGE.get_or_insert(Default::default());
    match action {
        FTStorageAction::GetBalance(account) => storage.get_balance(&account),
        FTStorageAction::IncreaseBalance {
            transaction_hash,
            account,
            amount,
        } => storage.increase_balance(transaction_hash, &account, amount),
        FTStorageAction::DecreaseBalance {
            transaction_hash,
            msg_source,
            account,
            amount,
        } => storage.decrease_balance(transaction_hash, &msg_source, &account, amount),
    }
}

#[no_mangle]
unsafe extern "C" fn init() {
    let storage = FTStorage {
        ft_logic_id: msg::source(),
        ..Default::default()
    };
    FT_STORAGE = Some(storage);
}

#[no_mangle]
unsafe extern "C" fn meta_state() -> *mut [i32; 2] {
    let query: FTStorageState = msg::load().expect("Unable to decode `State");
    let storage: &mut FTStorage = FT_STORAGE.get_or_insert(Default::default());

    let encoded = match query {
        FTStorageState::Balance(account) => {
            let balance = storage.balances.get(&account).unwrap_or(&0);
            FTStorageStateReply::Balance(*balance)
        }
    }
    .encode();
    gstd::util::to_leak_ptr(encoded)
}

gstd::metadata! {
    title: "Storage Fungible Token contract",
    handle:
        input: FTStorageAction,
        output: FTStorageEvent,
    state:
        input: FTStorageState,
        output: FTStorageStateReply,
}

fn reply_ok() {
    msg::reply(FTStorageEvent::Ok, 0).expect("error in sending a reply `FTStorageEvent::Ok");
}

fn reply_err() {
    msg::reply(FTStorageEvent::Err, 0).expect("error in sending a reply `FTStorageEvent::Err");
}
