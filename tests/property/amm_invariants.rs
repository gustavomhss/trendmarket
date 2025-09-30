use credit_engine_core::amm::errors::AmmError;
use credit_engine_core::amm::guardrails::u256_to_u128_checked;
use credit_engine_core::amm::pricing;
use credit_engine_core::amm::swap;
use credit_engine_core::amm::types::{Ppm, Wad, MIN_RESERVE, PPM_SCALE, U256, WAD};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, RngSeed, TestCaseError, TestRunner};

const DEFAULT_PROPTEST_CASES: u32 = 512;
const BASE_SEED: u64 = 0x0CE0_7E57_D3AD_F00D;
const FRACTION_DENOMINATOR: u64 = 10_000;

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

fn fee_on_input_ceil(dx: Wad, fee_ppm: Ppm) -> Result<Wad, AmmError> {
    if fee_ppm == 0 {
        return Ok(0);
    }
    let numerator = U256::from(dx) * U256::from(fee_ppm as u64);
    let denominator = U256::from(PPM_SCALE as u64);
    let fee = (numerator + (denominator - U256::from(1u8))) / denominator;
    u256_to_u128_checked(fee)
}

fn arb_reserve() -> impl Strategy<Value = Wad> {
    (200_000u128..=2_000_000u128).prop_map(|units| units * WAD)
}

fn arb_small_reserve() -> impl Strategy<Value = Wad> {
    (50_000u128..=500_000u128).prop_map(|units| units * WAD)
}

fn arb_positive_dx() -> impl Strategy<Value = Wad> {
    (25u128..=200_000u128).prop_map(|units| units * WAD)
}

fn arb_fee() -> impl Strategy<Value = Ppm> {
    (0u32..=3_000u32).prop_map(|fee| fee as Ppm)
}

fn fraction_of(value: Wad, numerator: u64) -> Wad {
    if value == 0 || numerator == 0 {
        return 0;
    }
    let scaled = U256::from(value) * U256::from(numerator);
    let denominator = U256::from(FRACTION_DENOMINATOR);
    let quotient = scaled / denominator;
    u256_to_u128_checked(quotient).unwrap_or(0)
}

#[test]
fn zero_inputs_rejected_and_outputs_positive() {
    let strategy = (arb_reserve(), arb_reserve(), arb_positive_dx(), arb_fee());
    let mut runner = runner_for("property::amm::zero_inputs_rejected", 0);

    runner
        .run(&strategy, |(x, y, dx, fee)| {
            prop_assert!(x >= MIN_RESERVE && y >= MIN_RESERVE);

            let zero_out = swap::get_amount_out(x, y, 0, fee).unwrap_err();
            prop_assert_eq!(zero_out, AmmError::ZeroAmount);

            let zero_in = swap::get_amount_in(x, y, 0, fee).unwrap_err();
            prop_assert_eq!(zero_in, AmmError::ZeroAmount);

            match swap::get_amount_out(x, y, dx, fee) {
                Ok(dy) => {
                    prop_assert!(dy > 0);
                    prop_assert!(dy <= y);
                }
                Err(err) => {
                    prop_assert!(matches!(
                        err,
                        AmmError::InputTooSmall | AmmError::MinReserveBreached
                    ));
                }
            }

            Ok(())
        })
        .expect("property run should succeed");
}

#[test]
fn guardrails_preserve_min_reserve_on_success() {
    let strategy = (arb_reserve(), arb_reserve(), arb_positive_dx(), arb_fee());
    let mut runner = runner_for("property::amm::guardrails_preserve_min_reserve", 17);

    runner
        .run(&strategy, |(x, y, dx, fee)| {
            let out = match swap::get_amount_out(x, y, dx, fee) {
                Ok(dy) => dy,
                Err(AmmError::InputTooSmall) => {
                    return Err(reject("input too small"));
                }
                Err(AmmError::MinReserveBreached) => {
                    return Err(reject("min reserve breached"));
                }
                Err(other) => {
                    return Err(fail_with("get_amount_out", other));
                }
            };

            let dx_fee =
                fee_on_input_ceil(dx, fee).map_err(|err| fail_with("fee_on_input_ceil", err))?;
            let dx_net = dx.checked_sub(dx_fee).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;
            let x_after = x.checked_add(dx_net).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;
            let y_after = y.checked_sub(out).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;

            prop_assert!(x_after >= MIN_RESERVE);
            prop_assert!(y_after >= MIN_RESERVE);

            Ok(())
        })
        .expect("property run should succeed");
}

