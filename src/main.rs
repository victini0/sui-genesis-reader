// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Sui mainnet genesis.blob analyzer.
//!
//! Parses the genesis file using Sui's own deserialization code and outputs
//! a comprehensive analysis of the initial state: token distribution, validator
//! set, concentration metrics, and on-chain locking status.
//!
//! Usage:
//!   cargo run --release -p sui-genesis-analyzer -- /path/to/genesis.blob

use std::collections::BTreeMap;

use move_core_types::language_storage::StructTag;
use sui_config::genesis::Genesis;
use sui_types::gas_coin::GasCoin;
use sui_types::governance::StakedSui;
use sui_types::sui_system_state::SuiSystemStateTrait as _;

const MIST_PER_SUI: u64 = 1_000_000_000;

fn main() {
    let genesis_path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: sui-genesis-analyzer <path-to-genesis.blob>");
        eprintln!();
        eprintln!("Download the mainnet genesis blob:");
        eprintln!("  git clone https://github.com/MystenLabs/sui-genesis");
        eprintln!("  Then: cargo run --release -- sui-genesis/mainnet/genesis.blob");
        std::process::exit(1);
    });

    println!("Loading genesis from: {}", genesis_path);
    let genesis = Genesis::load(&genesis_path).expect("Failed to load genesis.blob");

    println!("\n{}", "=".repeat(80));
    println!("  SUI MAINNET GENESIS - COMPREHENSIVE ANALYSIS");
    println!("{}", "=".repeat(80));

    // ── Section 1: Genesis Metadata ──
    println!("\n{}", "─".repeat(80));
    println!("  1. GENESIS METADATA");
    println!("{}", "─".repeat(80));

    let checkpoint = genesis.checkpoint();
    println!("  Genesis Hash:              {:?}", genesis.hash());
    println!("  Epoch:                     {}", checkpoint.epoch);
    println!("  Sequence Number:           {}", checkpoint.sequence_number);
    println!("  Timestamp (ms):            {}", checkpoint.timestamp_ms);

    let clock = genesis.clock();
    println!("  Clock Timestamp (ms):      {}", clock.timestamp_ms);

    let total_objects = genesis.objects().len();
    println!("  Total Genesis Objects:     {}", total_objects);

    let ref_gas_price = genesis.reference_gas_price();
    println!("  Reference Gas Price:       {} MIST", ref_gas_price);

    // ── Section 2: System State ──
    println!("\n{}", "─".repeat(80));
    println!("  2. SYSTEM STATE");
    println!("{}", "─".repeat(80));

    let system_state = genesis.sui_system_object();
    let inner = system_state.into_genesis_version_for_tooling();

    println!("  Protocol Version:          {}", inner.protocol_version);
    println!("  System State Version:      {}", inner.system_state_version);
    println!("  Epoch:                     {}", inner.epoch);
    println!("  Reference Gas Price:       {} MIST", inner.reference_gas_price);
    println!("  Safe Mode:                 {}", inner.safe_mode);
    println!("  Epoch Start Timestamp:     {} ms", inner.epoch_start_timestamp_ms);

    // ── Section 3: System Parameters ──
    println!("\n{}", "─".repeat(80));
    println!("  3. SYSTEM PARAMETERS");
    println!("{}", "─".repeat(80));

    let params = &inner.parameters;
    println!(
        "  Epoch Duration:            {} ms ({:.1} hours)",
        params.epoch_duration_ms,
        params.epoch_duration_ms as f64 / 3_600_000.0
    );
    println!(
        "  Stake Subsidy Start Epoch: {}",
        params.stake_subsidy_start_epoch
    );
    println!(
        "  Max Validator Count:       {}",
        params.max_validator_count
    );
    println!(
        "  Min Validator Joining Stake:{} SUI ({} MIST)",
        params.min_validator_joining_stake / MIST_PER_SUI,
        params.min_validator_joining_stake
    );
    println!(
        "  Low Stake Threshold:       {} SUI",
        params.validator_low_stake_threshold / MIST_PER_SUI
    );
    println!(
        "  Very Low Stake Threshold:  {} SUI",
        params.validator_very_low_stake_threshold / MIST_PER_SUI
    );
    println!(
        "  Low Stake Grace Period:    {} epochs",
        params.validator_low_stake_grace_period
    );

    // ── Section 4: Stake Subsidy ──
    println!("\n{}", "─".repeat(80));
    println!("  4. STAKE SUBSIDY CONFIGURATION");
    println!("{}", "─".repeat(80));

    let subsidy = &inner.stake_subsidy;
    let subsidy_balance = subsidy.balance.value();
    println!(
        "  Subsidy Fund Balance:      {} SUI ({:.2}% of total supply)",
        subsidy_balance / MIST_PER_SUI,
        (subsidy_balance as f64 / (10_000_000_000u64 * MIST_PER_SUI) as f64) * 100.0
    );
    println!(
        "  Distribution Counter:      {}",
        subsidy.distribution_counter
    );
    println!(
        "  Current Distribution:      {} SUI per epoch",
        subsidy.current_distribution_amount / MIST_PER_SUI
    );
    println!(
        "  Period Length:              {} epochs",
        subsidy.stake_subsidy_period_length
    );
    println!(
        "  Decrease Rate:             {} basis points ({:.1}%)",
        subsidy.stake_subsidy_decrease_rate,
        subsidy.stake_subsidy_decrease_rate as f64 / 100.0
    );

    // Calculate subsidy projection
    println!("\n  --- Subsidy Projection ---");
    let mut dist_amount = subsidy.current_distribution_amount;
    let mut total_distributed: u64 = 0;
    let mut period = 0;
    while total_distributed < subsidy_balance && period < 50 {
        let period_total = dist_amount * subsidy.stake_subsidy_period_length;
        let remaining = subsidy_balance - total_distributed;
        let actual = if period_total > remaining {
            remaining
        } else {
            period_total
        };
        println!(
            "  Period {:2}: {:>12} SUI/epoch x {:3} epochs = {:>15} SUI (cumulative: {:>15} SUI)",
            period,
            dist_amount / MIST_PER_SUI,
            subsidy.stake_subsidy_period_length,
            actual / MIST_PER_SUI,
            (total_distributed + actual) / MIST_PER_SUI
        );
        total_distributed += actual;
        dist_amount -= dist_amount * subsidy.stake_subsidy_decrease_rate as u64 / 10000;
        period += 1;
    }
    println!(
        "  Total Subsidy to Distribute: {} SUI over ~{} periods",
        subsidy_balance / MIST_PER_SUI,
        period
    );

    // ── Section 5: Storage Fund ──
    println!("\n{}", "─".repeat(80));
    println!("  5. STORAGE FUND");
    println!("{}", "─".repeat(80));

    let storage_fund = &inner.storage_fund;
    println!(
        "  Total Object Storage Rebates: {} SUI",
        storage_fund.total_object_storage_rebates.value() / MIST_PER_SUI
    );
    println!(
        "  Non-Refundable Balance:       {} SUI",
        storage_fund.non_refundable_balance.value() / MIST_PER_SUI
    );

    // ── Section 6: Validators ──
    println!("\n{}", "─".repeat(80));
    println!(
        "  6. VALIDATORS ({} total)",
        inner.validators.active_validators.len()
    );
    println!("{}", "─".repeat(80));

    let validator_set = &inner.validators;
    println!(
        "  Total Stake:               {} SUI",
        validator_set.total_stake / MIST_PER_SUI
    );

    println!(
        "\n  {:<4} {:<35} {:>15} {:>10} {:>8} {:>10}",
        "#", "Name", "Stake (SUI)", "Gas Price", "Comm%", "Vote Power"
    );
    println!("  {}", "-".repeat(86));

    let mut validators: Vec<_> = validator_set.active_validators.iter().collect();
    validators.sort_by(|a, b| b.staking_pool.sui_balance.cmp(&a.staking_pool.sui_balance));

    let mut total_validator_stake: u64 = 0;
    for (i, v) in validators.iter().enumerate() {
        let meta = v.verified_metadata();
        let stake = v.staking_pool.sui_balance / MIST_PER_SUI;
        total_validator_stake += v.staking_pool.sui_balance;
        println!(
            "  {:<4} {:<35} {:>15} {:>10} {:>7}% {:>10}",
            i + 1,
            if meta.name.len() > 34 {
                &meta.name[..34]
            } else {
                &meta.name
            },
            stake,
            v.gas_price,
            v.commission_rate / 100,
            v.voting_power
        );
    }

    // Validator statistics
    println!("\n  --- Validator Statistics ---");
    let stakes: Vec<u64> = validators
        .iter()
        .map(|v| v.staking_pool.sui_balance / MIST_PER_SUI)
        .collect();
    let max_stake = stakes.iter().max().unwrap_or(&0);
    let min_stake = stakes.iter().min().unwrap_or(&0);
    let avg_stake = total_validator_stake / MIST_PER_SUI / validators.len() as u64;
    let median_stake = stakes[stakes.len() / 2];

    println!("  Max Stake:       {} SUI", max_stake);
    println!("  Min Stake:       {} SUI", min_stake);
    println!("  Average Stake:   {} SUI", avg_stake);
    println!("  Median Stake:    {} SUI", median_stake);

    // Stake distribution tiers
    let tier_20m = stakes.iter().filter(|&&s| s == 20_000_000).count();
    let tier_25m = stakes.iter().filter(|&&s| s == 25_000_000).count();
    let tier_80m = stakes.iter().filter(|&&s| s == 80_000_000).count();
    let tier_150m = stakes.iter().filter(|&&s| s == 150_000_000).count();
    let tier_other = validators.len() - tier_20m - tier_25m - tier_80m - tier_150m;

    println!("\n  --- Stake Tier Distribution ---");
    println!(
        "  20M SUI:   {} validators ({:.1}%)",
        tier_20m,
        tier_20m as f64 / validators.len() as f64 * 100.0
    );
    println!(
        "  25M SUI:   {} validators ({:.1}%)",
        tier_25m,
        tier_25m as f64 / validators.len() as f64 * 100.0
    );
    println!(
        "  80M SUI:   {} validators ({:.1}%)",
        tier_80m,
        tier_80m as f64 / validators.len() as f64 * 100.0
    );
    println!(
        "  150M SUI:  {} validators ({:.1}%)",
        tier_150m,
        tier_150m as f64 / validators.len() as f64 * 100.0
    );
    println!(
        "  Other:     {} validators ({:.1}%)",
        tier_other,
        tier_other as f64 / validators.len() as f64 * 100.0
    );

    // Gas price distribution
    let mut gas_prices: BTreeMap<u64, usize> = BTreeMap::new();
    for v in &validators {
        *gas_prices.entry(v.gas_price).or_insert(0) += 1;
    }
    println!("\n  --- Gas Price Distribution ---");
    for (price, count) in &gas_prices {
        println!("  {} MIST: {} validators", price, count);
    }

    // Commission rate distribution
    let mut comm_rates: BTreeMap<u64, usize> = BTreeMap::new();
    for v in &validators {
        *comm_rates.entry(v.commission_rate / 100).or_insert(0) += 1;
    }
    println!("\n  --- Commission Rate Distribution ---");
    for (rate, count) in &comm_rates {
        println!("  {}%: {} validators", rate, count);
    }

    // Validator network info
    println!("\n  --- Validator Network Details ---");
    for v in &validators {
        let meta = v.verified_metadata();
        println!(
            "  {} | addr: {} | net: {} | p2p: {}",
            if meta.name.len() > 25 {
                &meta.name[..25]
            } else {
                &meta.name
            },
            meta.sui_address,
            meta.net_address,
            meta.p2p_address
        );
    }

    // ── Section 7: Token Distribution ──
    println!("\n{}", "─".repeat(80));
    println!("  7. TOKEN DISTRIBUTION (All Genesis Addresses)");
    println!("{}", "─".repeat(80));

    let mut address_balances: BTreeMap<String, (u64, u64)> = BTreeMap::new(); // (liquid, staked)

    for obj in genesis.objects() {
        let obj_type = obj.type_();
        if let Some(t) = obj_type {
            let struct_tag: StructTag = t.clone().into();
            if GasCoin::is_gas_coin(&struct_tag) {
                if let Ok(gas_coin) = GasCoin::try_from(obj) {
                    let owner = format!("{}", obj.owner);
                    let entry = address_balances.entry(owner).or_insert((0, 0));
                    entry.0 += gas_coin.value();
                }
            } else if StakedSui::is_staked_sui(&struct_tag) {
                if let Ok(staked) = StakedSui::try_from(obj) {
                    let owner = format!("{}", obj.owner);
                    let entry = address_balances.entry(owner).or_insert((0, 0));
                    entry.1 += staked.principal();
                }
            }
        }
    }

    // Sort by total balance (liquid + staked) descending
    let mut sorted_balances: Vec<_> = address_balances.iter().collect();
    sorted_balances.sort_by(|a, b| (b.1 .0 + b.1 .1).cmp(&(a.1 .0 + a.1 .1)));

    let total_supply_mist = 10_000_000_000u64 * MIST_PER_SUI;

    println!(
        "\n  {:<4} {:<68} {:>15} {:>15} {:>15} {:>8}",
        "#", "Owner", "Liquid (SUI)", "Staked (SUI)", "Total (SUI)", "% Supply"
    );
    println!("  {}", "-".repeat(126));

    let mut running_total: u64 = 0;
    let mut total_liquid: u64 = 0;
    let mut total_staked_tokens: u64 = 0;
    let mut accounts_over_100m = 0;
    let mut accounts_over_1b = 0;

    for (i, (owner, (liquid, staked))) in sorted_balances.iter().enumerate() {
        let total = liquid + staked;
        running_total += total;
        total_liquid += liquid;
        total_staked_tokens += staked;
        let pct = (total as f64 / total_supply_mist as f64) * 100.0;

        if total / MIST_PER_SUI >= 100_000_000 {
            accounts_over_100m += 1;
        }
        if total / MIST_PER_SUI >= 1_000_000_000 {
            accounts_over_1b += 1;
        }

        // Show top 30 and any with >1% supply
        if i < 30 || pct > 1.0 {
            println!(
                "  {:<4} {:<68} {:>15} {:>15} {:>15} {:>7.2}%",
                i + 1,
                owner,
                liquid / MIST_PER_SUI,
                staked / MIST_PER_SUI,
                total / MIST_PER_SUI,
                pct
            );
        }
    }

    // Token distribution summary
    println!("\n  --- Token Distribution Summary ---");
    println!(
        "  Total Addresses:           {}",
        address_balances.len()
    );
    println!(
        "  Total Liquid:              {} SUI ({:.2}%)",
        total_liquid / MIST_PER_SUI,
        (total_liquid as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Total Staked:              {} SUI ({:.2}%)",
        total_staked_tokens / MIST_PER_SUI,
        (total_staked_tokens as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Stake Subsidy Fund:        {} SUI ({:.2}%)",
        subsidy_balance / MIST_PER_SUI,
        (subsidy_balance as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Accounts > 100M SUI:       {}",
        accounts_over_100m
    );
    println!("  Accounts > 1B SUI:         {}", accounts_over_1b);
    println!(
        "  Running Total:             {} SUI",
        running_total / MIST_PER_SUI
    );

    // ── Section 8: Concentration Analysis ──
    println!("\n{}", "─".repeat(80));
    println!("  8. CONCENTRATION ANALYSIS");
    println!("{}", "─".repeat(80));

    let top1_total = sorted_balances
        .first()
        .map(|(_, (l, s))| l + s)
        .unwrap_or(0);
    let top2_total: u64 = sorted_balances
        .iter()
        .take(2)
        .map(|(_, (l, s))| l + s)
        .sum();
    let top5_total: u64 = sorted_balances
        .iter()
        .take(5)
        .map(|(_, (l, s))| l + s)
        .sum();
    let top10_total: u64 = sorted_balances
        .iter()
        .take(10)
        .map(|(_, (l, s))| l + s)
        .sum();
    let top20_total: u64 = sorted_balances
        .iter()
        .take(20)
        .map(|(_, (l, s))| l + s)
        .sum();

    println!(
        "  Top  1 address:  {:>15} SUI ({:.2}%)",
        top1_total / MIST_PER_SUI,
        (top1_total as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Top  2 addresses:{:>15} SUI ({:.2}%)",
        top2_total / MIST_PER_SUI,
        (top2_total as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Top  5 addresses:{:>15} SUI ({:.2}%)",
        top5_total / MIST_PER_SUI,
        (top5_total as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Top 10 addresses:{:>15} SUI ({:.2}%)",
        top10_total / MIST_PER_SUI,
        (top10_total as f64 / total_supply_mist as f64) * 100.0
    );
    println!(
        "  Top 20 addresses:{:>15} SUI ({:.2}%)",
        top20_total / MIST_PER_SUI,
        (top20_total as f64 / total_supply_mist as f64) * 100.0
    );

    // Gini coefficient calculation
    let mut sorted_totals: Vec<f64> = sorted_balances
        .iter()
        .map(|(_, (l, s))| (l + s) as f64)
        .collect();
    sorted_totals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = sorted_totals.len() as f64;
    let mean = sorted_totals.iter().sum::<f64>() / n;
    let mut gini_sum = 0.0;
    for (i, val) in sorted_totals.iter().enumerate() {
        gini_sum += (2.0 * (i as f64 + 1.0) - n - 1.0) * val;
    }
    let gini = gini_sum / (n * n * mean);
    println!(
        "\n  Gini Coefficient:          {:.4} (0=equal, 1=concentrated)",
        gini
    );

    // ── Section 9: Genesis Objects Summary ──
    println!("\n{}", "─".repeat(80));
    println!("  9. GENESIS OBJECTS SUMMARY");
    println!("{}", "─".repeat(80));

    let mut object_types: BTreeMap<String, usize> = BTreeMap::new();
    for obj in genesis.objects() {
        let type_name = match obj.type_() {
            Some(t) => format!("{}", t),
            None => "Package".to_string(),
        };
        *object_types.entry(type_name).or_insert(0) += 1;
    }

    println!("  {:<60} {:>8}", "Object Type", "Count");
    println!("  {}", "-".repeat(70));
    let mut sorted_types: Vec<_> = object_types.iter().collect();
    sorted_types.sort_by(|a, b| b.1.cmp(a.1));
    for (type_name, count) in &sorted_types {
        let display_name = if type_name.len() > 59 {
            &type_name[..59]
        } else {
            type_name.as_str()
        };
        println!("  {:<60} {:>8}", display_name, count);
    }

    // ── Section 10: Committee ──
    println!("\n{}", "─".repeat(80));
    println!("  10. COMMITTEE INFO");
    println!("{}", "─".repeat(80));

    let committee = genesis.committee();
    println!("  Epoch:                     {}", committee.epoch());
    println!(
        "  Committee Size:            {}",
        committee.num_members()
    );
    println!(
        "  Total Voting Power:        {}",
        committee.total_votes()
    );

    // ── Section 11: Locking Analysis ──
    println!("\n{}", "─".repeat(80));
    println!("  11. ON-CHAIN LOCKING ANALYSIS");
    println!("{}", "─".repeat(80));

    let lock_keywords = [
        "time_lock", "timelock", "TimeLock", "Timelock",
        "vesting", "Vesting", "vest_", "Vest_",
        "locked_coin", "LockedCoin", "epoch_lock", "EpochLock",
    ];
    let mut has_lock_or_vest = false;
    for obj in genesis.objects() {
        if let Some(t) = obj.type_() {
            let type_str = format!("{}", t);
            if lock_keywords.iter().any(|kw| type_str.contains(kw)) {
                has_lock_or_vest = true;
                println!("  FOUND LOCK/VESTING OBJECT: {}", type_str);
            }
        }
    }
    if !has_lock_or_vest {
        println!("  NO on-chain time-lock or vesting contracts found in genesis.");
        println!("  All tokens are regular GasCoin/StakedSui objects.");
        println!("  Any claimed lockups are off-chain promises only.");
    }

    // ── Final Summary ──
    println!("\n{}", "=".repeat(80));
    println!("  FINAL SUMMARY");
    println!("{}", "=".repeat(80));
    println!("  Total Supply:      10,000,000,000 SUI");
    println!(
        "  Validators:        {} (stake range: {} - {} SUI)",
        validators.len(),
        min_stake,
        max_stake
    );
    println!("  Genesis Addresses: {}", address_balances.len());
    println!(
        "  Concentration:     Top 2 = {:.2}%, Gini = {:.4}",
        (top2_total as f64 / total_supply_mist as f64) * 100.0,
        gini
    );
    println!(
        "  On-chain Locking:  {}",
        if has_lock_or_vest { "YES" } else { "NONE" }
    );
    println!("{}", "=".repeat(80));
}
