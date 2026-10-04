use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Map, Vec};

// ExchangeRate stores per-submission data.
// `confidence` is encoded as u32 (0–100) because soroban-sdk's testutils
// feature requires contracttype fields to implement SorobanArbitrary, and
// u8 does not.  The valid range is still enforced at runtime (≤ 100).
#[contracttype]
#[derive(Clone)]
pub struct ExchangeRate {
    pub from_currency: Symbol,
    pub to_currency: Symbol,
    pub rate: u128,
    pub timestamp: u64,
    pub source: Symbol,
    pub confidence: u32,
}

// weight is also u32 for the same reason.
#[contracttype]
#[derive(Clone)]
pub struct RateSource {
    pub name: Symbol,
    pub address: Address,
    pub weight: u32,
    pub active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct AggregatedRate {
    pub rate: u128,
    pub weighted_average: u128,
    pub sources_count: u32,
    pub last_updated: u64,
    pub deviation_threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct PathResult {
    pub path: Vec<Symbol>,
    pub expected_amount: i128,
    pub rate: u128,
}

#[contract]
pub struct RateOracleContract;

#[contractimpl]
impl RateOracleContract {
    pub fn submit_rate(
        env: Env,
        source: Address,
        from_currency: Symbol,
        to_currency: Symbol,
        rate: u128,
        confidence: u32,
    ) -> bool {
        source.require_auth();

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let sources = env
            .storage()
            .persistent()
            .get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));

        let rate_source = sources
            .get(source.clone())
            .unwrap_or_else(|| panic!("Rate source not authorized"));

        if !rate_source.active {
            panic!("Rate source is not active");
        }

        if confidence > 100 {
            panic!("Confidence must be between 0 and 100");
        }

        let exchange_rate = ExchangeRate {
            from_currency: from_currency.clone(),
            to_currency: to_currency.clone(),
            rate,
            timestamp: env.ledger().timestamp(),
            source: rate_source.name,
            confidence,
        };

        let rates_key = Symbol::new(&env, "EXCHANGE_RATES");
        let mut rates = env
            .storage()
            .persistent()
            .get::<_, Map<(Symbol, Symbol), Vec<ExchangeRate>>>(&rates_key)
            .unwrap_or_else(|| Map::new(&env));

        let pair_key = (from_currency.clone(), to_currency.clone());
        let mut rate_list = rates
            .get(pair_key.clone())
            .unwrap_or_else(|| Vec::new(&env));

        rate_list.push_back(exchange_rate);
        rates.set(pair_key, rate_list);
        env.storage().persistent().set(&rates_key, &rates);

        Self::update_aggregated_rate(&env, from_currency, to_currency);

        true
    }

    pub fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate {
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env
            .storage()
            .persistent()
            .get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        aggregated_rates
            .get((from_currency, to_currency))
            .unwrap_or_else(|| panic!("Rate not found for this currency pair"))
    }

