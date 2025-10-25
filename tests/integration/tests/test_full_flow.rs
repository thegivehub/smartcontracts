use givehub_campaign::{CampaignContract, CampaignContractClient, CampaignStatus};
use givehub_donation::{DonationContract, DonationContractClient};
use givehub_verification::{MilestoneStatus, VerificationContract, VerificationContractClient};
use soroban_sdk::{testutils::Address as _, vec, Address, BytesN, Env, String};

#[test]
fn test_full_campaign_flow() {
    let env = Env::default();
    env.mock_all_auths();
    let donor = Address::generate(&env);
    let creator = Address::generate(&env);
    let verifier = Address::generate(&env);

    let campaign_addr = env.register_contract(None, CampaignContract);
    let donation_addr = env.register_contract(None, DonationContract);
    let verification_addr = env.register_contract(None, VerificationContract);

    let campaign_client = CampaignContractClient::new(&env, &campaign_addr);
    let donation_client = DonationContractClient::new(&env, &donation_addr);
    let verification_client = VerificationContractClient::new(&env, &verification_addr);

    let campaign_id = BytesN::from_array(&env, &[0; 32]);

    campaign_client.initialize(
        &creator,
        &campaign_id,
        &String::from_str(&env, "Test Campaign"),
        &String::from_str(&env, "Test Description"),
        &1000,
    );

    campaign_client.set_authorized_contracts(
        &creator,
        &campaign_id,
        &Some(donation_addr.clone()),
        &Some(verification_addr.clone()),
    );

    verification_client.configure_campaign(&creator, &campaign_addr, &campaign_id, &verifier);

    env.mock_all_auths();
    let active_campaign = campaign_client.activate(&creator, &campaign_id);
    assert_eq!(active_campaign.status, CampaignStatus::Active);

    let milestone = verification_client.create_milestone(
        &creator,
        &campaign_id,
        &String::from_str(&env, "First Milestone"),
        &500,
    );
    assert_eq!(milestone.status, MilestoneStatus::Pending);

    let donation = donation_client.donate(&donor, &campaign_addr, &campaign_id, &600, &None);
    assert_eq!(donation.amount, 600);

    let docs = vec![
        &env,
        String::from_str(&env, "verification.pdf"),
        String::from_str(&env, "photos.zip"),
    ];
    let verified = verification_client.verify_milestone(&verifier, &campaign_id, &0, &docs);
    assert_eq!(verified.status, MilestoneStatus::Verified);

    let completed = verification_client.complete_milestone(&verifier, &campaign_id, &0);
    assert_eq!(completed.status, MilestoneStatus::Completed);

    let final_campaign = campaign_client.get(&campaign_id);
    assert_eq!(final_campaign.current_amount, 600);
    assert_eq!(campaign_client.available_funds(&campaign_id), 100);

    let total_donated = donation_client.get_total_donated(&campaign_id);
    assert_eq!(total_donated, 600);
}
