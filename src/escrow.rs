use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Bytes, BytesN, Env, Map, Symbol, Vec,
};
use soroban_sdk::token::Client as TokenClient;

// ─── Data types ─────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, PartialEq)]
pub enum EscrowStatus {
    Pending,
    Completed,
    Refunded,
    Disputed,
}

/// Metadata stored alongside a dispute submission.
#[contracttype]
#[derive(Clone)]
pub struct DisputeEvidence {
    pub dispute_id:   u64,
    pub submitter:    Address,
    pub evidence:     Bytes,
    pub submitted_at: u64,
    pub description:  Symbol,
}

/// Core escrow record.
/// `#[contracttype]` is required for Soroban persistent-storage serialisation.
#[contracttype]
#[derive(Clone)]
pub struct Escrow {
    pub id:           BytesN<32>,
    pub sender:       Address,
    pub receiver:     Address,
    pub amount:       i128,
    pub token:        Address,
    pub status:       EscrowStatus,
    pub release_time: u64,
    pub created_at:   u64,
    pub metadata:     Map<Symbol, Vec<u8>>,
}

#[contracttype]
#[derive(Clone)]
pub struct Dispute {
    pub escrow_id:  BytesN<32>,
    pub challenger: Address,
    pub reason:     Symbol,
    pub evidence:   Vec<u8>,
    pub created_at: u64,
    pub resolved:   bool,
}

// ─── Storage-key constants ───────────────────────────────────────────────────

const ADMIN_KEY:           &str = "ADMIN";
const INIT_KEY:            &str = "INITIALIZED";
const ESCROW_KEY:          &str = "ESCROW";
const DISPUTES_KEY:        &str = "DISPUTES";
const EVIDENCE_KEY:        &str = "DISPUTE_EVIDENCE";
const DISPUTES_BY_ESCROW:  &str = "DISPUTES_BY_ESCROW";
const DISPUTES_BY_USER:    &str = "DISPUTES_BY_USER";

// ─── Stand-alone evidence helpers (used in tests) ────────────────────────────

pub fn store_evidence(env: &Env, dispute_id: u64, evidence: DisputeEvidence) {
    let key = (Symbol::new(env, EVIDENCE_KEY), dispute_id);
    env.storage().persistent().set(&key, &evidence);
}

pub fn get_evidence(env: &Env, dispute_id: u64) -> Option<DisputeEvidence> {
    let key = (Symbol::new(env, EVIDENCE_KEY), dispute_id);
    env.storage().persistent().get(&key)
}

// ─── Contract ────────────────────────────────────────────────────────────────

pub struct EscrowContract;

#[contract]
pub trait EscrowTrait {
    /// One-time initialisation — sets the admin address.
    /// Must be called before any other admin-gated function.
    fn initialize(env: Env, admin: Address) -> bool;

    fn create_escrow(
        env: Env,
        sender: Address,
        receiver: Address,
        amount: i128,
        token: Address,
        release_time: u64,
        metadata: Map<Symbol, Vec<u8>>,
    ) -> BytesN<32>;

    fn release_escrow(env: Env, escrow_id: BytesN<32>) -> bool;

    fn refund_escrow(env: Env, escrow_id: BytesN<32>) -> bool;

    fn dispute_escrow(
        env: Env,
        escrow_id: BytesN<32>,
        challenger: Address,
        reason: Symbol,
        evidence: Vec<u8>,
    ) -> BytesN<32>;

    fn resolve_dispute(
        env: Env,
        dispute_id: BytesN<32>,
        in_favor_of_challenger: bool,
    ) -> bool;

    fn get_escrow(env: Env, escrow_id: BytesN<32>) -> Escrow;

    fn get_escrow_status(env: Env, escrow_id: BytesN<32>) -> EscrowStatus;

    fn get_dispute(env: Env, dispute_id: BytesN<32>) -> Dispute;

    fn get_disputes_by_escrow(env: Env, escrow_id: BytesN<32>) -> Vec<BytesN<32>>;

