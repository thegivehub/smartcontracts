#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, BytesN, Env,
    String,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracttype]
pub enum CampaignStatus {
    Draft,
    Active,
    Funded,
    Completed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Campaign {
    pub id: BytesN<32>,
    pub title: String,
    pub description: String,
    pub target_amount: i128,
    pub current_amount: i128,
    pub released_amount: i128,
    pub creator: Address,
    pub status: CampaignStatus,
    pub created_at: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(i32)]
pub enum CampaignError {
    CampaignNotFound = 1,
    InvalidTarget = 2,
    NotDraft = 3,
    NotActive = 4,
    Unauthorized = 5,
    InsufficientFunds = 6,
}

#[contract]
pub struct CampaignContract;

#[contractimpl]
impl CampaignContract {
    pub fn initialize(
        env: Env,
        creator: Address,
        campaign_id: BytesN<32>,
        title: String,
        description: String,
        target_amount: i128,
    ) -> Campaign {
        creator.require_auth();

        if target_amount <= 0 {
            panic_with_error!(&env, CampaignError::InvalidTarget);
        }

        let campaign = Campaign {
            id: campaign_id.clone(),
            title,
            description,
            target_amount,
            current_amount: 0,
            released_amount: 0,
            creator: creator.clone(),
            status: CampaignStatus::Draft,
            created_at: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&campaign_id, &campaign);
        campaign
    }

    pub fn activate(env: Env, creator: Address, campaign_id: BytesN<32>) -> Campaign {
        creator.require_auth();

        let mut campaign = Self::get_campaign(&env, &campaign_id);
        if campaign.creator != creator {
            panic_with_error!(&env, CampaignError::Unauthorized);
        }
        if campaign.status != CampaignStatus::Draft {
            panic_with_error!(&env, CampaignError::NotDraft);
        }

        campaign.status = CampaignStatus::Active;
        Self::save_campaign(&env, &campaign_id, &campaign);
        campaign
    }

    pub fn add_donation(env: Env, campaign_id: BytesN<32>, amount: i128) -> Campaign {
        let mut campaign = Self::get_campaign(&env, &campaign_id);
        if campaign.status != CampaignStatus::Active && campaign.status != CampaignStatus::Funded {
            panic_with_error!(&env, CampaignError::NotActive);
        }

        campaign.current_amount += amount;
        if campaign.current_amount >= campaign.target_amount
            && campaign.status == CampaignStatus::Active
        {
            campaign.status = CampaignStatus::Funded;
        }

        Self::save_campaign(&env, &campaign_id, &campaign);
        campaign
    }

    pub fn mark_milestone_completed(env: Env, campaign_id: BytesN<32>, amount: i128) -> Campaign {
        let mut campaign = Self::get_campaign(&env, &campaign_id);
        let available = campaign.current_amount - campaign.released_amount;
        if available < amount {
            panic_with_error!(&env, CampaignError::InsufficientFunds);
        }

        campaign.released_amount += amount;
        if campaign.released_amount >= campaign.target_amount {
            campaign.status = CampaignStatus::Completed;
        }

        Self::save_campaign(&env, &campaign_id, &campaign);
        campaign
    }

    pub fn cancel(env: Env, creator: Address, campaign_id: BytesN<32>) -> Campaign {
        creator.require_auth();

        let mut campaign = Self::get_campaign(&env, &campaign_id);
        if campaign.creator != creator {
            panic_with_error!(&env, CampaignError::Unauthorized);
        }

        campaign.status = CampaignStatus::Cancelled;
        Self::save_campaign(&env, &campaign_id, &campaign);
        campaign
    }

    pub fn get(env: Env, campaign_id: BytesN<32>) -> Campaign {
        Self::get_campaign(&env, &campaign_id)
    }

    pub fn status(env: Env, campaign_id: BytesN<32>) -> CampaignStatus {
        let campaign = Self::get_campaign(&env, &campaign_id);
        campaign.status
    }

    pub fn is_active(env: Env, campaign_id: BytesN<32>) -> bool {
        let campaign = Self::get_campaign(&env, &campaign_id);
        matches!(
            campaign.status,
            CampaignStatus::Active | CampaignStatus::Funded
        )
    }

    pub fn creator(env: Env, campaign_id: BytesN<32>) -> Address {
        let campaign = Self::get_campaign(&env, &campaign_id);
        campaign.creator
    }

    pub fn available_funds(env: Env, campaign_id: BytesN<32>) -> i128 {
        let campaign = Self::get_campaign(&env, &campaign_id);
        campaign.current_amount - campaign.released_amount
    }

    fn get_campaign(env: &Env, campaign_id: &BytesN<32>) -> Campaign {
        env.storage()
            .persistent()
            .get(campaign_id)
            .unwrap_or_else(|| panic_with_error!(env, CampaignError::CampaignNotFound))
    }

    fn save_campaign(env: &Env, campaign_id: &BytesN<32>, campaign: &Campaign) {
        env.storage().persistent().set(campaign_id, campaign);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn test_campaign_lifecycle() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, CampaignContract);
        let client = CampaignContractClient::new(&env, &contract_id);

        let creator = Address::generate(&env);
        let campaign_id = BytesN::from_array(&env, &[0; 32]);

        let campaign = client.initialize(
            &creator,
            &campaign_id,
            &String::from_str(&env, "Test Campaign"),
            &String::from_str(&env, "Test Description"),
            &1000,
        );
        assert_eq!(campaign.status, CampaignStatus::Draft);

        let active_campaign = client.activate(&creator, &campaign_id);
        assert_eq!(active_campaign.status, CampaignStatus::Active);

        let updated = client.add_donation(&campaign_id, &600);
        assert_eq!(updated.current_amount, 600);
        assert!(client.is_active(&campaign_id));

        let funded = client.add_donation(&campaign_id, &500);
        assert_eq!(funded.status, CampaignStatus::Funded);

        let completed = client.mark_milestone_completed(&campaign_id, &1100);
        assert_eq!(completed.status, CampaignStatus::Completed);
    }
}
