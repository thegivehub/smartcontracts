#![no_std]
use givehub_campaign::CampaignContractClient;
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, vec, Address, BytesN,
    Env, String, Vec,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum MilestoneStatus {
    Pending,
    Verified,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Milestone {
    pub description: String,
    pub amount: i128,
    pub status: MilestoneStatus,
    pub verification_docs: Vec<String>,
    pub verified_by: Option<Address>,
    pub completed_at: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(i32)]
pub enum VerificationError {
    InvalidAmount = 1,
    MilestoneNotFound = 2,
    MilestoneNotPending = 3,
    MilestoneNotVerified = 4,
}

#[contract]
pub struct VerificationContract;

#[contractimpl]
impl VerificationContract {
    pub fn create_milestone(
        env: Env,
        campaign_id: BytesN<32>,
        description: String,
        amount: i128,
    ) -> Milestone {
        if amount <= 0 {
            panic_with_error!(&env, VerificationError::InvalidAmount);
        }

        let milestone = Milestone {
            description,
            amount,
            status: MilestoneStatus::Pending,
            verification_docs: vec![&env],
            verified_by: None,
            completed_at: None,
        };

        let mut milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| vec![&env]);

        milestones.push_back(milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);
        milestone
    }

    pub fn verify_milestone(
        env: Env,
        verifier: Address,
        campaign_id: BytesN<32>,
        milestone_index: u32,
        docs: Vec<String>,
    ) -> Milestone {
        verifier.require_auth();

        let mut milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound));

        let mut milestone = milestones
            .get(milestone_index)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound));

        if milestone.status != MilestoneStatus::Pending {
            panic_with_error!(&env, VerificationError::MilestoneNotPending);
        }

        milestone.status = MilestoneStatus::Verified;
        milestone.verified_by = Some(verifier);
        milestone.verification_docs = docs;

        milestones.set(milestone_index, milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);
        milestone
    }

    pub fn complete_milestone(
        env: Env,
        campaign_contract: Address,
        campaign_id: BytesN<32>,
        milestone_index: u32,
    ) -> Milestone {
        let mut milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound));

        let mut milestone = milestones
            .get(milestone_index)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound));

        if milestone.status != MilestoneStatus::Verified {
            panic_with_error!(&env, VerificationError::MilestoneNotVerified);
        }

        let campaign_client = CampaignContractClient::new(&env, &campaign_contract);
        campaign_client.mark_milestone_completed(&campaign_id, &milestone.amount);

        milestone.status = MilestoneStatus::Completed;
        milestone.completed_at = Some(env.ledger().timestamp());

        milestones.set(milestone_index, milestone.clone());
        env.storage().persistent().set(&campaign_id, &milestones);
        milestone
    }

    pub fn get_milestones(env: Env, campaign_id: BytesN<32>) -> Vec<Milestone> {
        env.storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| vec![&env])
    }

    pub fn get_milestone(env: Env, campaign_id: BytesN<32>, index: u32) -> Milestone {
        let milestones: Vec<Milestone> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound));
        milestones
            .get(index)
            .unwrap_or_else(|| panic_with_error!(&env, VerificationError::MilestoneNotFound))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use givehub_campaign::{CampaignContract, CampaignContractClient};
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn test_milestone_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        let campaign_addr = env.register_contract(None, CampaignContract);
        let verification_addr = env.register_contract(None, VerificationContract);

        let campaign_client = CampaignContractClient::new(&env, &campaign_addr);
        let verification_client = VerificationContractClient::new(&env, &verification_addr);

        let creator = Address::generate(&env);
        let verifier = Address::generate(&env);
        let campaign_id = BytesN::from_array(&env, &[1; 32]);

        campaign_client.initialize(
            &creator,
            &campaign_id,
            &String::from_str(&env, "Build wells"),
            &String::from_str(&env, "Provide clean water"),
            &1000,
        );

        campaign_client.activate(&creator, &campaign_id);
        campaign_client.add_donation(&campaign_id, &500);

        let milestone = verification_client.create_milestone(
            &campaign_id,
            &String::from_str(&env, "Drill first well"),
            &400,
        );

        assert_eq!(milestone.status, MilestoneStatus::Pending);

        let docs = vec![&env, String::from_str(&env, "report.pdf")];
        let verified = verification_client.verify_milestone(&verifier, &campaign_id, &0, &docs);
        assert_eq!(verified.status, MilestoneStatus::Verified);

        let completed = verification_client.complete_milestone(&campaign_addr, &campaign_id, &0);
        assert_eq!(completed.status, MilestoneStatus::Completed);
        assert!(completed.completed_at.is_some());
    }
}