    fn get_dispute_by_escrow(env: Env, escrow_id: BytesN<32>) -> Option<Dispute>;

    fn get_user_escrows(env: Env, user: Address) -> Vec<BytesN<32>>;

    fn get_user_disputes(env: Env, user: Address) -> Vec<BytesN<32>>;
}

#[contractimpl]
impl EscrowTrait for EscrowContract {
    // ── initialize ──────────────────────────────────────────────────────────

    fn initialize(env: Env, admin: Address) -> bool {
        let init_key = Symbol::new(&env, INIT_KEY);
        if env.storage().persistent().has(&init_key) {
            panic!("Contract already initialized");
        }
        env.storage().persistent().set(&Symbol::new(&env, ADMIN_KEY), &admin);
        env.storage().persistent().set(&init_key, &true);
        true
    }

    // ── create_escrow ────────────────────────────────────────────────────────
    //
    // Transfers `amount` tokens from `sender` into the contract's own address,
    // locking the funds until release or refund.

    fn create_escrow(
        env: Env,
        sender: Address,
        receiver: Address,
        amount: i128,
        token: Address,
        release_time: u64,
        metadata: Map<Symbol, Vec<u8>>,
    ) -> BytesN<32> {
        sender.require_auth();

        assert!(amount > 0, "Amount must be positive");
        assert!(
            release_time > env.ledger().timestamp(),
            "Release time must be in the future"
        );

        // ── Transfer tokens from sender → contract (lock funds) ──────────────
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&sender, &env.current_contract_address(), &amount);

        // ── Build and store escrow record ────────────────────────────────────
        let escrow_id: BytesN<32> = env.crypto().sha256(
            &(sender.clone(), receiver.clone(), amount, env.ledger().timestamp()).into(),
        );

        let escrow = Escrow {
            id: escrow_id.clone(),
            sender: sender.clone(),
            receiver: receiver.clone(),
            amount,
            token: token.clone(),
            status: EscrowStatus::Pending,
            release_time,
            created_at: env.ledger().timestamp(),
            metadata,
        };

        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let mut escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        escrows.set(escrow_id.clone(), escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        // Index by sender for fast user-escrow lookup
        let sender_key = (Symbol::new(&env, "USER_ESCROWS"), sender.clone());
        let mut sender_escrows = env
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&sender_key)
            .unwrap_or_else(|| Vec::new(&env));
        sender_escrows.push_back(escrow_id.clone());
        env.storage().persistent().set(&sender_key, &sender_escrows);

