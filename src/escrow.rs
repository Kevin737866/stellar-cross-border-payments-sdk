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

const ADMIN_KEY:     &str = "ADMIN";
const INIT_KEY:      &str = "INITIALIZED";
const ESCROW_KEY:    &str = "ESCROW";
const DISPUTES_KEY:  &str = "DISPUTES";
const EVIDENCE_KEY:  &str = "DISPUTE_EVIDENCE";

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
    ) -> bool;

    fn resolve_dispute(
        env: Env,
        dispute_id: BytesN<32>,
        in_favor_of_challenger: bool,
    ) -> bool;

    fn get_escrow(env: Env, escrow_id: BytesN<32>) -> Escrow;

    fn get_escrow_status(env: Env, escrow_id: BytesN<32>) -> EscrowStatus;

    fn get_dispute(env: Env, dispute_id: BytesN<32>) -> Dispute;

    fn get_user_escrows(env: Env, user: Address) -> Vec<BytesN<32>>;
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

    fn dispute_escrow(
        env: Env,
        escrow_id: BytesN<32>,
        challenger: Address,
        reason: Symbol,
        evidence: Vec<u8>,
    ) -> bool {
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
            challenger,
            reason,
            evidence,
            created_at: env.ledger().timestamp(),
            resolved: false,
        };

        let disputes_key = Symbol::new(&env, DISPUTES_KEY);
        let mut disputes = env
            .storage()
            .persistent()
            .get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));
        disputes.set(dispute_id, dispute);
        env.storage().persistent().set(&disputes_key, &disputes);

        true
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

    fn get_user_escrows(env: Env, user: Address) -> Vec<BytesN<32>> {
        let user_key = (Symbol::new(&env, "USER_ESCROWS"), user);
        env.storage()
            .persistent()
            .get::<_, Vec<BytesN<32>>>(&user_key)
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
}
