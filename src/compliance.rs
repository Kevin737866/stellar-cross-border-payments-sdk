use soroban_sdk::{contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Symbol, Map, Vec};

#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub enum ComplianceLevel {
    None,
    Basic,
    Enhanced,
    Full,
}

#[contracttype]
#[derive(PartialEq, Clone, Debug)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Restricted,
}

#[contracttype]
#[derive(Clone)]
pub struct ComplianceRecord {
    pub user: Address,
    pub kyc_level: ComplianceLevel,
    pub risk_level: RiskLevel,
    pub jurisdiction: Symbol,
    pub registration_date: u64,
    pub last_updated: u64,
    pub aml_flags: Vec<Symbol>,
    pub transaction_limits: Map<Symbol, i128>,
}

#[contracttype]
#[derive(Clone)]
pub struct TransactionRule {
    pub id: BytesN<32>,
    pub name: Symbol,
    pub description: Symbol,
    pub conditions: Map<Symbol, Bytes>,
    pub actions: Map<Symbol, Bytes>,
    pub active: bool,
    pub priority: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct ComplianceCheck {
    pub transaction_id: BytesN<32>,
    pub from_user: Address,
    pub to_user: Address,
    pub amount: i128,
    pub currency: Symbol,
    pub jurisdiction_from: Symbol,
    pub jurisdiction_to: Symbol,
    pub timestamp: u64,
    pub approved: bool,
    pub reason: Symbol,
    pub rules_triggered: Vec<BytesN<32>>,
}

pub struct ComplianceContract;

// #[contract]
// pub trait ComplianceTrait {
//     fn register_user(
//         env: Env,
//         user: Address,
//         kyc_level: ComplianceLevel,
//         risk_level: RiskLevel,
//         jurisdiction: Symbol,
//         aml_flags: Vec<Symbol>,
//         transaction_limits: Map<Symbol, i128>,
//     ) -> bool;
//
//     fn update_user_compliance(
//         env: Env,
//         user: Address,
//         kyc_level: ComplianceLevel,
//         risk_level: RiskLevel,
//         aml_flags: Vec<Symbol>,
//         transaction_limits: Map<Symbol, i128>,
//     ) -> bool;
//
//     fn check_transaction_compliance(
//         env: Env,
//         transaction_id: BytesN<32>,
//         from_user: Address,
//         to_user: Address,
//         amount: i128,
//         currency: Symbol,
//         jurisdiction_from: Symbol,
//         jurisdiction_to: Symbol,
//     ) -> ComplianceCheck;
//
//     fn add_compliance_rule(
//         env: Env,
//         name: Symbol,
//         description: Symbol,
//         conditions: Map<Symbol, Vec<u8>>,
//         actions: Map<Symbol, Vec<u8>>,
//         priority: u32,
//     ) -> BytesN<32>;
//
//     fn update_compliance_rule(
//         env: Env,
//         rule_id: BytesN<32>,
//         active: bool,
//         priority: u32,
//     ) -> bool;
//
//     fn get_user_compliance(env: Env, user: Address) -> ComplianceRecord;
//
//     fn get_compliance_rules(env: Env) -> Vec<TransactionRule>;
//
//     fn get_transaction_history(env: Env, user: Address) -> Vec<ComplianceCheck>;
//
//     fn set_admin(env: Env, admin: Address);
//
//     fn add_restricted_jurisdiction(env: Env, jurisdiction: Symbol);
//
//     fn remove_restricted_jurisdiction(env: Env, jurisdiction: Symbol);
//
//     fn is_jurisdiction_restricted(env: Env, jurisdiction: Symbol) -> bool;
// }

// #[contractimpl]
// impl ComplianceTrait for ComplianceContract {
impl ComplianceContract {
    fn register_user(
        env: Env,
        user: Address,
        kyc_level: ComplianceLevel,
        risk_level: RiskLevel,
        jurisdiction: Symbol,
        aml_flags: Vec<Symbol>,
        transaction_limits: Map<Symbol, i128>,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let compliance_record = ComplianceRecord {
            user: user.clone(),
            kyc_level,
            risk_level,
            jurisdiction,
            registration_date: env.ledger().timestamp(),
            last_updated: env.ledger().timestamp(),
            aml_flags,
            transaction_limits,
        };

        let records_key = Symbol::new(&env, "COMPLIANCE_RECORDS");
        let mut records = env.storage().persistent().get::<_, Map<Address, ComplianceRecord>>(&records_key)
            .unwrap_or_else(|| Map::new(&env));
        records.set(user, compliance_record);
        env.storage().persistent().set(&records_key, &records);

        true
    }

