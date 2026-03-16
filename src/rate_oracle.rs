use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Map, Vec, U256};

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

pub struct RateOracleContract;

#[contract]
pub trait RateOracleTrait {
    fn submit_rate(
        env: Env,
        source: Address,
        from_currency: Symbol,
        to_currency: Symbol,
        rate: u128,
        confidence: u8,
    ) -> bool;

    fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate;

    fn get_all_rates(env: Env, from_currency: Symbol) -> Map<Symbol, AggregatedRate>;

    fn add_rate_source(
        env: Env,
        name: Symbol,
        address: Address,
        weight: u8,
    ) -> bool;

    fn update_rate_source(
        env: Env,
        address: Address,
        weight: u8,
        active: bool,
    ) -> bool;

    fn get_rate_sources(env: Env) -> Vec<RateSource>;

    fn set_admin(env: Env, admin: Address);

    fn set_deviation_threshold(env: Env, threshold: u32);

    fn get_supported_currencies(env: Env) -> Vec<Symbol>;
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

        Self::update_aggregated_rate(env, from_currency, to_currency);

        true
    }

    fn get_rate(env: Env, from_currency: Symbol, to_currency: Symbol) -> AggregatedRate {
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        aggregated_rates.get((from_currency, to_currency))
            .unwrap_or_else(|| panic!("Rate not found for this currency pair"))
    }

    fn get_all_rates(env: Env, from_currency: Symbol) -> Map<Symbol, AggregatedRate> {
        let aggregated_key = Symbol::new(&env, "AGGREGATED_RATES");
        let aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Map::new(&env);
        
        for ((from, to), rate) in aggregated_rates.iter() {
            if from == from_currency {
                result.set(to, rate);
            }
        }

        result
    }

    fn add_rate_source(
        env: Env,
        name: Symbol,
        address: Address,
        weight: u8,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        if weight > 100 {
            panic!("Weight must be between 0 and 100");
        }

        let rate_source = RateSource {
            name: name.clone(),
            address: address.clone(),
            weight,
            active: true,
        };

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);

        let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
        let mut currencies = env.storage().persistent().get::<_, Vec<Symbol>>(&currencies_key)
            .unwrap_or_else(|| Vec::new(&env));
        
        if !currencies.contains(&name) {
            currencies.push_back(name);
            env.storage().persistent().set(&currencies_key, &currencies);
        }

        true
    }

    fn update_rate_source(
        env: Env,
        address: Address,
        weight: u8,
        active: bool,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        if weight > 100 {
            panic!("Weight must be between 0 and 100");
        }

        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let mut sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut rate_source = sources.get(address.clone())
            .unwrap_or_else(|| panic!("Rate source not found"));
        
        rate_source.weight = weight;
        rate_source.active = active;
        sources.set(address, rate_source);
        env.storage().persistent().set(&sources_key, &sources);

        true
    }

    fn get_rate_sources(env: Env) -> Vec<RateSource> {
        let sources_key = Symbol::new(&env, "RATE_SOURCES");
        let sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut result = Vec::new(&env);
        for (_, source) in sources.iter() {
            result.push_back(source);
        }

        result
    }

    fn set_admin(env: Env, admin: Address) {
        let admin_key = Symbol::new(&env, "ADMIN");
        env.storage().persistent().set(&admin_key, &admin);
    }

    fn set_deviation_threshold(env: Env, threshold: u32) {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let threshold_key = Symbol::new(&env, "DEVIATION_THRESHOLD");
        env.storage().persistent().set(&threshold_key, &threshold);
    }

    fn get_supported_currencies(env: Env) -> Vec<Symbol> {
        let currencies_key = Symbol::new(&env, "SUPPORTED_CURRENCIES");
        env.storage().persistent().get::<_, Vec<Symbol>>(&currencies_key)
            .unwrap_or_else(|| Vec::new(&env))
    }
}

impl RateOracleContract {
    fn update_aggregated_rate(env: &Env, from_currency: Symbol, to_currency: Symbol) {
        let rates_key = Symbol::new(env, "EXCHANGE_RATES");
        let rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), Vec<ExchangeRate>>>(&rates_key)
            .unwrap_or_else(|| Map::new(env));

        let sources_key = Symbol::new(env, "RATE_SOURCES");
        let sources = env.storage().persistent().get::<_, Map<Address, RateSource>>(&sources_key)
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
        let mut total_weight = 0u32;
        let mut valid_rates = Vec::new(env);

        for exchange_rate in rate_list.iter() {
            if let Some(source) = sources.get(Address::from_contract_id(&env.crypto().sha256(&exchange_rate.source.into()))) {
                if source.active && exchange_rate.confidence >= 50 {
                    let weight = source.weight as u32 * exchange_rate.confidence as u32;
                    weighted_sum += exchange_rate.rate as u128 * weight as u128;
                    total_weight += weight;
                    valid_rates.push_back(exchange_rate.rate);
                }
            }
        }

        if total_weight == 0 {
            return;
        }

        let weighted_average = weighted_sum / total_weight as u128;

        let deviation_threshold_key = Symbol::new(env, "DEVIATION_THRESHOLD");
        let deviation_threshold = env.storage().persistent().get::<_, u32>(&deviation_threshold_key)
            .unwrap_or(10);

        let mut filtered_rates = Vec::new(env);
        for rate in valid_rates.iter() {
            let deviation = if weighted_average > 0 {
                ((rate.abs_diff(weighted_average) as f64 / weighted_average as f64) * 100.0) as u32
            } else {
                0
            };
            
            if deviation <= deviation_threshold {
                filtered_rates.push_back(rate);
            }
        }

        let final_rate = if !filtered_rates.is_empty() {
            let sum: u128 = filtered_rates.iter().sum();
            sum / filtered_rates.len() as u128
        } else {
            weighted_average
        };

        let aggregated_rate = AggregatedRate {
            rate: final_rate,
            weighted_average,
            sources_count: filtered_rates.len() as u32,
            last_updated: env.ledger().timestamp(),
            deviation_threshold,
        };

        let aggregated_key = Symbol::new(env, "AGGREGATED_RATES");
        let mut aggregated_rates = env.storage().persistent().get::<_, Map<(Symbol, Symbol), AggregatedRate>>(&aggregated_key)
            .unwrap_or_else(|| Map::new(env));
        aggregated_rates.set(pair_key, aggregated_rate);
        env.storage().persistent().set(&aggregated_key, &aggregated_rates);
    }
}
