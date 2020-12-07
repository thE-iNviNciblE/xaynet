use super::influx::InfluxClient;
use crate::utils::terminal::spinner;
use tokio::time::{interval, Duration};
use xaynet_sdk::{client::Client as HttpApiClient, XaynetClient};
use xaynet_server::state_machine::phases::PhaseName;

pub async fn wait_until_coordinator_is_ready(client: &mut HttpApiClient<reqwest::Client>) {
    let mut interval = interval(Duration::from_millis(500));
    while client.get_round_params().await.is_err() {
        interval.tick().await;
    }
}

pub async fn wait_until_influxdb_is_ready(client: &InfluxClient) {
    let mut interval = interval(Duration::from_millis(500));
    while client.ping().await.is_err() {
        interval.tick().await;
    }
}

pub async fn wait_until_phase(client: &InfluxClient, phase: PhaseName) {
    let spinner = spinner(&format!("Wait for phase {:?}", phase), "");
    let mut interval = interval(Duration::from_millis(500));
    loop {
        let current_phase = client.get_current_phase().await;
        match current_phase {
            Ok(current_phase) => {
                if current_phase == phase {
                    break;
                } else {
                    spinner.set_message(&format!("current phase: {:?}", current_phase));
                }
            }
            Err(_) => spinner.set_message("No phase metrics available"),
        }
        interval.tick().await;
    }
    spinner.finish_with_message("Ok");
}
