use async_trait::async_trait;
use std::sync::Arc;
use xaynet_core::{
    common::RoundParameters,
    crypto::{ByteObject, Signature, SigningKeyPair},
    mask::Model,
    ParticipantSecretKey,
};
use xaynet_sdk::{
    client::Client as ApiClient,
    settings::PetSettings,
    ModelStore,
    Notify,
    StateMachine,
};

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub enum ClientType {
    Awaiting,
    Sum,
    Update,
}

pub fn generate_client(
    r#type: &ClientType,
    round_params: &RoundParameters,
    api_client: ApiClient<reqwest::Client>,
    model_store: LocalModel,
    notify: Notifier,
) -> anyhow::Result<StateMachine> {
    loop {
        let (client_type, key_pair) = new_client(&round_params);
        if client_type == *r#type {
            let new_client =
                StateMachine::new(PetSettings::new(key_pair), api_client, model_store, notify);
            break Ok(new_client);
        }
    }
}

fn new_client(round_params: &RoundParameters) -> (ClientType, SigningKeyPair) {
    let key_pair = SigningKeyPair::generate();
    let role = determine_role(
        key_pair.secret.clone(),
        round_params.seed.as_slice(),
        round_params.sum,
        round_params.update,
    );
    (role, key_pair)
}

pub fn determine_role(
    secret_key: ParticipantSecretKey,
    round_seed: &[u8],
    round_sum: f64,
    round_update: f64,
) -> ClientType {
    let (sum_signature, update_signature) = compute_signatures(secret_key, round_seed);
    if sum_signature.is_eligible(round_sum) {
        ClientType::Sum
    } else if update_signature.is_eligible(round_update) {
        ClientType::Update
    } else {
        ClientType::Awaiting
    }
}

/// Compute the sum and update signatures for the given round seed.
fn compute_signatures(
    secret_key: ParticipantSecretKey,
    round_seed: &[u8],
) -> (Signature, Signature) {
    (
        secret_key.sign_detached(&[round_seed, b"sum"].concat()),
        secret_key.sign_detached(&[round_seed, b"update"].concat()),
    )
}

#[derive(Clone)]
pub struct Notifier;

impl Notify for Notifier {
    fn new_round(&mut self) {}
    fn sum(&mut self) {}
    fn update(&mut self) {}
    fn idle(&mut self) {}
    fn load_model(&mut self) {}
}

pub struct LocalModel(pub Arc<Model>);

#[async_trait]
impl ModelStore for LocalModel {
    type Model = Arc<Model>;
    type Error = std::convert::Infallible;

    async fn load_model(&mut self) -> Result<Option<Self::Model>, Self::Error> {
        Ok(Some(self.0.clone()))
    }
}