    pub fn find_best_path(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult {
        let currencies = Self::get_supported_currencies(env.clone());
        let mut dist: Map<Symbol, i128> = Map::new(&env);
        let mut prev: Map<Symbol, Symbol> = Map::new(&env);

        for c in currencies.iter() {
            dist.set(c.clone(), 0i128);
        }
        dist.set(from.clone(), amount);

        // Simplified Bellman-Ford maximising output across at most 3 hops.
        for _ in 0..3 {
            let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
            let aggregated_rates = env
                .storage()
                .persistent()
                .get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
                .unwrap_or_else(|| Map::new(&env));

            for ((u, v), rate_data) in aggregated_rates.iter() {
                let u_dist = dist.get(u.clone()).unwrap_or(0);
                if u_dist > 0 {
                    let new_dist = (u_dist as u128 * rate_data.rate / 1_000_000) as i128;
                    let v_dist = dist.get(v.clone()).unwrap_or(0);
                    if new_dist > v_dist {
                        dist.set(v.clone(), new_dist);
                        prev.set(v.clone(), u.clone());
                    }
                }
            }
        }

        let best_amount = dist.get(to.clone()).unwrap_or(0);
        if best_amount == 0 {
            panic!("No path found");
        }

        // Reconstruct path by walking `prev` back to `from`.
        let mut path: Vec<Symbol> = Vec::new(&env);
        let mut curr = to.clone();
        path.push_front(curr.clone());
        while curr != from {
            curr = prev.get(curr).expect("Inconsistent path");
            path.push_front(curr.clone());
        }

        PathResult {
            path,
            expected_amount: best_amount,
            rate: (best_amount as u128 * 1_000_000 / amount as u128),
        }
    }

    pub fn get_optimal_execution(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult {
        let best_path = Self::find_best_path(env.clone(), from.clone(), to.clone(), amount);

        // Compare against the direct oracle rate.
        let direct_aggregated = Self::get_rate(env.clone(), from.clone(), to.clone());
        let direct_amount = (amount as u128 * direct_aggregated.rate / 1_000_000) as i128;

        if direct_amount > best_path.expected_amount {
            let mut path: Vec<Symbol> = Vec::new(&env);
            path.push_back(from);
            path.push_back(to);
            PathResult {
                path,
                expected_amount: direct_amount,
                rate: direct_aggregated.rate,
            }
        } else {
            best_path
        }
    }

    pub fn get_dex_rate(env: Env, from: Symbol, to: Symbol) -> u128 {
        // Placeholder for native DEX integration.
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env
            .storage()
            .persistent()
            .get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        if let Some(rate) = aggregated_rates.get((from.clone(), to.clone())) {
            rate.rate * 101 / 100
        } else {
            0
        }
    }

    pub fn add_rate_source(env: Env, name: Symbol, address: Address, weight: u32) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env
            .storage()
            .persistent()
            .get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let rate_source = RateSource {
            name: name.clone(),
            address: address.clone(),
            weight,
            active: true,
        };
        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env
            .storage()
            .persistent()
            .get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);

        let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
        let mut currencies = env
            .storage()
            .persistent()
            .get::<_, Vec<Symbol>>(&currencies_key)
            .unwrap_or_else(|| Vec::new(&env));
        if !currencies.contains(&name) {
            currencies.push_back(name);
            env.storage().persistent().set(&currencies_key, &currencies);
        }
        true
    }

