use soroban_sdk::{contract, contractimpl, contracttype, Address, Env, Symbol, Vec, Map, BytesN};

#[contracttype]
pub enum EscrowStatus {
    Pending,
    Completed,
    Refunded,
    Disputed,
}

#[contracttype]
pub struct Escrow {
    pub id: BytesN<32>,
    pub sender: Address,
    pub receiver: Address,
    pub amount: i128,
    pub token: Address,
    pub status: EscrowStatus,
    pub release_time: u64,
    pub created_at: u64,
    pub metadata: Map<Symbol, Vec<u8>>,
}

#[contracttype]
pub struct Dispute {
    pub escrow_id: BytesN<32>,
    pub challenger: Address,
    pub reason: Symbol,
    pub evidence: Vec<u8>,
    pub created_at: u64,
    pub resolved: bool,
}

pub struct EscrowContract;

#[contract]
pub trait EscrowTrait {
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

        let escrow_id = env.crypto().sha256(&(
            sender.clone(),
            receiver.clone(),
            amount,
            env.ledger().timestamp(),
        )
            .into());

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

        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        escrows.set(escrow_id.clone(), escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        let user_escrows_key = Symbol::new(&env, &format!("USER_ESCROWS_{}", sender));
        let mut user_escrows = env.storage().persistent().get::<_, Vec<BytesN<32>>>(&user_escrows_key)
            .unwrap_or_else(|| Vec::new(&env));
        user_escrows.push_back(escrow_id.clone());
        env.storage().persistent().set(&user_escrows_key, &user_escrows);

        escrow_id
    }

    fn release_escrow(env: Env, escrow_id: BytesN<32>) -> bool {
        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut escrow = escrows.get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        if escrow.status != EscrowStatus::Pending {
            panic!("Escrow is not in pending status");
        }

        if env.ledger().timestamp() < escrow.release_time {
            panic!("Escrow is still time-locked");
        }

        escrow.receiver.require_auth();

        escrow.status = EscrowStatus::Completed;
        escrows.set(escrow_id.clone(), escrow.clone());
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    fn refund_escrow(env: Env, escrow_id: BytesN<32>) -> bool {
        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut escrow = escrows.get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        if escrow.status != EscrowStatus::Pending {
            panic!("Escrow is not in pending status");
        }

        escrow.sender.require_auth();

        escrow.status = EscrowStatus::Refunded;
        escrows.set(escrow_id.clone(), escrow.clone());
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    fn dispute_escrow(
        env: Env,
        escrow_id: BytesN<32>,
        challenger: Address,
        reason: Symbol,
        evidence: Vec<u8>,
    ) -> bool {
        challenger.require_auth();

        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut escrow = escrows.get(escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        if escrow.status != EscrowStatus::Pending {
            panic!("Escrow is not in pending status");
        }

        escrow.status = EscrowStatus::Disputed;
        escrows.set(escrow_id.clone(), escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        let dispute_id = env.crypto().sha256(&(
            escrow_id.clone(),
            challenger.clone(),
            env.ledger().timestamp(),
        ).into());

        let dispute = Dispute {
            escrow_id: escrow_id.clone(),
            challenger,
            reason,
            evidence,
            created_at: env.ledger().timestamp(),
            resolved: false,
        };

        let disputes_key = Symbol::new(&env, "DISPUTES");
        let mut disputes = env.storage().persistent().get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));
        disputes.set(dispute_id.clone(), dispute);
        env.storage().persistent().set(&disputes_key, &disputes);

        true
    }

    fn resolve_dispute(
        env: Env,
        dispute_id: BytesN<32>,
        in_favor_of_challenger: bool,
    ) -> bool {
        let admin_key = Symbol::new(&env, "ADMIN");
        let admin = env.storage().persistent().get::<_, Address>(&admin_key)
            .unwrap_or_else(|| panic!("Admin not set"));
        admin.require_auth();

        let disputes_key = Symbol::new(&env, "DISPUTES");
        let mut disputes = env.storage().persistent().get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut dispute = disputes.get(dispute_id.clone())
            .unwrap_or_else(|| panic!("Dispute not found"));

        if dispute.resolved {
            panic!("Dispute already resolved");
        }

        dispute.resolved = true;
        disputes.set(dispute_id.clone(), dispute.clone());
        env.storage().persistent().set(&disputes_key, &disputes);

        let escrow_key = Symbol::new(&env, "ESCROW");
        let mut escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut escrow = escrows.get(dispute.escrow_id.clone())
            .unwrap_or_else(|| panic!("Escrow not found"));

        escrow.status = if in_favor_of_challenger {
            EscrowStatus::Refunded
        } else {
            EscrowStatus::Completed
        };

        escrows.set(dispute.escrow_id.clone(), escrow);
        env.storage().persistent().set(&escrow_key, &escrows);

        true
    }

    fn get_escrow(env: Env, escrow_id: BytesN<32>) -> Escrow {
        let escrow_key = Symbol::new(&env, "ESCROW");
        let escrows = env.storage().persistent().get::<_, Map<BytesN<32>, Escrow>>(&escrow_key)
            .unwrap_or_else(|| Map::new(&env));
        
        escrows.get(escrow_id)
            .unwrap_or_else(|| panic!("Escrow not found"))
    }

    fn get_escrow_status(env: Env, escrow_id: BytesN<32>) -> EscrowStatus {
        Self::get_escrow(env, escrow_id).status
    }

    fn get_dispute(env: Env, dispute_id: BytesN<32>) -> Dispute {
        let disputes_key = Symbol::new(&env, "DISPUTES");
        let disputes = env.storage().persistent().get::<_, Map<BytesN<32>, Dispute>>(&disputes_key)
            .unwrap_or_else(|| Map::new(&env));
        
        disputes.get(dispute_id)
            .unwrap_or_else(|| panic!("Dispute not found"))
    }

    fn get_user_escrows(env: Env, user: Address) -> Vec<BytesN<32>> {
        let user_escrows_key = Symbol::new(&env, &format!("USER_ESCROWS_{}", user));
        env.storage().persistent().get::<_, Vec<BytesN<32>>>(&user_escrows_key)
            .unwrap_or_else(|| Vec::new(&env))
    }
}
