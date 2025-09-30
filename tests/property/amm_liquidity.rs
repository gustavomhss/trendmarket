use credit_engine_core::amm::errors::AmmError;
use credit_engine_core::amm::liquidity;
use credit_engine_core::amm::types::{Wad, MIN_RESERVE, WAD};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, RngSeed, TestCaseError, TestRunner};

const DEFAULT_PROPTEST_CASES: u32 = 512;
const BASE_SEED: u64 = 0x0CE0_7E57_D3AD_F00D;

fn proptest_cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .filter(|&cases| cases > 0)
        .unwrap_or(DEFAULT_PROPTEST_CASES)
}

fn base_seed() -> u64 {
    std::env::var("PROPTEST_BASE_SEED")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(BASE_SEED)
}

fn runner_for(test_name: &str, offset: u64) -> TestRunner {
    let mut config = ProptestConfig::default();
    config.cases = proptest_cases();
    config.failure_persistence = None;
    config.rng_seed = RngSeed::Fixed(base_seed().wrapping_add(offset));
    let runner = TestRunner::new(config);
    if let RngSeed::Fixed(seed) = runner.config().rng_seed {
        println!("seed:{} test={}", seed, test_name);
    }
    runner
}

fn fail_with(context: &str, err: AmmError) -> TestCaseError {
    TestCaseError::fail(format!("{context}: {err:?}"))
}

fn reject(reason: &str) -> TestCaseError {
    TestCaseError::reject(reason.to_string())
}

fn arb_reserve() -> impl Strategy<Value = Wad> {
    (200_000u128..=2_000_000u128).prop_map(|units| units * WAD)
}

fn arb_candidate_reserve() -> impl Strategy<Value = Wad> {
    (0u128..=2_000_000u128).prop_map(|units| units * WAD)
}

fn arb_deposit() -> impl Strategy<Value = Wad> {
    (1_000u128..=50_000u128).prop_map(|units| units * WAD)
}

#[test]
fn initial_mint_respects_domain_guards() {
    let strategy = (arb_candidate_reserve(), arb_candidate_reserve());
    let mut runner = runner_for("property::liquidity::initial_mint_domain", 101);

    runner
        .run(&strategy, |(x, y)| {
            let result = liquidity::initial_mint(x, y);
            let zero_reserve = x == 0 || y == 0;
            let below_min = x < MIN_RESERVE || y < MIN_RESERVE;

            match result {
                Ok(shares) => {
                    prop_assert!(!zero_reserve && !below_min);
                    prop_assert!(shares > 0);
                }
                Err(err) => {
                    if zero_reserve {
                        prop_assert_eq!(err, AmmError::ZeroReserve);
                    } else if below_min {
                        prop_assert_eq!(err, AmmError::MinReserveBreached);
                    } else {
                        return Err(fail_with("initial_mint unexpected", err));
                    }
                }
            }

            Ok(())
        })
        .expect("property run should succeed");
}

#[test]
fn add_then_remove_returns_near_original_state() {
    let strategy = (arb_reserve(), arb_reserve(), arb_deposit(), arb_deposit());
    let mut runner = runner_for("property::liquidity::add_remove_roundtrip", 137);

    runner
        .run(&strategy, |(x, y, dx, dy)| {
            let total_shares = match liquidity::initial_mint(x, y) {
                Ok(value) => value,
                Err(err) => return Err(fail_with("initial_mint", err)),
            };
            prop_assume!(total_shares > 0);

            let minted = match liquidity::add_liquidity(x, y, dx, dy, total_shares) {
                Ok(value) => value,
                Err(AmmError::InputTooSmall) => {
                    return Err(reject("input too small"));
                }
                Err(err) => {
                    return Err(fail_with("add_liquidity", err));
                }
            };
            prop_assume!(minted > 0);

            let new_total = total_shares
                .checked_add(minted)
                .ok_or_else(|| fail_with("total shares overflow", AmmError::Overflow))?;
            let x_after_add = x
                .checked_add(dx)
                .ok_or_else(|| fail_with("reserve overflow (x)", AmmError::Overflow))?;
            let y_after_add = y
                .checked_add(dy)
                .ok_or_else(|| fail_with("reserve overflow (y)", AmmError::Overflow))?;

            let (x_out, y_out) =
                match liquidity::remove_liquidity(x_after_add, y_after_add, minted, new_total) {
                    Ok(amounts) => amounts,
                    Err(err) => {
                        return Err(fail_with("remove_liquidity", err));
                    }
                };

            prop_assert!(x_out <= dx);
            prop_assert!(y_out <= dy);

            let x_remaining = x_after_add
                .checked_sub(x_out)
                .ok_or_else(|| fail_with("post-remove underflow (x)", AmmError::Overflow))?;
            let y_remaining = y_after_add
                .checked_sub(y_out)
                .ok_or_else(|| fail_with("post-remove underflow (y)", AmmError::Overflow))?;

            prop_assert!(x_remaining >= x);
            prop_assert!(y_remaining >= y);

            let delta_x = x_remaining
                .checked_sub(x)
                .ok_or_else(|| fail_with("delta underflow (x)", AmmError::Overflow))?;
            let delta_y = y_remaining
                .checked_sub(y)
                .ok_or_else(|| fail_with("delta underflow (y)", AmmError::Overflow))?;

            prop_assert!(delta_x <= dx);
            prop_assert!(delta_y <= dy);
            prop_assert!(x_remaining >= MIN_RESERVE);
            prop_assert!(y_remaining >= MIN_RESERVE);

            Ok(())
        })
        .expect("property run should succeed");
}

#[test]
fn removing_all_shares_trips_min_reserve_guard() {
    let strategy = (arb_reserve(), arb_reserve());
    let mut runner = runner_for("property::liquidity::remove_all_guard", 173);

    runner
        .run(&strategy, |(x, y)| {
            let total_shares = match liquidity::initial_mint(x, y) {
                Ok(value) => value,
                Err(err) => return Err(fail_with("initial_mint", err)),
            };
            prop_assume!(total_shares > 0);

            let err = liquidity::remove_liquidity(x, y, total_shares, total_shares)
                .expect_err("removing all shares must fail");
            prop_assert_eq!(err, AmmError::MinReserveBreached);

            Ok(())
        })
        .expect("property run should succeed");
}
