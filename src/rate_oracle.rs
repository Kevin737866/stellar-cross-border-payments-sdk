use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Map, Vec};

#[contracttype]
pub struct ExchangeRate {
    pub from_currency: Symbol,
    pub to_currency: Symbol,
    pub rate: u128,
    pub timestamp: u64,
    pub source: Symbol,
    pub confidence: u8,
}

#[contracttype]
pub struct RateSource {
    pub name: Symbol,
    pub address: Address,
    pub weight: u8,
    pub active: bool,
}

#[contracttype]
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

pub struct RateOracleContract;

#[contract]
pub trait RateOracleTrait {
    // ... (keep existing mixed with new)
    fn submit_rate(
        env: Env,
        source: Address,
        from_currency: Symbol,
        to_currency: Symbol,
        rate: u128,
        confidence: u8,
    ) -> bool;

    fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate;

    fn find_best_path(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult;

    fn get_optimal_execution(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult;

    fn get_dex_rate(env: Env, from: Symbol, to: Symbol) -> u128;
    
    fn add_rate_source(env: Env, name: Symbol, address: Address, weight: u8) -> bool;
    fn update_rate_source(env: Env, address: Address, weight: u8, active: bool) -> bool;
    fn get_rate_sources(env: Env) -> Vec<RateSource>;
    fn set_admin(env: Env, admin: Address);
}

#[contractimpl]
impl RateOracleTrait for RateOracleContract {
    fn submit_rate(
        env: Env,
        source: Address,
        from_currency: Symbol,
        to_currency: Symbol,
        rate: u128,
        confidence: u8,
    ) -> bool {
        source.require_auth();

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));

        let rate_source = sources.get(source.clone())
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
        let mut rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), Vec<ExchangeRate>>>(&rates_key)
            .unwrap_or_else(|| Map::new(&env));

        let pair_key = (from_currency.clone(), to_currency.clone());
        let mut rate_list = rates.get(pair_key.clone())
            .unwrap_or_else(|| Vec::new(&env));

        rate_list.push_back(exchange_rate);
        rates.set(pair_key, rate_list);
        env.storage().persistent().set(&rates_key, &rates);

        Self::update_aggregated_rate(&env, from_currency, to_currency);

        true
    }

    fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate {
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        aggregated_rates.get((from_currency, to_currency))
            .unwrap_or_else(|| panic!("Rate not found for this currency pair"))
    }

    fn find_best_path(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult {
        let currencies = Self::get_supported_currencies(env.clone());
        let mut dist = Map::new(&env);
        let mut prev = Map::new(&env);

        for c in currencies.iter() {
            dist.set(c.clone(), 0i128); // Focus on maximizing output
        }
        dist.set(from.clone(), amount);

        // Bellman-Ford simplified for finding MAX output in fixed hops
        for _ in 0..3 { // MAX 3 hops for gas
            let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
            let aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
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

        // Reconstruct path
        let mut path = Vec::new(&env);
        let mut curr = to.clone();
        path.push_front(curr.clone());
        while curr != from {
            curr = prev.get(curr).expect("Inconsistent path");
            path.push_front(curr.clone());
        }

        PathResult {
            path,
            expected_amount: best_amount,
            rate: (best_amount as u128 * 1_000_000 / amount as u128) as u128,
        }
    }

    fn get_optimal_execution(env: Env, from: Symbol, to: Symbol, amount: i128) -> PathResult {
        let best_path = Self::find_best_path(env.clone(), from.clone(), to.clone(), amount);
        
        // Simple comparison with direct oracle rate
        let direct_aggregated = Self::get_rate(env.clone(), from.clone(), to.clone());
        let direct_amount = (amount as u128 * direct_aggregated.rate / 1_000_000) as i128;

        if direct_amount > best_path.expected_amount {
            let mut path = Vec::new(&env);
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

    fn get_dex_rate(env: Env, from: Symbol, to: Symbol) -> u128 {
        // Implementation using soroban-env host functions OR dedicated DEX contract call
        // Mocking for now as placeholder for native DEX integration
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        if let Some(rate) = aggregated_rates.get((from.clone(), to.clone())) {
            // Apply a "dex multiplier" to simulate slightly better rates
            rate.rate * 101 / 100 
        } else {
            0
        }
    }

    fn add_rate_source(env: Env, name: Symbol, address: Address, weight: u8) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key).unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let rate_source = RateSource { name: name.clone(), address: address.clone(), weight, active: true };
        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key).unwrap_or_else(|| Map::new(&env));
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);

        let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
        let mut currencies = env.storage().persistent().get::<_, Vec<Symbol>>(&currencies_key).unwrap_or_else(|| Vec::new(&env));
        if !currencies.contains(&name) {
            currencies.push_back(name);
            env.storage().persistent().set(&currencies_key, &currencies);
        }
        true
    }

    fn update_rate_source(env: Env, address: Address, weight: u8, active: bool) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key).unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key).unwrap_or_else(|| Map::new(&env));
        let mut rate_source = sources.get(address.clone()).unwrap_or_else(|| panic!("Rate source not found"));
        rate_source.weight = weight;
        rate_source.active = active;
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);
        true
    }

    fn get_rate_sources(env: Env) -> Vec<RateSource> {
        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key).unwrap_or_else(|| Map::new(&env));
        let mut result = Vec::new(&env);
        for (_, source) in sources.iter() { result.push_back(source); }
        result
    }

    fn set_admin(env: Env, admin: Address) {
        let admin_key = Symbol::new(&env, "ADMIN");
        env.storage().persistent().set(&admin_key, &admin);
    }
}

// ... helper get_supported_currencies and internal update_aggregated_rate ...
fn get_supported_currencies(env: Env) -> Vec<Symbol> {
    let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
    env.storage().persistent().get::<_, Vec<Symbol>>(&currencies_key).unwrap_or_else(|| Vec::new(&env))
}

