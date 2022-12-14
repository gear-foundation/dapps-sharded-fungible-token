pub mod utils;
use gtest::{Program, System};
use utils::*;

const ACCOUNTS_AMOUNT: u64 = 1_000;

#[test]
fn high_load_mint() {
    let system = System::new();
    system.init_logger();
    let mut transaction_id: u64 = 100;
    let amount: u128 = 100_000;
    let ftoken = Program::ftoken(&system);

    while transaction_id < ACCOUNTS_AMOUNT {
        // Mint tokens to account and check it
        println!("id is {transaction_id}");
        ftoken.mint(
            transaction_id,
            transaction_id,
            transaction_id,
            amount,
            false,
        );
        ftoken.check_balance(transaction_id, amount);
        transaction_id += 1;
    }
}
