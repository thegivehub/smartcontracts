#![no_std]
use givehub_campaign::CampaignContractClient;
use soroban_sdk::{
    auth::{ContractContext, InvokerContractAuthEntry, SubContractInvocation},
    contract, contracterror, contractimpl, contracttype, panic_with_error, vec, Address, BytesN,
    Env, IntoVal, Map, String, Symbol, Vec,
};

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Donation {
    pub campaign_id: BytesN<32>,
    pub donor: Address,
    pub amount: i128,
    pub timestamp: u64,
    pub note: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
#[repr(i32)]
pub enum DonationError {
    CampaignInactive = 1,
    InvalidAmount = 2,
    Unauthorized = 3,
}

#[contract]
pub struct DonationContract;

#[contractimpl]
impl DonationContract {
    pub fn donate(
        env: Env,
        donor: Address,
        campaign_contract: Address,
        campaign_id: BytesN<32>,
        amount: i128,
        note: Option<String>,
    ) -> Donation {
        donor.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, DonationError::InvalidAmount);
        }

        let campaign_client = CampaignContractClient::new(&env, &campaign_contract);
        let campaign = campaign_client.get(&campaign_id);
        if campaign.donation_contract != Some(env.current_contract_address()) {
            panic_with_error!(&env, DonationError::Unauthorized);
        }
        if !matches!(
            campaign.status,
            givehub_campaign::CampaignStatus::Active | givehub_campaign::CampaignStatus::Funded
        ) {
            panic_with_error!(&env, DonationError::CampaignInactive);
        }

        let auth_entry = InvokerContractAuthEntry::Contract(SubContractInvocation {
            context: ContractContext {
                contract: campaign_contract.clone(),
                fn_name: Symbol::new(&env, "add_donation"),
                args: vec![
                    &env,
                    campaign_id.clone().into_val(&env),
                    amount.into_val(&env),
                ],
            },
            sub_invocations: vec![&env],
        });
        env.authorize_as_current_contract(vec![&env, auth_entry]);

        let donation = Donation {
            campaign_id: campaign_id.clone(),
            donor: donor.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
            note,
        };

        let mut donations: Map<Address, Vec<Donation>> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| Map::new(&env));

        let mut donor_donations = donations.get(donor.clone()).unwrap_or_else(|| vec![&env]);
        donor_donations.push_back(donation.clone());
        donations.set(donor.clone(), donor_donations);
        env.storage().persistent().set(&campaign_id, &donations);

        campaign_client.add_donation(&campaign_id, &amount);

        donation
    }

    pub fn get_donations(env: Env, campaign_id: BytesN<32>, donor: Address) -> Vec<Donation> {
        let donations: Map<Address, Vec<Donation>> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| Map::new(&env));
        donations.get(donor).unwrap_or_else(|| vec![&env])
    }

    pub fn get_total_donated(env: Env, campaign_id: BytesN<32>) -> i128 {
        let donations: Map<Address, Vec<Donation>> = env
            .storage()
            .persistent()
            .get(&campaign_id)
            .unwrap_or_else(|| Map::new(&env));

        let mut total = 0;
        for donor_donations in donations.values() {
            for donation in donor_donations.iter() {
                total += donation.amount;
            }
        }
        total
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use givehub_campaign::{CampaignContract, CampaignStatus};
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn test_donation_flow() {
        let env = Env::default();
        env.mock_all_auths();
        let campaign_addr = env.register_contract(None, CampaignContract);
        let donation_addr = env.register_contract(None, DonationContract);

        let campaign_client = CampaignContractClient::new(&env, &campaign_addr);
        let donation_client = DonationContractClient::new(&env, &donation_addr);

        let creator = Address::generate(&env);
        let donor = Address::generate(&env);
        let campaign_id = BytesN::from_array(&env, &[0; 32]);

        let _campaign = campaign_client.initialize(
            &creator,
            &campaign_id,
            &String::from_str(&env, "Save the Rainforest"),
            &String::from_str(&env, "Plant trees"),
            &500,
        );

        campaign_client.set_authorized_contracts(
            &creator,
            &campaign_id,
            &Some(donation_addr.clone()),
            &None,
        );

        let activated = campaign_client.activate(&creator, &campaign_id);
        assert_eq!(activated.status, CampaignStatus::Active);

        let donation = donation_client.donate(&donor, &campaign_addr, &campaign_id, &250, &None);

        assert_eq!(donation.amount, 250);
        assert_eq!(donation.donor, donor);

        let total = donation_client.get_total_donated(&campaign_id);
        assert_eq!(total, 250);
    }
}
