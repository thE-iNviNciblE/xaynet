use super::{
    runner::ClientRunner,
    utils::{generate_client, ClientType, LocalModel, Notifier},
};
use crate::utils::concurrent_futures::ConcurrentFutures;
use futures::Future;
use std::{pin::Pin, sync::Arc};
use xaynet_core::mask::{FromPrimitives, Model};
use xaynet_sdk::{client::Client as ApiClient, StateMachine, TransitionOutcome, XaynetClient};

pub struct TestClientBuilderSettings {
    number_of_sum: u64,
    number_of_update: u64,
    model_length: usize,
}

impl TestClientBuilderSettings {
    pub fn from_coordinator_settings(
        coordinator_settings: xaynet_server::settings::Settings,
    ) -> Self {
        Self {
            number_of_sum: coordinator_settings.pet.min_sum_count,
            number_of_update: coordinator_settings.pet.min_update_count,
            model_length: coordinator_settings.model.length,
        }
    }
}

pub struct TestClientBuilder {
    settings: TestClientBuilderSettings,
    api_client: ApiClient<reqwest::Client>,
    model: Arc<Model>,
    notify: Notifier,
}

impl TestClientBuilder {
    pub fn new(
        settings: TestClientBuilderSettings,
        api_client: ApiClient<reqwest::Client>,
    ) -> Self {
        let model = Model::from_primitives(vec![1; settings.model_length].into_iter()).unwrap();
        Self {
            api_client,
            settings,
            model: Arc::new(model),
            notify: Notifier,
        }
    }

    pub async fn generate_clients(
        &mut self,
        number: u64,
        r#type: ClientType,
    ) -> anyhow::Result<Vec<StateMachine>> {
        let round_params = self.api_client.get_round_params().await?;
        let mut container = Vec::with_capacity(number as usize);

        for _ in 0..number {
            let new_client = generate_client(
                &r#type,
                &round_params,
                self.api_client.clone(),
                LocalModel(self.model.clone()),
                self.notify.clone(),
            )?;
            container.push(new_client);
        }

        Ok(container)
    }

    pub async fn build_sum_clients(
        &mut self,
    ) -> anyhow::Result<
        ConcurrentFutures<Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>>,
    > {
        let round_params = self.api_client.get_round_params().await?;
        let mut sum_clients = ConcurrentFutures::<
            Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>,
        >::new(100);

        for _ in 0..self.settings.number_of_sum {
            let sum_client = generate_client(
                &ClientType::Sum,
                &round_params,
                self.api_client.clone(),
                LocalModel(self.model.clone()),
                self.notify.clone(),
            )?;

            sum_clients.push(Box::pin(async {
                let mut sum_client = sum_client;

                for _ in 0..4 {
                    sum_client = match sum_client.transition().await {
                        TransitionOutcome::Pending(s) => s,
                        TransitionOutcome::Complete(s) => s,
                    };
                }

                sum_client
            }));
        }

        Ok(sum_clients)
    }

    pub async fn build_update_clients(
        &mut self,
    ) -> anyhow::Result<
        ConcurrentFutures<Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>>,
    > {
        let round_params = self.api_client.get_round_params().await?;

        let mut update_clients = ConcurrentFutures::<
            Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>,
        >::new(100);

        for _ in 0..self.settings.number_of_update {
            let update_client = generate_client(
                &ClientType::Update,
                &round_params,
                self.api_client.clone(),
                LocalModel(self.model.clone()),
                self.notify.clone(),
            )?;

            update_clients.push(Box::pin(async {
                let mut update_client = update_client;

                for _ in 0..8 {
                    update_client = match update_client.transition().await {
                        TransitionOutcome::Pending(s) => s,
                        TransitionOutcome::Complete(s) => s,
                    };
                }

                update_client
            }));
        }

        Ok(update_clients)
    }

    pub async fn build_clients(&mut self) -> anyhow::Result<ClientRunner> {
        let sum_clients = self.build_sum_clients().await?;
        let update_clients = self.build_update_clients().await?;
        Ok(ClientRunner::new(sum_clients, update_clients))
    }
}
