#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Env, String, Vec, symbol_short, Val,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum MilestoneStatus {
    Pending,
    Verified,
    Completed,
    Failed,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct Milestone {
    description: String,
    amount: i128,
    status: MilestoneStatus,
    verification_docs: Vec<String>,
    verified_by: Option<Address>,
    completed_at: Option<u64>,
}

#[contract]
pub struct VerificationContract;

#[contractimpl]
impl VerificationContract {
    // Initialize milestone
    pub fn create_milestone(
        env: Env,
        campaign_id: Address,
        description: String,
        amount: i128,
    ) -> Milestone {
        let milestone = Milestone {
            description,
            amount,
            status: MilestoneStatus::Pending,
            verification_docs: vec![&env],
            verified_by: None,
            completed_at: None,
        };

        let mut milestones = env.storage().persistent().get(&campaign_id)
            .map(|m: Vec<Milestone>| m)
            .unwrap_or_else(|| vec![&env]);
        
        milestones.push_back(milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);

        milestone
    }

    // Submit milestone verification
    pub fn verify_milestone(
        env: Env,
        campaign_id: Address,
        milestone_index: u32,
        verifier: Address,
        docs: Vec<String>,
    ) -> Milestone {
        verifier.require_auth();

        let mut milestones: Vec<Milestone> = env.storage().persistent().get(&campaign_id).unwrap();
        let mut milestone = milestones.get(milestone_index).unwrap();
        
        if milestone.status != MilestoneStatus::Pending {
            panic!("Milestone not pending");
        }

        milestone.status = MilestoneStatus::Verified;
        milestone.verified_by = Some(verifier);
        milestone.verification_docs = docs;
        
        milestones.set(milestone_index, milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);

        milestone
    }

    // Release milestone funds
    pub fn complete_milestone(
        env: Env,
        campaign_id: Address,
        milestone_index: u32,
        token: Address,
    ) -> Milestone {
        let mut milestones: Vec<Milestone> = env.storage().persistent().get(&campaign_id).unwrap();
        let mut milestone = milestones.get(milestone_index).unwrap();

        if milestone.status != MilestoneStatus::Verified {
            panic!("Milestone not verified");
        }

        // Get campaign creator address
        let creator: Address = env.invoke_contract(
            &campaign_id,
            &symbol_short!("creator"),
            Vec::<Val>::new(&env),
        );

        // Transfer tokens to creator
        let client = soroban_sdk::token::Client::new(&env, &token);
        client.transfer(
            &env.current_contract_address(),
            &creator,
            &milestone.amount,
        );

        milestone.status = MilestoneStatus::Completed;
        milestone.completed_at = Some(env.ledger().timestamp());
        
        milestones.set(milestone_index, milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);

        milestone
    }

    // View functions
    pub fn get_milestones(env: Env, campaign_id: Address) -> Vec<Milestone> {
        env.storage().persistent().get(&campaign_id)
            .map(|m: Vec<Milestone>| m)
            .unwrap_or_else(|| vec![&env])
    }

    pub fn get_milestone(env: Env, campaign_id: Address, index: u32) -> Milestone {
        let milestones: Vec<Milestone> = env.storage().persistent().get(&campaign_id).unwrap();
        milestones.get(index).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_milestone_lifecycle() {
        let env = Env::default();
        let contract_id = env.register_contract(None, VerificationContract);
        
        let campaign_id = BytesN::from_array(&env, &[0; 32]);
        let verifier = Address::random(&env);
        
        let milestone = VerificationContract::create_milestone(
            &env,
            &contract_id,
            campaign_id.clone(),
            String::from_str(&env, "Test Milestone"),
            1000,
        );

        assert_eq!(milestone.status, MilestoneStatus::Pending);
        
        let docs = vec![&env, String::from_str(&env, "verification.pdf")];
        
        let verified_milestone = VerificationContract::verify_milestone(
            &env,
            &contract_id,
            campaign_id.clone(),
            0,
            verifier.clone(),
            docs,
        );

        assert_eq!(verified_milestone.status, MilestoneStatus::Verified);
        assert_eq!(verified_milestone.verified_by, Some(verifier));
        
        // Mock campaign creator for testing
        let creator = Address::random(&env);
        env.register_contract_rust(
            &campaign_id,
            mockutil::make_mock_contract(move |_, _, _| Ok(creator.into())),
        );
        
        let token = env.register_stellar_asset_contract(creator.clone());
        
        let completed_milestone = VerificationContract::complete_milestone(
            &env,
            &contract_id,
            campaign_id.clone(),
            0,
            token.clone(),
        );

        assert_eq!(completed_milestone.status, MilestoneStatus::Completed);
        assert!(completed_milestone.completed_at.is_some());
    }
}
