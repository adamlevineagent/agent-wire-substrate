use agent_wire_contracts::{
    ContractVerb, ContractWrap, HandlePathDto, PrivateGraphRegistrationDto,
};
use agent_wire_foundation::{GraphSlug, HandlePath, PrivateGraphRegistration};

#[test]
fn contract_wrap_records_wrap_verb() {
    let wrapped = ContractWrap::wrap(HandlePathDto {
        handle: "playful".to_owned(),
        wire_day: 183,
        graph_slug: Some("kitty".to_owned()),
        sequence: 7,
    });

    assert_eq!(wrapped.verb, ContractVerb::Wrap);
    let runtime: HandlePath = wrapped.payload.into();
    assert_eq!(runtime.to_string(), "playful/183/kitty/7");
}

#[test]
fn foundation_converts_private_graph_registration_without_reexport() {
    let dto = PrivateGraphRegistrationDto {
        slug: "kitty".to_owned(),
        operator_handle: "playful".to_owned(),
        endpoint: "https://kitty-wire.example".to_owned(),
        annual_renewal_credits: 5_000,
        grace_days: 45,
        competitive_bidding: false,
        signature: "sig".to_owned(),
    };

    let runtime = PrivateGraphRegistration::from(dto);
    assert_eq!(runtime.slug, GraphSlug::new("kitty"));
    assert_eq!(runtime.grace_days, 45);
    assert!(!runtime.competitive_bidding);
}