        // Also index by receiver
        let receiver_key = (Symbol::new(&env, "USER_ESCROWS"), receiver.clone());
        let mut receiver_escrows = env
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&receiver_key)
            .unwrap_or_else(|| Vec::new(&env));
        receiver_escrows.push_back(escrow_id.clone());
        env.storage().persistent().set(&receiver_key, &receiver_escrows);

        escrow_id
    }

    // ── release_escrow ───────────────────────────────────────────────────────
    //
    // Receiver calls this after the time-lock expires.
    // Transfers the held tokens from the contract → receiver.

    fn release_escrow(env: Env, escrow_id: BytesN<32>) -> bool {
        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let mut escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut escrow = escrows
            .get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        assert!(
            escrow.status == EscrowStatus::Pending,
            "Escrow is not in pending status"
        );
        assert!(
            env.ledger().timestamp() >= escrow.release_time,
            "Escrow is still time-locked"
        );

        escrow.receiver.require_auth();

        // ── Transfer tokens: contract → receiver ─────────────────────────────
        let token_client = TokenClient::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.receiver, &escrow.amount);

        escrow.status = EscrowStatus::Completed;
        escrows.set(escrow_id, escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    // ── refund_escrow ────────────────────────────────────────────────────────
    //
    // Sender cancels the escrow before it's released.
    // Transfers the held tokens from the contract → sender.

    fn refund_escrow(env: Env, escrow_id: BytesN<32>) -> bool {
        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let mut escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut escrow = escrows
            .get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        assert!(
            escrow.status == EscrowStatus::Pending,
            "Escrow is not in pending status"
        );

        assert!(
            escrow.release_time <= env.ledger().timestamp(),
            "Escrow is still time-locked"
        );

        escrow.sender.require_auth();

        // ── Transfer tokens: contract → sender ──────────────────────────────
        let token_client = TokenClient::new(&env, &escrow.token);
        token_client.transfer(&env.current_contract_address(), &escrow.sender, &escrow.amount);

        escrow.status = EscrowStatus::Refunded;
        escrows.set(escrow_id, escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    // ── dispute_escrow ───────────────────────────────────────────────────────
    //
    // Either party can open a dispute while the escrow is Pending.
    // Funds remain locked in the contract until resolve_dispute is called.
    // Returns the dispute_id for reliable tracking and retrieval.

    fn dispute_escrow(
        env: Env,
        escrow_id: BytesN<32>,
        challenger: Address,
        reason: Symbol,
        evidence: Vec<u8>,
    ) -> BytesN<32> {
        challenger.require_auth();

        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let mut escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut escrow = escrows
            .get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        assert!(
            escrow.status == EscrowStatus::Pending,
            "Escrow is not in pending status"
        );

        // Only sender or receiver may raise a dispute
        assert!(
            challenger == escrow.sender || challenger == escrow.receiver,
            "Only escrow parties may dispute"
        );

        // Mark as disputed — tokens remain locked in the contract
        escrow.status = EscrowStatus::Disputed;
        escrows.set(escrow_id.clone(), escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        let dispute_id: BytesN<32> = env.crypto().sha256(
            &(escrow_id.clone(), challenger.clone(), env.ledger().timestamp()).into(),
        );

        let dispute = Dispute {
            escrow_id: escrow_id.clone(),
            challenger: challenger.clone(),
            reason,
            evidence,
            created_at: env.ledger().timestamp(),
            resolved: false,
        };

        // ── Store the dispute in main disputes map ──────────────────────────
        let disputes_key = Symbol::new(&env, DISPUTES_KEY);
        let mut disputes = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));
        disputes.set(dispute_id.clone(), dispute);
        env.storage().persistent().set(&disputes_key, &disputes);

        // ── Index dispute by escrow_id for fast lookup ──────────────────────
        let escrow_disputes_key = (Symbol::new(&env, DISPUTES_BY_ESCROW), escrow_id.clone());
        let mut escrow_disputes = env
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&escrow_disputes_key)
            .unwrap_or_else(|| Vec::new(&env));
        escrow_disputes.push_back(dispute_id.clone());
        env.storage().persistent().set(&escrow_disputes_key, &escrow_disputes);

        // ── Index dispute by challenger user for user's dispute history ─────
        let user_disputes_key = (Symbol::new(&env, DISPUTES_BY_USER), challenger.clone());
        let mut user_disputes = env
            .storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&user_disputes_key)
            .unwrap_or_else(|| Vec::new(&env));
        user_disputes.push_back(dispute_id.clone());
        env.storage().persistent().set(&user_disputes_key, &user_disputes);

        dispute_id
    }

    // ── resolve_dispute ──────────────────────────────────────────────────────
    //
    // Admin-only. Transfers funds to the winning party:
    //   - in_favor_of_challenger=true  → refund to sender (challenger or party 1)
    //   - in_favor_of_challenger=false → release to receiver

    fn resolve_dispute(
        env: Env,
        dispute_id: BytesN<32>,
        in_favor_of_challenger: bool,
    ) -> bool {
        let admin_key = Symbol::new(&env, ADMIN_KEY);
        let admin = env
            .storage()
            .persistent()
            .get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Contract not initialized"));
        admin.require_auth();

        let disputes_key = Symbol::new(&env, DISPUTES_KEY);
        let mut disputes = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut dispute = disputes
            .get(dispute_id.clone())
            .unwrap_or_else(|| panic!("Dispute not found"));

        assert!(!dispute.resolved, "Dispute already resolved");

        dispute.resolved = true;
        disputes.set(dispute_id.clone(), dispute.clone());
        env.storage().persistent().set(&disputes_key, &disputes);

        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let mut escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));

        let mut escrow = escrows
            .get(dispute.escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        let token_client = TokenClient::new(&env, &escrow.token);

        if in_favor_of_challenger {
            // ── Refund locked tokens → sender ────────────────────────────────
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.sender,
                &escrow.amount,
            );
            escrow.status = EscrowStatus::Refunded;
        } else {
            // ── Release locked tokens → receiver ─────────────────────────────
            token_client.transfer(
                &env.current_contract_address(),
                &escrow.receiver,
                &escrow.amount,
            );
            escrow.status = EscrowStatus::Completed;
        }

        escrows.set(dispute.escrow_id, escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    // ── read-only queries ────────────────────────────────────────────────────

    fn get_escrow(env: Env, escrow_id: BytesN<32>) -> Escrow {
        let escrow_key = Symbol::new(&env, ESCROW_KEY);
        let escrows = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));

        escrows
            .get(escrow_id)
            .unwrap_or_else(|| panic!("Escrow not found"))
    }

    fn get_escrow_status(env: Env, escrow_id: BytesN<32>) -> EscrowStatus {
        Self::get_escrow(env, escrow_id).status
    }

    fn get_dispute(env: Env, dispute_id: BytesN<32>) -> Dispute {
        let disputes_key = Symbol::new(&env, DISPUTES_KEY);
        let disputes = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));

        disputes
            .get(dispute_id)
            .unwrap_or_else(|| panic!("Dispute not found"))
    }

    /// Get all disputes associated with a specific escrow.
    /// Returns a vector of dispute IDs for the given escrow.
    fn get_disputes_by_escrow(env: Env, escrow_id: BytesN<32>) -> Vec<BytesN<32>> {
        let escrow_disputes_key = (Symbol::new(&env, DISPUTES_BY_ESCROW), escrow_id);
        env.storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&escrow_disputes_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get the most recent (or only) dispute for an escrow if it exists.
    /// Useful for simple escrow + dispute workflows.
    fn get_dispute_by_escrow(env: Env, escrow_id: BytesN<32>) -> Option<Dispute> {
        let dispute_ids = Self::get_disputes_by_escrow(env.clone(), escrow_id);
        
        if dispute_ids.len() > 0 {
            // Return the last (most recent) dispute
            let last_dispute_id = dispute_ids.get(dispute_ids.len() - 1);
            Some(Self::get_dispute(env, last_dispute_id))
        } else {
            None
        }
    }

    fn get_user_escrows(env: Env, user: Address) -> Vec<BytesN<32>> {
        let user_key = (Symbol::new(&env, "USER_ESCROWS"), user);
        env.storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&user_key)
            .unwrap_or_else(|| Vec::new(&env))
    }

    /// Get all disputes initiated by a specific user (as challenger).
    fn get_user_disputes(env: Env, user: Address) -> Vec<BytesN<32>> {
        let user_disputes_key = (Symbol::new(&env, DISPUTES_BY_USER), user);
        env.storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&user_disputes_key)
            .unwrap_or_else(|| Vec::new(&env))
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn test_evidence_stored_and_retrieved() {
        let env = Env::default();
        let submitter = Address::generate(&env);
        let evidence_bytes = Bytes::from_slice(&env, b"proof_hash_abc");

        let evidence = DisputeEvidence {
            dispute_id:   1,
            submitter:    submitter.clone(),
            evidence:     evidence_bytes.clone(),
            submitted_at: env.ledger().timestamp(),
            description:  Symbol::new(&env, "payment_proof"),
        };

        store_evidence(&env, 1, evidence);
        let retrieved = get_evidence(&env, 1).expect("evidence should exist");
        assert_eq!(retrieved.dispute_id, 1);
        assert_eq!(retrieved.evidence, evidence_bytes);
    }

    #[test]
    fn test_missing_evidence_returns_none() {
        let env = Env::default();
        assert!(get_evidence(&env, 999).is_none());
    }

    #[test]
    fn test_dispute_creation_returns_dispute_id() {
        let env = Env::default();
        let contract = EscrowContract;
        
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        let challenger = sender.clone();
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        // Create a token for the escrow
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            challenger.clone(),
            Symbol::new(&env, "PAYMENT_NOT_RECEIVED"),
            Vec::new(&env),
        );
        
        // Verify dispute_id is returned (not a bool)
        assert!(!dispute_id.is_zero()); // Should be a non-zero hash
        
        // Verify we can retrieve the dispute by ID
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert_eq!(dispute.escrow_id, escrow_id);
        assert_eq!(dispute.challenger, challenger);
    }

    #[test]
    fn test_get_disputes_by_escrow() {
        let env = Env::default();
        let contract = EscrowContract;
        
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "PAYMENT_NOT_RECEIVED"),
            Vec::new(&env),
        );
        
        // Retrieve disputes by escrow ID
        let dispute_ids = EscrowTrait::get_disputes_by_escrow(env.clone(), escrow_id.clone());
        assert_eq!(dispute_ids.len(), 1);
        assert_eq!(dispute_ids.get(0), dispute_id);
    }

    #[test]
    fn test_get_dispute_by_escrow_returns_latest() {
        let env = Env::default();
        let contract = EscrowContract;
        
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create first dispute
        let _dispute_id_1 = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "FIRST_DISPUTE"),
            Vec::new(&env),
        );
        
        // Retrieve dispute by escrow (convenience method)
        let dispute_opt = EscrowTrait::get_dispute_by_escrow(env.clone(), escrow_id.clone());
        assert!(dispute_opt.is_some());
        let dispute = dispute_opt.unwrap();
        assert_eq!(dispute.escrow_id, escrow_id);
    }

    #[test]
    fn test_get_user_disputes() {
        let env = Env::default();
        let contract = EscrowContract;
        
        let admin = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow 1
        let escrow_id_1 = EscrowTrait::create_escrow(
            env.clone(),
            user1.clone(),
            user2.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // User1 creates dispute
        let dispute_id_1 = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id_1.clone(),
            user1.clone(),
            Symbol::new(&env, "DISPUTE1"),
            Vec::new(&env),
        );
        
        // Retrieve user1's disputes
        let user_disputes = EscrowTrait::get_user_disputes(env.clone(), user1.clone());
        assert_eq!(user_disputes.len(), 1);
        assert_eq!(user_disputes.get(0), dispute_id_1);
        
        // User2 should have no disputes created
        let user2_disputes = EscrowTrait::get_user_disputes(env.clone(), user2.clone());
        assert_eq!(user2_disputes.len(), 0);
    }

    #[test]
    fn test_create_escrow_generates_valid_id() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let amount = 1000;
        let release_time = env.ledger().timestamp() + 1000;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            amount,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Verify escrow_id is not zero
        assert!(!escrow_id.is_zero());
        
        // Verify escrow can be retrieved
        let escrow = EscrowTrait::get_escrow(env.clone(), escrow_id.clone());
        assert_eq!(escrow.id, escrow_id);
        assert_eq!(escrow.sender, sender);
        assert_eq!(escrow.receiver, receiver);
        assert_eq!(escrow.amount, amount);
        assert_eq!(escrow.token, token);
        assert_eq!(escrow.status, EscrowStatus::Pending);
        assert_eq!(escrow.release_time, release_time);
    }

    #[test]
    fn test_create_escrow_validates_amount_positive() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Attempt to create escrow with zero amount - should panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::create_escrow(
                env.clone(),
                sender.clone(),
                receiver.clone(),
                0,
                token.clone(),
                env.ledger().timestamp() + 1000,
                Map::new(&env),
            );
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_create_escrow_validates_future_release_time() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let current_time = env.ledger().timestamp();
        
        // Attempt to create escrow with past release time - should panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::create_escrow(
                env.clone(),
                sender.clone(),
                receiver.clone(),
                1000,
                token.clone(),
                current_time - 100,
                Map::new(&env),
            );
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_create_escrow_indexes_by_sender_and_receiver() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Verify sender can retrieve their escrows
        let sender_escrows = EscrowTrait::get_user_escrows(env.clone(), sender.clone());
        assert_eq!(sender_escrows.len(), 1);
        assert_eq!(sender_escrows.get(0), escrow_id);
        
        // Verify receiver can retrieve their escrows
        let receiver_escrows = EscrowTrait::get_user_escrows(env.clone(), receiver.clone());
        assert_eq!(receiver_escrows.len(), 1);
        assert_eq!(receiver_escrows.get(0), escrow_id);
    }

    #[test]
    fn test_time_lock_release_after_expiration() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Verify escrow is pending
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Pending
        );
        
        // Attempt to release before time-lock expires - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        }));
        assert!(result.is_err());
        
        // Advance ledger time past release_time
        env.ledger().set_timestamp(release_time + 1);
        
        // Release should now succeed
        let released = EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        assert!(released);
        
        // Verify escrow status is completed
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Completed
        );
    }

    #[test]
    fn test_time_lock_release_at_exact_expiration() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Set ledger time to exactly release_time
        env.ledger().set_timestamp(release_time);
        
        // Release should succeed at exact expiration time
        let released = EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        assert!(released);
        
        // Verify escrow status is completed
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Completed
        );
    }

    #[test]
    fn test_refund_fallback_after_expiration() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Attempt to refund before time-lock expires - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::refund_escrow(env.clone(), escrow_id.clone());
        }));
        assert!(result.is_err());
        
        // Advance ledger time past release_time
        env.ledger().set_timestamp(release_time + 1);
        
        // Refund should now succeed
        let refunded = EscrowTrait::refund_escrow(env.clone(), escrow_id.clone());
        assert!(refunded);
        
        // Verify escrow status is refunded
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Refunded
        );
    }

    #[test]
    fn test_refund_fallback_at_exact_expiration() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Set ledger time to exactly release_time
        env.ledger().set_timestamp(release_time);
        
        // Refund should succeed at exact expiration time
        let refunded = EscrowTrait::refund_escrow(env.clone(), escrow_id.clone());
        assert!(refunded);
        
        // Verify escrow status is refunded
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Refunded
        );
    }

    #[test]
    fn test_cannot_release_completed_escrow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Advance time and release
        env.ledger().set_timestamp(release_time + 1);
        EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        
        // Attempt to release again - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_cannot_refund_refunded_escrow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Advance time and refund
        env.ledger().set_timestamp(release_time + 1);
        EscrowTrait::refund_escrow(env.clone(), escrow_id.clone());
        
        // Attempt to refund again - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::refund_escrow(env.clone(), escrow_id.clone());
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_creation_transitions_to_disputed_status() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Verify initial status is pending
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Pending
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "PAYMENT_NOT_RECEIVED"),
            Vec::new(&env),
        );
        
        // Verify dispute was created
        assert!(!dispute_id.is_zero());
        
        // Verify escrow status is now disputed
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Disputed
        );
    }

    #[test]
    fn test_dispute_creation_by_sender() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Sender creates dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        
        // Verify dispute was created
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert_eq!(dispute.challenger, sender);
        assert_eq!(dispute.escrow_id, escrow_id);
    }

    #[test]
    fn test_dispute_creation_by_receiver() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Receiver creates dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            receiver.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        
        // Verify dispute was created
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert_eq!(dispute.challenger, receiver);
        assert_eq!(dispute.escrow_id, escrow_id);
    }

    #[test]
    fn test_dispute_creation_by_unauthorized_party_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        let unauthorized = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Unauthorized party attempts to create dispute - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::dispute_escrow(
                env.clone(),
                escrow_id.clone(),
                unauthorized.clone(),
                Symbol::new(&env, "DISPUTE_REASON"),
                Vec::new(&env),
            );
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_creation_on_non_pending_escrow_fails() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        let release_time = env.ledger().timestamp() + 100;
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            release_time,
            Map::new(&env),
        );
        
        // Advance time and complete the escrow
        env.ledger().set_timestamp(release_time + 1);
        EscrowTrait::release_escrow(env.clone(), escrow_id.clone());
        
        // Attempt to dispute a completed escrow - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::dispute_escrow(
                env.clone(),
                escrow_id.clone(),
                sender.clone(),
                Symbol::new(&env, "DISPUTE_REASON"),
                Vec::new(&env),
            );
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_resolution_in_favor_of_challenger() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        
        // Verify escrow is disputed
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Disputed
        );
        
        // Resolve dispute in favor of challenger (sender gets refund)
        let resolved = EscrowTrait::resolve_dispute(env.clone(), dispute_id.clone(), true);
        assert!(resolved);
        
        // Verify escrow status is refunded
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Refunded
        );
        
        // Verify dispute is marked as resolved
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert!(dispute.resolved);
    }

    #[test]
    fn test_dispute_resolution_in_favor_of_receiver() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        
        // Resolve dispute in favor of receiver (receiver gets funds)
        let resolved = EscrowTrait::resolve_dispute(env.clone(), dispute_id.clone(), false);
        assert!(resolved);
        
        // Verify escrow status is completed
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Completed
        );
        
        // Verify dispute is marked as resolved
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert!(dispute.resolved);
    }

    #[test]
    fn test_cannot_resolve_already_resolved_dispute() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create dispute
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        
        // Resolve dispute
        EscrowTrait::resolve_dispute(env.clone(), dispute_id.clone(), true);
        
        // Attempt to resolve again - should fail
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            EscrowTrait::resolve_dispute(env.clone(), dispute_id.clone(), false);
        }));
        
        assert!(result.is_err());
    }

    #[test]
    fn test_dispute_state_transitions() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // State 1: Pending
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Pending
        );
        
        // State 2: Disputed
        let dispute_id = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "DISPUTE_REASON"),
            Vec::new(&env),
        );
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Disputed
        );
        
        // Verify dispute is not resolved yet
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id.clone());
        assert!(!dispute.resolved);
        
        // State 3: Resolved (Refunded)
        EscrowTrait::resolve_dispute(env.clone(), dispute_id.clone(), true);
        assert_eq!(
            EscrowTrait::get_escrow_status(env.clone(), escrow_id.clone()),
            EscrowStatus::Refunded
        );
        
        // Verify dispute is now resolved
        let dispute = EscrowTrait::get_dispute(env.clone(), dispute_id);
        assert!(dispute.resolved);
    }

    #[test]
    fn test_multiple_disputes_on_same_escrow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let sender = Address::generate(&env);
        let receiver = Address::generate(&env);
        
        // Initialize
        EscrowTrait::initialize(env.clone(), admin.clone());
        
        let token = Address::generate(&env);
        
        // Create escrow
        let escrow_id = EscrowTrait::create_escrow(
            env.clone(),
            sender.clone(),
            receiver.clone(),
            1000,
            token.clone(),
            env.ledger().timestamp() + 1000,
            Map::new(&env),
        );
        
        // Create first dispute
        let dispute_id_1 = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            sender.clone(),
            Symbol::new(&env, "FIRST_DISPUTE"),
            Vec::new(&env),
        );
        
        // Create second dispute (escrow is already disputed, but this should still work)
        let dispute_id_2 = EscrowTrait::dispute_escrow(
            env.clone(),
            escrow_id.clone(),
            receiver.clone(),
            Symbol::new(&env, "SECOND_DISPUTE"),
            Vec::new(&env),
        );
        
        // Verify both disputes exist
        let disputes = EscrowTrait::get_disputes_by_escrow(env.clone(), escrow_id.clone());
        assert_eq!(disputes.len(), 2);
        
        // Verify dispute IDs are different
        assert_ne!(dispute_id_1, dispute_id_2);
    }
}