    fn update_user_compliance(
        env: Env,
        user: Address,
        kyc_level: ComplianceLevel,
        risk_level: RiskLevel,
        aml_flags: Vec<Symbol>,
        transaction_limits: Map<Symbol, i128>,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let records_key = Symbol::new(&env, "COMPLIANCE_RECORDS");
        let mut records = env.storage().persistent().get::<_, Map<Address, ComplianceRecord>>(&records_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut record = records.get(user.clone())
            .unwrap_or_else(|| panic!("User not registered"));

        record.kyc_level = kyc_level;
        record.risk_level = risk_level;
        record.aml_flags = aml_flags;
        record.transaction_limits = transaction_limits;
        record.last_updated = env.ledger().timestamp();

        records.set(user, record);
        env.storage().persistent().set(&records_key, &records);

        true
    }

    fn check_transaction_compliance(
        env: Env,
        transaction_id: BytesN<32>,
        from_user: Address,
        to_user: Address,
        amount: i128,
        currency: Symbol,
        jurisdiction_from: Symbol,
        jurisdiction_to: Symbol,
    ) -> ComplianceCheck {
        let restricted_key = Symbol::new(&env, "RESTRICTED_JURISDICTIONS");
        let restricted_jurisdictions = env.storage().persistent().get::<_, Vec<Symbol>>(&restricted_key)
            .unwrap_or_else(|| Vec::new(&env));

        if restricted_jurisdictions.contains(&jurisdiction_from) || restricted_jurisdictions.contains(&jurisdiction_to) {
            return ComplianceCheck {
                transaction_id,
                from_user: from_user.clone(),
                to_user: to_user.clone(),
                amount,
                currency,
                jurisdiction_from,
                jurisdiction_to,
                timestamp: env.ledger().timestamp(),
                approved: false,
                reason: Symbol::new(&env, "RESTRICTED_JURISDICTION"),
                rules_triggered: Vec::new(&env),
            };
        }

        let records_key = Symbol::new(&env, "COMPLIANCE_RECORDS");
        let records = env.storage().persistent().get::<_, Map<Address, ComplianceRecord>>(&records_key)
            .unwrap_or_else(|| Map::new(&env));

        let from_record = match records.get(from_user.clone()) {
            Some(record) => record,
            None => {
                return ComplianceCheck {
                    transaction_id,
                    from_user: from_user.clone(),
                    to_user: to_user.clone(),
                    amount,
                    currency,
                    jurisdiction_from,
                    jurisdiction_to,
                    timestamp: env.ledger().timestamp(),
                    approved: false,
                    reason: Symbol::new(&env, "SENDER_NOT_REGISTERED"),
                    rules_triggered: Vec::new(&env),
                };
            }
        };
        let to_record = match records.get(to_user.clone()) {
            Some(record) => record,
            None => {
                return ComplianceCheck {
                    transaction_id,
                    from_user,
                    to_user,
                    amount,
                    currency,
                    jurisdiction_from,
                    jurisdiction_to,
                    timestamp: env.ledger().timestamp(),
                    approved: false,
                    reason: Symbol::new(&env, "RECEIVER_NOT_REGISTERED"),
                    rules_triggered: Vec::new(&env),
                };
            }
        };

        if from_record.risk_level == RiskLevel::Restricted || to_record.risk_level == RiskLevel::Restricted {
            return ComplianceCheck {
                transaction_id,
                from_user: from_user.clone(),
                to_user: to_user.clone(),
                amount,
                currency,
                jurisdiction_from,
                jurisdiction_to,
                timestamp: env.ledger().timestamp(),
                approved: false,
                reason: Symbol::new(&env, "RESTRICTED_USER"),
                rules_triggered: Vec::new(&env),
            };
        }

        let from_limit = from_record.transaction_limits.get(currency.clone())
            .unwrap_or(0i128);
        if amount > from_limit {
            return ComplianceCheck {
                transaction_id,
                from_user: from_user.clone(),
                to_user: to_user.clone(),
                amount,
                currency,
                jurisdiction_from,
                jurisdiction_to,
                timestamp: env.ledger().timestamp(),
                approved: false,
                reason: Symbol::new(&env, "EXCEEDS_LIMIT"),
                rules_triggered: Vec::new(&env),
            };
        }

        let rules_key = Symbol::new(&env, "COMPLIANCE_RULES");
        let rules = env.storage().persistent().get::<_, Vec<TransactionRule>>(&rules_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut rules_triggered = Vec::new(&env);
        let mut approved = true;
        let mut reason = Symbol::new(&env, "APPROVED");

        for rule in rules.iter() {
            if rule.active && Self::evaluate_rule(&env, &rule, &from_record, &to_record, amount, &currency) {
                rules_triggered.push_back(rule.id);
                
                if rule.priority >= 8 {
                    approved = false;
                    reason = Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED");
                    break;
                }
            }
        }

        let compliance_check = ComplianceCheck {
            transaction_id,
            from_user,
            to_user,
            amount,
            currency,
            jurisdiction_from,
            jurisdiction_to,
            timestamp: env.ledger().timestamp(),
            approved,
            reason,
            rules_triggered,
        };

        let history_key = Symbol::new(&env, "COMPLIANCE_HISTORY");
        let mut history = env.storage().persistent().get::<_, Vec<ComplianceCheck>>(&history_key)
            .unwrap_or_else(|| Vec::new(&env));
        history.push_back(compliance_check.clone());
        env.storage().persistent().set(&history_key, &history);

        compliance_check
    }

    fn add_compliance_rule(
        env: Env,
        name: Symbol,
        description: Symbol,
        conditions: Map<Symbol, Bytes>,
        actions: Map<Symbol, Bytes>,
        priority: u32,
    ) -> BytesN<32> {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let rule_id = BytesN::from_array(&env, &[1u8; 32]);

        let rule = TransactionRule {
            id: rule_id.clone(),
            name,
            description,
            conditions,
            actions,
            active: true,
            priority,
        };

        let rules_key = Symbol::new(&env, "COMPLIANCE_RULES");
        let mut rules = env.storage().persistent().get::<_, Vec<TransactionRule>>(&rules_key)
            .unwrap_or_else(|| Vec::new(&env));
        rules.push_back(rule);
        env.storage().persistent().set(&rules_key, &rules);

        rule_id
    }

    fn update_compliance_rule(
        env: Env,
        rule_id: BytesN<32>,
        active: bool,
        priority: u32,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let rules_key = Symbol::new(&env, "COMPLIANCE_RULES");
        let mut rules = env.storage().persistent().get::<_, Vec<TransactionRule>>(&rules_key)
            .unwrap_or_else(|| Vec::new(&env));

        for i in 0..rules.len() {
            let rule = rules.get(i).unwrap();
            if rule.id == rule_id {
                let mut updated_rule = rule;
                updated_rule.active = active;
                updated_rule.priority = priority;
                rules.set(i, updated_rule);
                env.storage().persistent().set(&rules_key, &rules);
                return true;
            }
        }

        panic!("Rule not found")
    }

    fn get_user_compliance(env: Env, user: Address) -> ComplianceRecord {
        let records_key = Symbol::new(&env, "COMPLIANCE_RECORDS");
        let records = env.storage().persistent().get::<_, Map<Address, ComplianceRecord>>(&records_key)
            .unwrap_or_else(|| Map::new(&env));

        records.get(user)
            .unwrap_or_else(|| panic!("User not registered"))
    }

    fn get_compliance_rules(env: Env) -> Vec<TransactionRule> {
        let rules_key = Symbol::new(&env, "COMPLIANCE_RULES");
        env.storage().persistent().get::<_, Vec<TransactionRule>>(&rules_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    fn get_transaction_history(env: Env, user: Address) -> Vec<ComplianceCheck> {
        let history_key = Symbol::new(&env, "COMPLIANCE_HISTORY");
        let history = env.storage().persistent().get::<_, Vec<ComplianceCheck>>(&history_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut user_history = Vec::new(&env);
        for check in history.iter() {
            if check.from_user == user || check.to_user == user {
                user_history.push_back(check);
            }
        }

        user_history
    }

    fn set_admin(env: Env, admin: Address) {
        let admin_key = Symbol::new(&env, "ADMIN");
        env.storage().persistent().set(&admin_key, &admin);
    }

    fn add_restricted_jurisdiction(env: Env, jurisdiction: Symbol) {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let restricted_key = Symbol::new(&env, "RESTRICTED_JURISDICTIONS");
        let mut restricted = env.storage().persistent().get::<_, Vec<Symbol>>(&restricted_key)
            .unwrap_or_else(|| Vec::new(&env));
        
        if !restricted.contains(&jurisdiction) {
            restricted.push_back(jurisdiction);
            env.storage().persistent().set(&restricted_key, &restricted);
        }
    }

    fn remove_restricted_jurisdiction(env: Env, jurisdiction: Symbol) {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let restricted_key = Symbol::new(&env, "RESTRICTED_JURISDICTIONS");
        let mut restricted = env.storage().persistent().get::<_, Vec<Symbol>>(&restricted_key)
            .unwrap_or_else(|| Vec::new(&env));
        
        let mut new_restricted = Vec::new(&env);
        for j in restricted.iter() {
            if j != jurisdiction {
                new_restricted.push_back(j);
            }
        }
        env.storage().persistent().set(&restricted_key, &new_restricted);
    }

    fn is_jurisdiction_restricted(env: Env, jurisdiction: Symbol) -> bool {
        let restricted_key = Symbol::new(&env, "RESTRICTED_JURISDICTIONS");
        let restricted = env.storage().persistent().get::<_, Vec<Symbol>>(&restricted_key)
            .unwrap_or_else(|| Vec::new(&env));
        
        restricted.contains(&jurisdiction)
    }
}

impl ComplianceContract {
    fn evaluate_rule(
        env: &Env,
        rule: &TransactionRule,
        from_record: &ComplianceRecord,
        to_record: &ComplianceRecord,
        amount: i128,
        currency: &Symbol,
    ) -> bool {
        for (condition_key, condition_value) in rule.conditions.iter() {
            match condition_key.to_string().as_str() {
                "HIGH_RISK_SENDER" => {
                    if from_record.risk_level == RiskLevel::High {
                        return true;
                    }
                }
                "HIGH_RISK_RECEIVER" => {
                    if to_record.risk_level == RiskLevel::High {
                        return true;
                    }
                }
                "HIGH_AMOUNT_THRESHOLD" => {
                    let threshold = i128::from_be_bytes(condition_value.try_into().unwrap());
                    if amount > threshold {
                        return true;
                    }
                }
                "AML_FLAGGED_SENDER" => {
                    if !from_record.aml_flags.is_empty() {
                        return true;
                    }
                }
                "AML_FLAGGED_RECEIVER" => {
                    if !to_record.aml_flags.is_empty() {
                        return true;
                    }
                }
                "CROSS_BORDER_HIGH_VALUE" => {
                    let threshold = i128::from_be_bytes(condition_value.try_into().unwrap());
                    if from_record.jurisdiction != to_record.jurisdiction && amount > threshold {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_us_to_mexico_compliance_flow_approved() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[1u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, true);
            assert_eq!(check.amount, amount);
            assert_eq!(check.from_user, us_user);
            assert_eq!(check.to_user, mx_user);
            assert_eq!(check.jurisdiction_from, us_jurisdiction);
            assert_eq!(check.jurisdiction_to, mx_jurisdiction);
            assert_eq!(check.currency, usd);
        });
    }

    #[test]
    fn test_us_to_mexico_compliance_flow_restricted_jurisdiction() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let restricted_jurisdiction = Symbol::new(&env, "MX");
            ComplianceContract::add_restricted_jurisdiction(env.clone(), restricted_jurisdiction.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[2u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "RESTRICTED_JURISDICTION"));
        });
    }

    #[test]
    fn test_amount_threshold_exceeds_limit() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 1000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[3u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "EXCEEDS_LIMIT"));
            assert_eq!(check.amount, amount);
        });
    }