#[test]
fn constant_product_roundtrip_is_lossy() {
    let strategy = (arb_reserve(), arb_reserve(), arb_positive_dx(), arb_fee());
    let mut runner = runner_for("property::amm::roundtrip_lossy", 33);

    runner
        .run(&strategy, |(x, y, dx, fee)| {
            let out = match swap::get_amount_out(x, y, dx, fee) {
                Ok(dy) => dy,
                Err(AmmError::InputTooSmall) => {
                    return Err(reject("input too small"));
                }
                Err(AmmError::MinReserveBreached) => {
                    return Err(reject("min reserve breached"));
                }
                Err(err) => {
                    return Err(fail_with("swap execution", err));
                }
            };
            prop_assume!(out > 0);

            let dx_fee =
                fee_on_input_ceil(dx, fee).map_err(|err| fail_with("fee_on_input_ceil", err))?;
            let dx_net = dx.checked_sub(dx_fee).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;
            let x_after = x.checked_add(dx_net).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;
            let y_after = y.checked_sub(out).ok_or_else(|| {
                fail_with("overflow while adjusting reserves", AmmError::Overflow)
            })?;
            prop_assume!(x_after >= MIN_RESERVE && y_after >= MIN_RESERVE);

            match swap::get_amount_out(y_after, x_after, out, fee) {
                Ok(back_dx) => {
                    prop_assert!(back_dx <= dx);
                }
                Err(AmmError::InputTooSmall) => {
                    // round-trip producing a value too small still respects non-profitable invariant
                    prop_assert!(dx > 0);
                }
                Err(err) => {
                    return Err(fail_with("swap execution", err));
                }
            }

            let k0 = U256::from(x) * U256::from(y);
            let k1 = U256::from(x_after) * U256::from(y_after);
            let delta = if k1 >= k0 { k1 - k0 } else { k0 - k1 };
            let tolerance = (k0 / U256::from(1_000_000_000u64)) + U256::from(1u8);
            if fee == 0 {
                prop_assert!(delta <= tolerance);
            } else if k1 < k0 {
                prop_assert!(delta <= tolerance);
            }

            Ok(())
        })
        .expect("property run should succeed");
}

#[test]
fn execution_price_is_monotonic_in_trade_size() {
    let strategy = (
        arb_small_reserve(),
        arb_small_reserve(),
        (50u128..=20_000u128, 20_001u128..=80_000u128),
        arb_fee(),
    );
    let mut runner = runner_for("property::amm::execution_price_monotonic", 51);

    runner
        .run(
            &strategy,
            |(x, y, (dx_small_units, dx_large_units), fee)| {
                let x = x;
                let y = y;
                let dx_small = dx_small_units * WAD;
                let dx_large = dx_large_units * WAD;

                let out_small = match swap::get_amount_out(x, y, dx_small, fee) {
                    Ok(value) => value,
                    Err(AmmError::InputTooSmall) => {
                        return Err(reject("input too small"));
                    }
                    Err(AmmError::MinReserveBreached) => {
                        return Err(reject("min reserve breached"));
                    }
                    Err(err) => {
                        return Err(fail_with("swap execution", err));
                    }
                };
                let out_large = match swap::get_amount_out(x, y, dx_large, fee) {
                    Ok(value) => value,
                    Err(AmmError::InputTooSmall) => {
                        return Err(reject("input too small"));
                    }
                    Err(AmmError::MinReserveBreached) => {
                        return Err(reject("min reserve breached"));
                    }
                    Err(err) => {
                        return Err(fail_with("swap execution", err));
                    }
                };

                prop_assert!(out_large >= out_small);

                let price_small = pricing::execution_price_x_to_y(x, y, dx_small, fee)
                    .map_err(|err| fail_with("execution price (small)", err))?;
                let price_large = pricing::execution_price_x_to_y(x, y, dx_large, fee)
                    .map_err(|err| fail_with("execution price (large)", err))?;
                prop_assert!(price_large <= price_small);

                Ok(())
            },
        )
        .expect("property run should succeed");
}

#[test]
fn required_input_grows_with_target_output() {
    let strategy = (
        arb_reserve(),
        arb_reserve(),
        (1_500u32..=4_000u32, 4_001u32..=8_500u32),
        arb_fee(),
    );
    let mut runner = runner_for("property::amm::input_monotonic_with_target", 73);

    runner
        .run(&strategy, |(x, y, (frac_small, frac_big), fee)| {
            let available = y.checked_sub(MIN_RESERVE).ok_or_else(|| {
                fail_with(
                    "available reserve computation",
                    AmmError::MinReserveBreached,
                )
            })?;
            prop_assume!(available > 0);

            let dy_small = fraction_of(available, frac_small as u64);
            let dy_large = fraction_of(available, frac_big as u64);
            prop_assume!(dy_small > 0);
            prop_assume!(dy_large > dy_small);

            let dx_small = match swap::get_amount_in(x, y, dy_small, fee) {
                Ok(value) => value,
                Err(AmmError::InputTooSmall) => {
                    return Err(reject("input too small"));
                }
                Err(AmmError::MinReserveBreached) => {
                    return Err(reject("min reserve breached"));
                }
                Err(err) => {
                    return Err(fail_with("swap execution", err));
                }
            };
            let dx_large = match swap::get_amount_in(x, y, dy_large, fee) {
                Ok(value) => value,
                Err(AmmError::InputTooSmall) => {
                    return Err(reject("input too small"));
                }
                Err(AmmError::MinReserveBreached) => {
                    return Err(reject("min reserve breached"));
                }
                Err(err) => {
                    return Err(fail_with("swap execution", err));
                }
            };

            prop_assert!(dx_large >= dx_small);

            Ok(())
        })
        .expect("property run should succeed");
}
