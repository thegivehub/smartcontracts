#![no_std]
use givehub_campaign::CampaignContractClient;
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contracterror, contractimpl, contracttype, panic_with_error, symbol_short, vec,
    Address, BytesN, Env, IntoVal, String, Symbol, Vec,
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

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct VerificationConfig {
    pub campaign_contract: Address,
    pub owner: Address,
    pub verifier: Address,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(i32)]
pub enum VerificationError {
    InvalidAmount = 1,
    MilestoneNotFound = 2,
    MilestoneNotPending = 3,
    MilestoneNotVerified = 4,
    Unauthorized = 5,
    NotConfigured = 6,
}

#[contract]
pub struct VerificationContract;

#[contractimpl]
impl VerificationContract {
    pub fn configure_campaign(
        env: Env,
        owner: Address,
        campaign_contract: Address,
        campaign_id: BytesN<32>,
        verifier: Address,
    ) -> VerificationConfig {
        owner.require_auth();

        let campaign_client = CampaignContractClient::new(&env, &campaign_contract);
        let campaign_owner = campaign_client.creator(&campaign_id);
        if campaign_owner != owner {
            panic_with_error!(&env, VerificationError::Unauthorized);
        }

        let config = VerificationConfig {
            campaign_contract,
            owner,
            verifier,
        };

        let key = (symbol_short!("cfg"), campaign_id.clone());
        env.storage().persistent().set(&key, &config);
        config
    }

    pub fn create_milestone(
        env: Env,
        owner: Address,
        campaign_id: BytesN<32>,
        description: String,
        amount: i128,
    ) -> Milestone {
        if amount <= 0 {
            panic_with_error!(&env, VerificationError::InvalidAmount);
        }

        owner.require_auth();
        let config = Self::read_config(&env, &campaign_id);
        if config.owner != owner {
            panic_with_error!(&env, VerificationError::Unauthorized);
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

        let config = Self::read_config(&env, &campaign_id);
        if config.verifier != verifier {
            panic_with_error!(&env, VerificationError::Unauthorized);
        }

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
        verifier: Address,
        campaign_id: BytesN<32>,
        milestone_index: u32,
    ) -> Milestone {
        verifier.require_auth();

        let config = Self::read_config(&env, &campaign_id);
        if config.verifier != verifier {
            panic_with_error!(&env, VerificationError::Unauthorized);
        }

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

        let auth_entry = InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract: config.campaign_contract.clone(),
                fn_name: Symbol::new(&env, "mark_milestone_completed"),
                args: vec![
                    &env,
                    campaign_id.clone().into_val(&env),
                    milestone.amount.into_val(&env),
                ],
            },
            sub_invocations: vec![&env],
        });
        env.authorize_as_current_contract(vec![&env, auth_entry]);

        let campaign_client = CampaignContractClient::new(&env, &config.campaign_contract);
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

    pub fn get_config(env: Env, campaign_id: BytesN<32>) -> VerificationConfig {
        Self::read_config(&env, &campaign_id)
    }

    fn read_config(env: &Env, campaign_id: &BytesN<32>) -> VerificationConfig {
        let key = (symbol_short!("cfg"), campaign_id.clone());
        env.storage()
            .persistent()
            .get(&key)
            .unwrap_or_else(|| panic_with_error!(env, VerificationError::NotConfigured))
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
        let donation_contract = Address::generate(&env);
        let campaign_id = BytesN::from_array(&env, &[1; 32]);

        campaign_client.initialize(
            &creator,
            &campaign_id,
            &String::from_str(&env, "Build wells"),
            &String::from_str(&env, "Provide clean water"),
            &1000,
        );

        campaign_client.set_authorized_contracts(
            &creator,
            &campaign_id,
            &Some(donation_contract.clone()),
            &Some(verification_addr.clone()),
        );

        verification_client.configure_campaign(&creator, &campaign_addr, &campaign_id, &verifier);

        campaign_client.activate(&creator, &campaign_id);
        campaign_client.add_donation(&campaign_id, &500);

        let milestone = verification_client.create_milestone(
            &creator,
            &campaign_id,
            &String::from_str(&env, "Drill first well"),
            &400,
        );

        assert_eq!(milestone.status, MilestoneStatus::Pending);

        let docs = vec![&env, String::from_str(&env, "report.pdf")];
        let verified = verification_client.verify_milestone(&verifier, &campaign_id, &0, &docs);
        assert_eq!(verified.status, MilestoneStatus::Verified);

        let completed = verification_client.complete_milestone(&verifier, &campaign_id, &0);
        assert_eq!(completed.status, MilestoneStatus::Completed);
        assert!(completed.completed_at.is_some());
    }
}