    #[test]
    fn test_aml_flagged_sender_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            let mut aml_flags = Vec::new(&env);
            aml_flags.push_back(Symbol::new(&env, "SANCTIONED"));

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                aml_flags.clone(),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "AML_FLAGGED_SENDER"),
                Bytes::new(&env),
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "AML_SENDER_RULE"),
                Symbol::new(&env, "Block AML flagged senders"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[4u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_aml_flagged_receiver_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            let mut aml_flags = Vec::new(&env);
            aml_flags.push_back(Symbol::new(&env, "SANCTIONED"));

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                aml_flags.clone(),
                limits.clone(),
            );

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "AML_FLAGGED_RECEIVER"),
                Bytes::new(&env),
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "AML_RECEIVER_RULE"),
                Symbol::new(&env, "Block AML flagged receivers"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[5u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_high_risk_sender_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::High,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "HIGH_RISK_SENDER"),
                Bytes::new(&env),
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "HIGH_RISK_SENDER_RULE"),
                Symbol::new(&env, "Block high risk senders"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[6u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_high_risk_receiver_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::High,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "HIGH_RISK_RECEIVER"),
                Bytes::new(&env),
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "HIGH_RISK_RECEIVER_RULE"),
                Symbol::new(&env, "Block high risk receivers"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[7u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_cross_border_high_value_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 100000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let threshold = 50000i128;
            let threshold_bytes = Bytes::from_slice(&env, &threshold.to_be_bytes());

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "CROSS_BORDER_HIGH_VALUE"),
                threshold_bytes,
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "CROSS_BORDER_RULE"),
                Symbol::new(&env, "Block high value cross-border transactions"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[8u8; 32]);
            let amount = 75000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_sender_not_registered_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[9u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "SENDER_NOT_REGISTERED"));
        });
    }

    #[test]
    fn test_receiver_not_registered_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[10u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "RECEIVER_NOT_REGISTERED"));
        });
    }

    #[test]
    fn test_restricted_user_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Restricted,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[11u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "RESTRICTED_USER"));
        });
    }

    #[test]
    fn test_low_priority_rule_triggered_but_approved() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::High,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "HIGH_RISK_SENDER"),
                Bytes::new(&env),
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "MONITOR"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "MONITOR_RULE"),
                Symbol::new(&env, "Monitor high risk senders"),
                conditions,
                actions,
                5,
            );

            let transaction_id = BytesN::from_array(&env, &[12u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, true);
            assert_eq!(check.reason, Symbol::new(&env, "APPROVED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_high_amount_threshold_rule() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 100000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let threshold = 10000i128;
            let threshold_bytes = Bytes::from_slice(&env, &threshold.to_be_bytes());

            let mut conditions = Map::new(&env);
            conditions.set(
                Symbol::new(&env, "HIGH_AMOUNT_THRESHOLD"),
                threshold_bytes,
            );

            let mut actions = Map::new(&env);
            actions.set(
                Symbol::new(&env, "BLOCK"),
                Bytes::new(&env),
            );

            let rule_id = ComplianceContract::add_compliance_rule(
                env.clone(),
                Symbol::new(&env, "HIGH_AMOUNT_RULE"),
                Symbol::new(&env, "Block high amount transactions"),
                conditions,
                actions,
                10,
            );

            let transaction_id = BytesN::from_array(&env, &[13u8; 32]);
            let amount = 15000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, false);
            assert_eq!(check.reason, Symbol::new(&env, "HIGH_PRIORITY_RULE_TRIGGERED"));
            assert_eq!(check.rules_triggered.len(), 1);
            assert_eq!(check.rules_triggered.get(0).unwrap(), rule_id);
        });
    }

    #[test]
    fn test_compliance_check_return_values_approved() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = BytesN::from_array(&env, &[0u8; 32]);

        env.as_contract(&contract_id, || {
            let admin = Address::generate(&env);
            ComplianceContract::set_admin(env.clone(), admin.clone());

            let us_user = Address::generate(&env);
            let mx_user = Address::generate(&env);

            let us_jurisdiction = Symbol::new(&env, "US");
            let mx_jurisdiction = Symbol::new(&env, "MX");
            let usd = Symbol::new(&env, "USD");

            let mut limits = Map::new(&env);
            limits.set(usd.clone(), 10000i128);

            ComplianceContract::register_user(
                env.clone(),
                us_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                us_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            ComplianceContract::register_user(
                env.clone(),
                mx_user.clone(),
                ComplianceLevel::Enhanced,
                RiskLevel::Low,
                mx_jurisdiction.clone(),
                Vec::new(&env),
                limits.clone(),
            );

            let transaction_id = BytesN::from_array(&env, &[14u8; 32]);
            let amount = 5000i128;

            let check = ComplianceContract::check_transaction_compliance(
                env.clone(),
                transaction_id.clone(),
                us_user.clone(),
                mx_user.clone(),
                amount,
                usd.clone(),
                us_jurisdiction.clone(),
                mx_jurisdiction.clone(),
            );

            assert_eq!(check.approved, true);
            assert_eq!(check.reason, Symbol::new(&env, "APPROVED"));
            assert_eq!(check.transaction_id, transaction_id);
            assert_eq!(check.from_user, us_user);
            assert_eq!(check.to_user, mx_user);
            assert_eq!(check.amount, amount);
            assert_eq!(check.currency, usd);
            assert_eq!(check.jurisdiction_from, us_jurisdiction);
            assert_eq!(check.jurisdiction_to, mx_jurisdiction);
        });
    }

    #[test]
    fn test_compliance_check_return_values_denied() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let restricted_jurisdiction = Symbol::new(&env, "MX");
        ComplianceContract::add_restricted_jurisdiction(env.clone(), restricted_jurisdiction);

        let us_user = Address::generate(&env);
        let mx_user = Address::generate(&env);

        let us_jurisdiction = Symbol::new(&env, "US");
        let mx_jurisdiction = Symbol::new(&env, "MX");
        let usd = Symbol::new(&env, "USD");

        let mut limits = Map::new(&env);
        limits.set(usd.clone(), 10000i128);

        ComplianceContract::register_user(
            env.clone(),
            us_user.clone(),
            ComplianceLevel::Enhanced,
            RiskLevel::Low,
            us_jurisdiction.clone(),
            Vec::new(&env),
            limits.clone(),
        );

        ComplianceContract::register_user(
            env.clone(),
            mx_user.clone(),
            ComplianceLevel::Enhanced,
            RiskLevel::Low,
            mx_jurisdiction.clone(),
            Vec::new(&env),
            limits.clone(),
        );

        let transaction_id = BytesN::from_array(&env, &[15u8; 32]);
        let amount = 5000i128;

        let check = ComplianceContract::check_transaction_compliance(
            env.clone(),
            transaction_id.clone(),
            us_user.clone(),
            mx_user.clone(),
            amount,
            usd.clone(),
            us_jurisdiction.clone(),
            mx_jurisdiction.clone(),
        );

        assert_eq!(check.approved, false);
        assert_eq!(check.reason, Symbol::new(&env, "RESTRICTED_JURISDICTION"));
        assert_eq!(check.transaction_id, transaction_id);
        assert_eq!(check.from_user, us_user);
        assert_eq!(check.to_user, mx_user);
        assert_eq!(check.amount, amount);
        assert_eq!(check.currency, usd);
        assert_eq!(check.jurisdiction_from, us_jurisdiction);
        assert_eq!(check.jurisdiction_to, mx_jurisdiction);
    }

    #[test]
    fn test_is_jurisdiction_restricted() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let restricted_jurisdiction = Symbol::new(&env, "MX");
        ComplianceContract::add_restricted_jurisdiction(env.clone(), restricted_jurisdiction.clone());

        assert_eq!(
            ComplianceContract::is_jurisdiction_restricted(env.clone(), restricted_jurisdiction.clone()),
            true
        );

        let non_restricted = Symbol::new(&env, "US");
        assert_eq!(
            ComplianceContract::is_jurisdiction_restricted(env.clone(), non_restricted),
            false
        );
    }

    #[test]
    fn test_remove_restricted_jurisdiction() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let restricted_jurisdiction = Symbol::new(&env, "MX");
        ComplianceContract::add_restricted_jurisdiction(env.clone(), restricted_jurisdiction.clone());

        assert_eq!(
            ComplianceContract::is_jurisdiction_restricted(env.clone(), restricted_jurisdiction.clone()),
            true
        );

        ComplianceContract::remove_restricted_jurisdiction(env.clone(), restricted_jurisdiction.clone());

        assert_eq!(
            ComplianceContract::is_jurisdiction_restricted(env.clone(), restricted_jurisdiction),
            false
        );
    }

    #[test]
    fn test_get_user_compliance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let user = Address::generate(&env);
        let jurisdiction = Symbol::new(&env, "US");
        let usd = Symbol::new(&env, "USD");

        let mut limits = Map::new(&env);
        limits.set(usd.clone(), 10000i128);

        ComplianceContract::register_user(
            env.clone(),
            user.clone(),
            ComplianceLevel::Enhanced,
            RiskLevel::Low,
            jurisdiction,
            Vec::new(&env),
            limits.clone(),
        );

        let record = ComplianceContract::get_user_compliance(env.clone(), user.clone());

        assert_eq!(record.user, user);
        assert_eq!(record.kyc_level, ComplianceLevel::Enhanced);
        assert_eq!(record.risk_level, RiskLevel::Low);
        assert_eq!(record.aml_flags.len(), 0);
    }

    #[test]
    fn test_update_user_compliance() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let user = Address::generate(&env);
        let jurisdiction = Symbol::new(&env, "US");
        let usd = Symbol::new(&env, "USD");

        let mut limits = Map::new(&env);
        limits.set(usd.clone(), 10000i128);

        ComplianceContract::register_user(
            env.clone(),
            user.clone(),
            ComplianceLevel::Basic,
            RiskLevel::Low,
            jurisdiction,
            Vec::new(&env),
            limits.clone(),
        );

        let mut new_limits = Map::new(&env);
        new_limits.set(usd.clone(), 20000i128);

        ComplianceContract::update_user_compliance(
            env.clone(),
            user.clone(),
            ComplianceLevel::Full,
            RiskLevel::Medium,
            Vec::new(&env),
            new_limits.clone(),
        );

        let record = ComplianceContract::get_user_compliance(env.clone(), user.clone());

        assert_eq!(record.kyc_level, ComplianceLevel::Full);
        assert_eq!(record.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn test_get_compliance_rules() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let mut conditions = Map::new(&env);
        conditions.set(
            Symbol::new(&env, "HIGH_RISK_SENDER"),
            Bytes::new(&env),
        );

        let mut actions = Map::new(&env);
        actions.set(
            Symbol::new(&env, "BLOCK"),
            Bytes::new(&env),
        );

        ComplianceContract::add_compliance_rule(
            env.clone(),
            Symbol::new(&env, "RULE1"),
            Symbol::new(&env, "Description1"),
            conditions.clone(),
            actions.clone(),
            10,
        );

        ComplianceContract::add_compliance_rule(
            env.clone(),
            Symbol::new(&env, "RULE2"),
            Symbol::new(&env, "Description2"),
            conditions,
            actions,
            5,
        );

        let rules = ComplianceContract::get_compliance_rules(env.clone());

        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_update_compliance_rule() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let mut conditions = Map::new(&env);
        conditions.set(
            Symbol::new(&env, "HIGH_RISK_SENDER"),
            Bytes::new(&env),
        );

        let mut actions = Map::new(&env);
        actions.set(
            Symbol::new(&env, "BLOCK"),
            Bytes::new(&env),
        );

        let rule_id = ComplianceContract::add_compliance_rule(
            env.clone(),
            Symbol::new(&env, "RULE1"),
            Symbol::new(&env, "Description1"),
            conditions,
            actions,
            10,
        );

        let updated = ComplianceContract::update_compliance_rule(env.clone(), rule_id, false, 5);

        assert_eq!(updated, true);

        let rules = ComplianceContract::get_compliance_rules(env.clone());
        let rule = rules.get(0).unwrap();

        assert_eq!(rule.active, false);
        assert_eq!(rule.priority, 5);
    }

    #[test]
    fn test_transaction_history() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        ComplianceContract::set_admin(env.clone(), admin.clone());

        let us_user = Address::generate(&env);
        let mx_user = Address::generate(&env);

        let us_jurisdiction = Symbol::new(&env, "US");
        let mx_jurisdiction = Symbol::new(&env, "MX");
        let usd = Symbol::new(&env, "USD");

        let mut limits = Map::new(&env);
        limits.set(usd.clone(), 10000i128);

        ComplianceContract::register_user(
            env.clone(),
            us_user.clone(),
            ComplianceLevel::Enhanced,
            RiskLevel::Low,
            us_jurisdiction.clone(),
            Vec::new(&env),
            limits.clone(),
        );

        ComplianceContract::register_user(
            env.clone(),
            mx_user.clone(),
            ComplianceLevel::Enhanced,
            RiskLevel::Low,
            mx_jurisdiction.clone(),
            Vec::new(&env),
            limits.clone(),
        );

        let transaction_id = BytesN::from_array(&env, &[16u8; 32]);
        let amount = 5000i128;

        ComplianceContract::check_transaction_compliance(
            env.clone(),
            transaction_id.clone(),
            us_user.clone(),
            mx_user.clone(),
            amount,
            usd.clone(),
            us_jurisdiction.clone(),
            mx_jurisdiction.clone(),
        );

        let history = ComplianceContract::get_transaction_history(env.clone(), us_user.clone());

        assert_eq!(history.len(), 1);
        assert_eq!(history.get(0).unwrap().from_user, us_user);
    }
}