    pub fn update_rate_source(env: Env, address: Address, weight: u32, active: bool) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env
            .storage()
            .persistent()
            .get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env
            .storage()
            .persistent()
            .get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));
        let mut rate_source = sources
            .get(address.clone())
            .unwrap_or_else(|| panic!("Rate source not found"));
        rate_source.weight = weight;
        rate_source.active = active;
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);
        true
    }

    pub fn get_rate_sources(env: Env) -> Vec<RateSource> {
        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let sources = env
            .storage()
            .persistent()
            .get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));
        let mut result: Vec<RateSource> = Vec::new(&env);
        for (_, source) in sources.iter() {
            result.push_back(source);
        }
        result
    }

    pub fn set_admin(env: Env, admin: Address) {
        let admin_key = Symbol::new(&env, "ADMIN");
        env.storage().persistent().set(&admin_key, &admin);
    }

    // ─── Private helpers ──────────────────────────────────────────────────

    fn get_supported_currencies(env: Env) -> Vec<Symbol> {
        let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
        env.storage()
            .persistent()
            .get::<_, Vec<Symbol>>(&currencies_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn update_aggregated_rate(env: &Env, from_currency: Symbol, to_currency: Symbol) {
        let rates_key = Symbol::new(env, "EXCHANGE_RATES");
        let rates = env
            .storage()
            .persistent()
            .get::<_, Map<(Symbol, Symbol), Vec<ExchangeRate>>>(&rates_key)
            .unwrap_or_else(|| Map::new(env));

        let pair_key = (from_currency.clone(), to_currency.clone());
        let rate_list = match rates.get(pair_key.clone()) {
            Some(list) => list,
            None => return,
        };
        if rate_list.is_empty() {
            return;
        }

        let mut weighted_sum = 0u128;
        let mut total_weight = 0u128;
        let mut count = 0u32;

        for exchange_rate in rate_list.iter() {
            weighted_sum += exchange_rate.rate * exchange_rate.confidence as u128;
            total_weight += exchange_rate.confidence as u128;
            count += 1;
        }

        if total_weight == 0 {
            return;
        }
        let rate = weighted_sum / total_weight;

        let aggregated_rate = AggregatedRate {
            rate,
            weighted_average: rate,
            sources_count: count,
            last_updated: env.ledger().timestamp(),
            deviation_threshold: 10,
        };

        let aggregated_key = Symbol::new(env, "AGGREGATED_RATES");
        let mut aggregated_rates = env
            .storage()
            .persistent()
            .get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(env));
        aggregated_rates.set(pair_key, aggregated_rate);
        env.storage()
            .persistent()
            .set(&aggregated_key, &aggregated_rates);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::Symbol;

    // ─── Test helpers ─────────────────────────────────────────────────────────

    /// Register the contract, set the admin, and return (env, admin, client).
    fn setup_test() -> (Env, Address, RateOracleContractClient<'static>) {
        let env = Env::default();
        env.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 22,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
            max_entry_ttl: 1_000_000,
        });
        let contract_id = env.register(RateOracleContract, ());
        let client = RateOracleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        client.set_admin(&admin);
        (env, admin, client)
    }

    /// Register a rate source and return its address.
    fn add_source(
        env: &Env,
        client: &RateOracleContractClient,
        _admin: &Address,
        name: &str,
        weight: u32,
    ) -> Address {
        let addr = Address::generate(env);
        // mock_all_auths satisfies the internal admin.require_auth() check
        client
            .mock_all_auths()
            .add_rate_source(&Symbol::new(env, name), &addr, &weight);
        addr
    }

    /// Submit a rate on behalf of `source_addr` using mock auth.
    fn submit(
        client: &RateOracleContractClient,
        source_addr: &Address,
        from: &Symbol,
        to: &Symbol,
        rate: u128,
        confidence: u32,
    ) -> bool {
        client
            .mock_all_auths()
            .submit_rate(source_addr, from, to, &rate, &confidence)
    }

    // ═══════════════════════════════════════════════════════════════════════
    // § 1  Rate Submission
    // ═══════════════════════════════════════════════════════════════════════

    /// submit_rate returns `true` on success.
    #[test]
    fn test_submit_rate_returns_true() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");

        let result = submit(&client, &src, &from, &to, 920_000, 90);
        assert!(result);
    }

    /// After one submission the aggregated rate equals the submitted rate and
    /// `last_updated` reflects the ledger timestamp set in setup.
    #[test]
    fn test_submit_and_get_single_rate() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");
        let rate: u128 = 920_000; // 0.92 with 6-decimal fixed-point

        submit(&client, &src, &from, &to, rate, 90);

        let agg = client.get_rate(&from, &to);
        assert_eq!(agg.rate, rate);
        assert_eq!(agg.sources_count, 1);
        assert_eq!(agg.last_updated, 12345);
    }

    /// Confidence value of exactly 100 is the legal maximum and must succeed.
    #[test]
    fn test_submit_rate_max_confidence_accepted() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");

        let result = submit(&client, &src, &from, &to, 1_000_000, 100);
        assert!(result);
    }

    /// Confidence > 100 must panic.
    #[test]
    #[should_panic(expected = "Confidence must be between 0 and 100")]
    fn test_submit_rate_confidence_above_100_panics() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        submit(
            &client,
            &src,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            1_000_000,
            101, // invalid
        );
    }

    /// Two sequential submissions from the same source accumulate;
    /// `sources_count` grows to 2 after the second call.
    #[test]
    fn test_multiple_submissions_increase_sources_count() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");

        submit(&client, &src, &from, &to, 900_000, 80);
        submit(&client, &src, &from, &to, 950_000, 80);

        let agg = client.get_rate(&from, &to);
        assert_eq!(agg.sources_count, 2);
    }

    /// Submitting from an address that was never registered must panic.
    #[test]
    #[should_panic(expected = "Rate source not authorized")]
    fn test_submit_from_unauthorized_source() {
        let (env, _, client) = setup_test();
        let unregistered = Address::generate(&env);
        client.mock_all_auths().submit_rate(
            &unregistered,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            &920_000,
            &90,
        );
    }

    /// A deactivated source must be rejected even if it was once valid.
    #[test]
    #[should_panic(expected = "Rate source is not active")]
    fn test_submit_from_inactive_source() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        // Deactivate the source
        client.mock_all_auths().update_rate_source(&src, &100, &false);

        submit(
            &client,
            &src,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            920_000,
            90,
        );
    }

    /// Querying a pair that has never received a submission must panic.
    #[test]
    #[should_panic(expected = "Rate not found for this currency pair")]
    fn test_get_rate_for_nonexistent_pair() {
        let (env, _, client) = setup_test();
        client.get_rate(&Symbol::new(&env, "AAA"), &Symbol::new(&env, "BBB"));
    }

    // ═══════════════════════════════════════════════════════════════════════
    // § 2  Weighted Aggregation
    // ═══════════════════════════════════════════════════════════════════════

    /// Two sources with different confidence scores produce a
    /// confidence-weighted average.
    ///
    /// Expected = (18_000_000×80 + 20_000_000×20) / 100 = 18_400_000
    #[test]
    fn test_aggregation_two_sources_confidence_weighted() {
        let (env, admin, client) = setup_test();
        let src1 = add_source(&env, &client, &admin, "SRC1", 50);
        let src2 = add_source(&env, &client, &admin, "SRC2", 50);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "MXN");

        submit(&client, &src1, &from, &to, 18_000_000, 80);
        submit(&client, &src2, &from, &to, 20_000_000, 20);

        let agg = client.get_rate(&from, &to);
        // (18_000_000×80 + 20_000_000×20) / 100 = 18_400_000
        assert_eq!(agg.rate, 18_400_000);
        assert_eq!(agg.sources_count, 2);
    }

    /// Three sources: weighted average must account for all three contributions.
    ///
    /// Rates / confidences: 10_000_000×50, 12_000_000×30, 14_000_000×20
    /// Expected = (500_000_000 + 360_000_000 + 280_000_000) / 100 = 11_400_000
    #[test]
    fn test_aggregation_three_sources() {
        let (env, admin, client) = setup_test();
        let src1 = add_source(&env, &client, &admin, "SRC1", 33);
        let src2 = add_source(&env, &client, &admin, "SRC2", 33);
        let src3 = add_source(&env, &client, &admin, "SRC3", 34);
        let from = Symbol::new(&env, "EUR");
        let to = Symbol::new(&env, "USD");

        submit(&client, &src1, &from, &to, 10_000_000, 50);
        submit(&client, &src2, &from, &to, 12_000_000, 30);
        submit(&client, &src3, &from, &to, 14_000_000, 20);

        let agg = client.get_rate(&from, &to);
        // (10_000_000*50 + 12_000_000*30 + 14_000_000*20) / 100 = 11_400_000
        assert_eq!(agg.rate, 11_400_000);
        assert_eq!(agg.sources_count, 3);
    }

    /// When all sources report the same rate the aggregated rate equals that
    /// value regardless of their individual confidence scores.
    #[test]
    fn test_aggregation_equal_rates_from_all_sources() {
        let (env, admin, client) = setup_test();
        let src1 = add_source(&env, &client, &admin, "SRC1", 33);
        let src2 = add_source(&env, &client, &admin, "SRC2", 33);
        let src3 = add_source(&env, &client, &admin, "SRC3", 34);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");
        let rate: u128 = 920_000;

        submit(&client, &src1, &from, &to, rate, 60);
        submit(&client, &src2, &from, &to, rate, 25);
        submit(&client, &src3, &from, &to, rate, 15);

        let agg = client.get_rate(&from, &to);
        assert_eq!(agg.rate, rate);
    }

    /// A single source with any confidence must set sources_count to 1 and
    /// reflect the exact submitted rate.
    #[test]
    fn test_aggregation_single_source_equals_submitted_rate() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SOLE", 100);
        let from = Symbol::new(&env, "GBP");
        let to = Symbol::new(&env, "USD");
        let rate: u128 = 1_270_000; // 1.27

        submit(&client, &src, &from, &to, rate, 75);

        let agg = client.get_rate(&from, &to);
        assert_eq!(agg.rate, rate);
        assert_eq!(agg.sources_count, 1);
    }

    /// After a second submission `last_updated` must equal the most-recent
    /// ledger timestamp.  We advance the timestamp between the two calls.
    #[test]
    fn test_aggregation_last_updated_reflects_latest_submission() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);
        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");

        // First submission at t = 12345 (set in setup_test)
        submit(&client, &src, &from, &to, 900_000, 90);

        // Advance only the timestamp — keep sequence_number identical to the
        // initial setup so the contract instance entry (created with a short
        // TTL at sequence 10) does not fall into the soroban test host's
        // "archived" state.
        env.ledger().set(LedgerInfo {
            timestamp: 99999,
            protocol_version: 22,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1_000_000,
            min_persistent_entry_ttl: 1_000_000,
            max_entry_ttl: 10_000_000,
        });

        // Second submission at t = 99999
        submit(&client, &src, &from, &to, 950_000, 90);

        let agg = client.get_rate(&from, &to);
        assert_eq!(agg.last_updated, 99999);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // § 3  find_best_path
    // ═══════════════════════════════════════════════════════════════════════

    /// Seed helper: submit a single rate so the aggregated store is populated.
    fn seed_rate(
        env: &Env,
        client: &RateOracleContractClient,
        admin: &Address,
        from: &str,
        to: &str,
        rate: u128,
    ) {
        let src = add_source(env, client, admin, from, 100);
        let f = Symbol::new(env, from);
        let t = Symbol::new(env, to);
        submit(client, &src, &f, &t, rate, 100);
    }

    /// Direct one-hop path A→B: path has exactly [A, B] and expected_amount
    /// equals amount × rate / 1_000_000.
    #[test]
    fn test_find_best_path_direct() {
        let (env, admin, client) = setup_test();
        // 1 USD = 1.25 EUR  (rate = 1_250_000)
        seed_rate(&env, &client, &admin, "USD", "EUR", 1_250_000);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");
        let result = client.find_best_path(&from, &to, &1_000_000i128);

        // expected_amount = 1_000_000 × 1_250_000 / 1_000_000 = 1_250_000
        assert_eq!(result.expected_amount, 1_250_000);
        assert_eq!(result.path.len(), 2);
        assert_eq!(result.path.get(0).unwrap(), from);
        assert_eq!(result.path.get(1).unwrap(), to);
    }

    /// Two-hop path A→B→C: when there is no direct A→C rate the algorithm
    /// must chain through B and produce the compounded output.
    #[test]
    fn test_find_best_path_two_hop() {
        let (env, admin, client) = setup_test();
        // USD → EUR at 1.0  (rate = 1_000_000)
        // EUR → GBP at 0.9  (rate =   900_000)
        // No direct USD → GBP rate
        seed_rate(&env, &client, &admin, "USD", "EUR", 1_000_000);
        seed_rate(&env, &client, &admin, "EUR", "GBP", 900_000);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "GBP");
        let result = client.find_best_path(&from, &to, &1_000_000i128);

        // 1_000_000 → (×1.0) → 1_000_000 EUR → (×0.9) → 900_000 GBP
        assert_eq!(result.expected_amount, 900_000);
        assert_eq!(result.path.len(), 3);
        assert_eq!(result.path.get(0).unwrap(), Symbol::new(&env, "USD"));
        assert_eq!(result.path.get(1).unwrap(), Symbol::new(&env, "EUR"));
        assert_eq!(result.path.get(2).unwrap(), Symbol::new(&env, "GBP"));
    }

    /// Requesting a path for a pair with no rates at all must panic.
    #[test]
    #[should_panic(expected = "No path found")]
    fn test_find_best_path_no_path_panics() {
        let (env, _, client) = setup_test();
        client.find_best_path(
            &Symbol::new(&env, "FOO"),
            &Symbol::new(&env, "BAR"),
            &1_000_000i128,
        );
    }

    /// The reconstructed path starts at `from` and ends at `to`.
    #[test]
    fn test_find_best_path_reconstruction_endpoints() {
        let (env, admin, client) = setup_test();
        seed_rate(&env, &client, &admin, "USD", "MXN", 18_000_000);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "MXN");
        let result = client.find_best_path(&from, &to, &100i128);

        let first = result.path.get(0).unwrap();
        let last = result.path.get((result.path.len() - 1) as u32).unwrap();
        assert_eq!(first, from);
        assert_eq!(last, to);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // § 4  get_optimal_execution
    // ═══════════════════════════════════════════════════════════════════════

    /// When the direct oracle rate produces more than the best multi-hop path,
    /// get_optimal_execution must return a two-element path [from, to].
    #[test]
    fn test_get_optimal_execution_prefers_direct_when_better() {
        let (env, admin, client) = setup_test();
        // Direct USD→MXN at 20.0 (rate = 20_000_000)
        seed_rate(&env, &client, &admin, "USD", "MXN", 20_000_000);
        // Indirect: USD→EUR at 0.9, EUR→MXN at 10.0  → compound ≈ 9.0  (worse)
        seed_rate(&env, &client, &admin, "USD", "EUR", 900_000);
        seed_rate(&env, &client, &admin, "EUR", "MXN", 10_000_000);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "MXN");
        let result = client.get_optimal_execution(&from, &to, &1_000_000i128);

        // Direct: 1_000_000 × 20 = 20_000_000
        // Multi-hop: 1_000_000 × 0.9 × 10 = 9_000_000
        // Direct wins → two-element path
        assert_eq!(result.path.len(), 2);
        assert_eq!(result.path.get(0).unwrap(), from);
        assert_eq!(result.path.get(1).unwrap(), to);
        assert_eq!(result.rate, 20_000_000);
    }

    /// When a multi-hop path produces more than the direct oracle rate,
    /// get_optimal_execution must return the longer path.
    #[test]
    fn test_get_optimal_execution_prefers_multihop_when_better() {
        let (env, admin, client) = setup_test();
        // Direct USD→MXN at 10.0
        seed_rate(&env, &client, &admin, "USD", "MXN", 10_000_000);
        // Indirect: USD→EUR at 1.1, EUR→MXN at 20.0  → compound ≈ 22.0 > 10.0
        seed_rate(&env, &client, &admin, "USD", "EUR", 1_100_000);
        seed_rate(&env, &client, &admin, "EUR", "MXN", 20_000_000);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "MXN");
        let result = client.get_optimal_execution(&from, &to, &1_000_000i128);

        // Multi-hop wins → path length > 2 and higher output
        assert!(result.path.len() > 2);
        assert!(result.expected_amount > 10_000_000);
    }

    // ═══════════════════════════════════════════════════════════════════════
    // § 5  Admin functions
    // ═══════════════════════════════════════════════════════════════════════

    /// add_rate_source called by a non-admin must panic.
    #[test]
    #[should_panic]
    fn test_add_rate_source_unauthorized() {
        let (env, _, client) = setup_test();
        let new_source = Address::generate(&env);
        // No auths mocked → admin.require_auth() inside the contract will panic
        client
            .mock_auths(&[])
            .add_rate_source(&Symbol::new(&env, "FAKE"), &new_source, &100);
    }

    /// update_rate_source called by a non-admin must panic.
    #[test]
    #[should_panic]
    fn test_update_rate_source_unauthorized() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);

        // No auths mocked → admin.require_auth() inside the contract will panic
        client
            .mock_auths(&[])
            .update_rate_source(&src, &50, &true);
    }

    /// add_rate_source returns true and the source is retrievable afterward.
    #[test]
    fn test_add_rate_source_success() {
        let (env, _admin, client) = setup_test();
        let new_source = Address::generate(&env);
        let result = client
            .mock_all_auths()
            .add_rate_source(&Symbol::new(&env, "SRC1"), &new_source, &75);
        assert!(result);

        let sources = client.get_rate_sources();
        assert_eq!(sources.len(), 1);
    }

    /// update_rate_source returns true and the source is recorded as inactive.
    #[test]
    fn test_update_rate_source_deactivates_success() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);

        let result = client.mock_all_auths().update_rate_source(&src, &100, &false);
        assert!(result);
    }

    /// Submitting from a source that has been deactivated must panic.
    #[test]
    #[should_panic(expected = "Rate source is not active")]
    fn test_update_rate_source_deactivates_source() {
        let (env, admin, client) = setup_test();
        let src = add_source(&env, &client, &admin, "SRC1", 100);

        client.mock_all_auths().update_rate_source(&src, &100, &false);

        // Submission from the now-inactive source must panic.
        submit(
            &client,
            &src,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            920_000,
            90,
        );
    }
}
