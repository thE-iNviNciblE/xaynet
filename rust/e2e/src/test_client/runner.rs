use crate::utils::concurrent_futures::ConcurrentFutures;
use futures::{Future, StreamExt};
use std::pin::Pin;
use xaynet_sdk::{StateMachine, TransitionOutcome};

pub struct ClientRunner {
    sum_clients:
        Option<ConcurrentFutures<Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>>>,
    update_clients:
        Option<ConcurrentFutures<Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>>>,
    sum2_clients:
        Option<ConcurrentFutures<Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>>>,
}

impl ClientRunner {
    pub fn new(
        sum_clients: ConcurrentFutures<
            Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>,
        >,
        update_clients: ConcurrentFutures<
            Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>,
        >,
    ) -> Self {
        Self {
            sum_clients: Some(sum_clients),
            update_clients: Some(update_clients),
            sum2_clients: None,
        }
    }

    pub async fn run_sum_clients(&mut self) -> anyhow::Result<()> {
        let mut sum2_clients = ConcurrentFutures::<
            Pin<Box<dyn Future<Output = StateMachine> + Send + 'static>>,
        >::new(100);

        let mut sum_clients = self
            .sum_clients
            .take()
            .ok_or_else(|| anyhow::anyhow!("No sum clients available"))?;

        while let Some(sum_client) = sum_clients.next().await {
            let sum_client = sum_client?;
            sum2_clients.push(Box::pin(async {
                let mut sum_client = sum_client;

                for _ in 0..5 {
                    sum_client = match sum_client.transition().await {
                        TransitionOutcome::Pending(s) => s,
                        TransitionOutcome::Complete(s) => s,
                    };
                }

                sum_client
            }));
        }

        self.sum2_clients = Some(sum2_clients);

        Ok(())
    }

    pub async fn run_update_clients(&mut self) -> anyhow::Result<()> {
        let mut update_clients = self
            .update_clients
            .take()
            .ok_or_else(|| anyhow::anyhow!("No update clients available"))?;

        while update_clients.next().await.is_some() {}

        Ok(())
    }
    pub async fn run_sum2_clients(&mut self) -> anyhow::Result<()> {
        let mut sum2_clients = self
            .sum2_clients
            .take()
            .ok_or_else(|| anyhow::anyhow!("No sum2 clients available"))?;

        while sum2_clients.next().await.is_some() {}

        Ok(())
    }
}
