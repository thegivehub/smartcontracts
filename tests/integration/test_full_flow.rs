use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, String, symbol_short,
};

use givehub_campaign::{CampaignContract, Campaign, CampaignStatus};
use givehub_donation::DonationContract;
use givehub_verification::{VerificationContract, MilestoneStatus};

fn create_test_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    let donor = Address::random(&env);
    let creator = Address::random(&env);
    let verifier = Address::random(&env);
    (env, donor, creator, verifier)
}

#[test]
fn test_full_campaign_flow() {
    let (env, donor, creator, verifier) = create_test_env();
    
    // Deploy contracts
    let campaign_id = env.register_contract(None, CampaignContract);
    let donation_id = env.register_contract(None, DonationContract);
    let verification_id = env.register_contract(None, VerificationContract);

    // Create test token
    let token = env.register_stellar_asset_contract(creator.clone());
    
    // Initialize campaign
    let campaign = CampaignContract::initialize(
        &env,
        &campaign_id,
        BytesN::from_array(&env, &[0; 32]),
        String::from_str(&env, "Test Campaign"),
        String::from_str(&env, "Test Description"),
        1000,
        creator.clone(),
    );

    // Activate campaign
    let active_campaign = CampaignContract::activate_campaign(
        &env,
        &campaign_id,
        campaign.id.clone(),
    );
    assert_eq!(active_campaign.status, CampaignStatus::Active);

    // Create milestone
    let milestone = VerificationContract::create_milestone(
        &env,
        &verification_id,
        campaign.id.clone(),
        String::from_str(&env, "First Milestone"),
        500,
    );

    // Make donation
    token::Client::new(&env, &token).mint(&donor, &1000);
    let donation = DonationContract::donate(
        &env,
        &donation_id,
        campaign.id.clone(),
        donor.clone(),
        600,
        token.clone(),
    );

    // Verify milestone
    let docs = vec![
        &env,
        String::from_str(&env, "verification.pdf"),
        String::from_str(&env, "photos.zip"),
    ];
    
    let verified_milestone = VerificationContract::verify_milestone(
        &env,
        &verification_id,
        campaign.id.clone(),
        0,
        verifier.clone(),
        docs,
    );
    assert_eq!(verified_milestone.status, MilestoneStatus::Verified);

    // Complete milestone
    let completed_milestone = VerificationContract::complete_milestone(
        &env,
        &verification_id,
        campaign.id.clone(),
        0,
        token.clone(),
    );
    assert_eq!(completed_milestone.status, MilestoneStatus::Completed);

    // Check final campaign status
    let final_campaign = CampaignContract::get_campaign(
        &env,
        &campaign_id,
        campaign.id.clone(),
    );
    
    assert!(final_campaign.current_amount >= 600);
    let total_donated = DonationContract::get_total_donated(
        &env,
        &donation_id,
        campaign.id.clone(),
    );
    assert_eq!(total_donated, 600);
}