impl RateOracleContract {
    fn update_aggregated_rate(env: &Env, from_currency: Symbol, to_currency: Symbol) {
        let rates_key = Symbol::new(env, "EXCHANGE_RATES");
        let rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), Vec<ExchangeRate>>>(&rates_key).unwrap_or_else(|| Map::new(env));
        let sources_key = Symbol::new(env, "RATE_SOURCES");
        let sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key).unwrap_or_else(|| Map::new(env));

        let pair_key = (from_currency.clone(), to_currency.clone());
        let rate_list = match rates.get(pair_key.clone()) { Some(list) => list, None => return };
        if rate_list.is_empty() { return }

        let mut weighted_sum = 0u128;
        let mut total_weight = 0u32;
        let mut filtered_rates = Vec::new(env);

        for exchange_rate in rate_list.iter() {
            // simplified for implementation
            weighted_sum += exchange_rate.rate as u128 * exchange_rate.confidence as u128;
            total_weight += exchange_rate.confidence as u32;
            filtered_rates.push_back(exchange_rate.rate);
        }

        if total_weight == 0 { return }
        let rate = weighted_sum / total_weight as u128;

        let aggregated_rate = AggregatedRate {
            rate,
            weighted_average: rate,
            sources_count: filtered_rates.len() as u32,
            last_updated: env.ledger().timestamp(),
            deviation_threshold: 10,
        };

        let aggregated_key = Symbol::new(env, "AGGREGATED_RATES");
        let mut aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key).unwrap_or_else(|| Map::new(env));
        aggregated_rates.set(pair_key, aggregated_rate);
        env.storage().persistent().set(&aggregated_key, &aggregated_rates);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
    use soroban_sdk::{vec, IntoVal, Symbol};

    fn setup_test() -> (Env, Address, RateOracleContractClient<'static>) {
        let env = Env::default();
        env.ledger().set(LedgerInfo {
            timestamp: 12345,
            protocol_version: 20,
            sequence_number: 10,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 1,
            min_persistent_entry_ttl: 1,
        });

        let contract_id = env.register_contract(None, RateOracleContract);
        let client = RateOracleContractClient::new(&env, &contract_id);
        let admin = Address::random(&env);
        client.set_admin(&admin);
        (env, admin, client)
    }

    #[test]
    fn test_submit_and_get_single_rate() {
        let (env, admin, client) = setup_test();
        let source_address = Address::random(&env);
        let source_name = Symbol::new(&env, "SRC1");

        client.add_rate_source(&admin, &source_name, &source_address, &100);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "EUR");
        let rate = 920000; // 0.92
        let confidence = 90;

        client.with_source_account(&source_address).submit_rate(
            &source_address,
            &from,
            &to,
            &rate,
            &confidence,
        );

        let aggregated_rate = client.get_rate(&from, &to);

        assert_eq!(aggregated_rate.rate, rate);
        assert_eq!(aggregated_rate.sources_count, 1);
        assert_eq!(aggregated_rate.last_updated, 12345);
    }

    #[test]
    fn test_aggregation_with_confidence_weighting() {
        let (env, admin, client) = setup_test();
        let source1_addr = Address::random(&env);
        let source2_addr = Address::random(&env);

        client.add_rate_source(&admin, &Symbol::new(&env, "SRC1"), &source1_addr, &50);
        client.add_rate_source(&admin, &Symbol::new(&env, "SRC2"), &source2_addr, &50);

        let from = Symbol::new(&env, "USD");
        let to = Symbol::new(&env, "MXN");

        // Source 1 submits rate 18.0 with confidence 80
        client.with_source_account(&source1_addr).submit_rate(
            &source1_addr,
            &from,
            &to,
            &18_000_000,
            &80,
        );

        // Source 2 submits rate 20.0 with confidence 20
        client.with_source_account(&source2_addr).submit_rate(
            &source2_addr,
            &from,
            &to,
            &20_000_000,
            &20,
        );

        let aggregated_rate = client.get_rate(&from, &to);

        // Expected rate = (18_000_000 * 80 + 20_000_000 * 20) / (80 + 20)
        // = (1_440_000_000 + 400_000_000) / 100
        // = 1_840_000_000 / 100 = 18_400_000
        let expected_rate = 18_400_000;

        assert_eq!(aggregated_rate.rate, expected_rate);
        assert_eq!(aggregated_rate.sources_count, 2);
    }

    #[test]
    #[should_panic(expected = "Rate not found for this currency pair")]
    fn test_get_rate_for_nonexistent_pair() {
        let (env, _, client) = setup_test();
        let from = Symbol::new(&env, "AAA");
        let to = Symbol::new(&env, "BBB");
        client.get_rate(&from, &to);
    }

    #[test]
    #[should_panic(expected = "Rate source not authorized")]
    fn test_submit_from_unauthorized_source() {
        let (env, _, client) = setup_test();
        let unauthorized_source = Address::random(&env);
        client.with_source_account(&unauthorized_source).submit_rate(
            &unauthorized_source,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            &920000,
            &90,
        );
    }

    #[test]
    #[should_panic(expected = "Rate source is not active")]
    fn test_submit_from_inactive_source() {
        let (env, admin, client) = setup_test();
        let source_addr = Address::random(&env);
        client.add_rate_source(&admin, &Symbol::new(&env, "SRC1"), &source_addr, &100);
        client.update_rate_source(&admin, &source_addr, &100, &false); // Deactivate

        client.with_source_account(&source_addr).submit_rate(
            &source_addr,
            &Symbol::new(&env, "USD"),
            &Symbol::new(&env, "EUR"),
            &920000,
            &90,
        );
    }
}
