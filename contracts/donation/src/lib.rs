#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, Address, Env, Map, Vec, symbol_short, vec, Val,
};

#[derive(Clone, Debug)]
#[contracttype]
pub struct Donation {
    campaign_id: Address,
    donor: Address,
    amount: i128,
    timestamp: u64,
}

#[contract]
pub struct DonationContract;

#[contractimpl]
impl DonationContract {
    // Process donation
    pub fn donate(
        env: Env,
        campaign_id: Address,
        donor: Address,
        amount: i128,
        token: Address,
    ) -> Donation {
        donor.require_auth();

        // Call campaign contract to check status
        let campaign_client = env.invoke_contract::<bool>(&campaign_id, &symbol_short!("is_active"), Vec::<Val>::new(&env));
        if !campaign_client {
            panic!("Campaign not active");
        }

        // Transfer tokens
        let client = soroban_sdk::token::Client::new(&env, &token);
        client.transfer(&donor, &env.current_contract_address(), &amount);

        let donation = Donation {
            campaign_id: campaign_id.clone(),
            donor: donor.clone(),
            amount,
            timestamp: env.ledger().timestamp(),
        };

        // Store donation record
        let mut donations = env.storage().persistent().get(&campaign_id)
            .map(|m: Map<Address, Vec<Donation>>| m)
            .unwrap_or_else(|| Map::new(&env));
        
        let mut donor_donations = donations.get(donor.clone())
            .unwrap_or_else(|| vec![&env]);
        donor_donations.push_back(donation.clone());
        donations.set(donor, donor_donations);
        
        env.storage().persistent().set(&campaign_id, &donations);

        // Update campaign balance
        env.invoke_contract::<()>(
            &campaign_id,
            &symbol_short!("upd_bal"),
            Vec::<Val>::from_slice(&env, &[Val::from(amount)])
        );

        donation
    }

    // Get donor's donations for a campaign
    pub fn get_donations(
        env: Env,
        campaign_id: Address,
        donor: Address,
    ) -> Vec<Donation> {
        let donations: Map<Address, Vec<Donation>> = env.storage().persistent().get(&campaign_id).unwrap();
        donations.get(donor).unwrap_or_else(|| vec![&env])
    }

    // Get total donated amount for a campaign
    pub fn get_total_donated(
        env: Env,
        campaign_id: Address,
    ) -> i128 {
        let donations: Map<Address, Vec<Donation>> = env.storage().persistent().get(&campaign_id).unwrap();
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
    use soroban_sdk::testutils::{Address as _, Ledger};
    use soroban_sdk::token;

    #[test]
    fn test_donation() {
        let env = Env::default();
        let contract_id = env.register_contract(None, DonationContract);
        
        let donor = Address::random(&env);
        let campaign_id = BytesN::from_array(&env, &[0; 32]);
        let token = env.register_stellar_asset_contract(donor.clone());
        
        // Mock campaign contract
        env.register_contract_rust(
            &campaign_id,
            mockutil::make_mock_contract(|_, _, _| Ok(true.into())),
        );
        
        token::Client::new(&env, &token).mint(&donor, &1000);
        
        let donation = DonationContract::donate(
            &env,
            &contract_id,
            campaign_id.clone(),
            donor.clone(),
            100,
            token.clone(),
        );

        assert_eq!(donation.amount, 100);
        assert_eq!(donation.donor, donor);
        
        let total = DonationContract::get_total_donated(
            &env,
            &contract_id,
            campaign_id.clone(),
        );
        
        assert_eq!(total, 100);
    }
}
