#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, vec, Address, Env, String, Symbol, Vec,
    Map, BytesN, ConversionError, TryFromVal, Val,
};

// Campaign status enum
#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub enum CampaignStatus {
    Draft,
    Active,
    Funded,
    Completed,
    Cancelled,
}

// Campaign struct
#[derive(Clone, Debug)]
#[contracttype]
pub struct Campaign {
    id: BytesN<32>,
    title: String,
    description: String,
    target_amount: i128,
    current_amount: i128,
    creator: Address,
    status: CampaignStatus,
    donors: Map<Address, i128>,
    created_at: u64,
}

#[contract]
pub struct CampaignContract;

#[contractimpl]
impl CampaignContract {
    // Initialize a new campaign
    pub fn initialize(
        env: Env,
        id: BytesN<32>,
        title: String,
        description: String,
        target_amount: i128,
        creator: Address,
    ) -> Campaign {
        creator.require_auth();

        if target_amount <= 0 {
            panic!("Target amount must be positive");
        }

        let campaign = Campaign {
            id,
            title,
            description,
            target_amount,
            current_amount: 0,
            creator,
            status: CampaignStatus::Draft,
            donors: Map::new(&env),
            created_at: env.ledger().timestamp(),
        };

        env.storage().set(&id, &campaign);
        campaign
    }

    // Activate campaign
    pub fn activate_campaign(env: Env, campaign_id: BytesN<32>) -> Campaign {
        let mut campaign: Campaign = env.storage().get(&campaign_id).unwrap();
        campaign.creator.require_auth();

        if campaign.status != CampaignStatus::Draft {
            panic!("Campaign must be in draft status");
        }

        campaign.status = CampaignStatus::Active;
        env.storage().set(&campaign_id, &campaign);
        campaign
    }

    // Update campaign status
    pub fn update_status(env: Env, campaign_id: BytesN<32>, new_status: CampaignStatus) -> Campaign {
        let mut campaign: Campaign = env.storage().get(&campaign_id).unwrap();
        campaign.creator.require_auth();

        campaign.status = new_status;
        env.storage().set(&campaign_id, &campaign);
        campaign
    }

    // View functions
    pub fn get_campaign(env: Env, campaign_id: BytesN<32>) -> Campaign {
        env.storage().get(&campaign_id).unwrap()
    }

    pub fn get_campaign_status(env: Env, campaign_id: BytesN<32>) -> CampaignStatus {
        let campaign: Campaign = env.storage().get(&campaign_id).unwrap();
        campaign.status
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::{Address as _, Ledger};

    #[test]
    fn test_campaign_lifecycle() {
        let env = Env::default();
        let contract_id = env.register_contract(None, CampaignContract);
        
        let creator = Address::random(&env);
        let campaign_id = BytesN::from_array(&env, &[0; 32]);
        let title = String::from_str(&env, "Test Campaign");
        let description = String::from_str(&env, "Test Description");
        let target_amount = 1000;

        let campaign = CampaignContract::initialize(
            &env,
            &contract_id,
            campaign_id.clone(),
            title,
            description,
            target_amount,
            creator.clone(),
        );

        assert_eq!(campaign.status, CampaignStatus::Draft);
        
        let active_campaign = CampaignContract::activate_campaign(
            &env,
            &contract_id,
            campaign_id.clone(),
        );

        assert_eq!(active_campaign.status, CampaignStatus::Active);
    }
}
