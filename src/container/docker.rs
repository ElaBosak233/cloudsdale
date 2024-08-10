use super::traits::Container;
use crate::{config, database::get_db, model::pod};
use async_trait::async_trait;
use bollard::{
    container::{Config, CreateContainerOptions, StartContainerOptions},
    secret::{ContainerCreateResponse, HostConfig, PortBinding},
    Docker as DockerClient,
};
use core::time;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::{collections::HashMap, env, error::Error, process, sync::OnceLock};
use tracing::{error, info};

static DOCKER_CLI: OnceLock<DockerClient> = OnceLock::new();

fn get_docker_client() -> &'static DockerClient {
    return DOCKER_CLI.get().unwrap();
}

#[derive(Clone)]
pub struct Docker;

impl Docker {
    pub fn new() -> Self {
        return Self {};
    }
}

#[async_trait]
impl Container for Docker {
    async fn init(&self) {
        let docker_uri = &crate::config::get_config().container.docker.uri;
        env::set_var("DOCKER_HOST", docker_uri);
        let docker = DockerClient::connect_with_defaults().unwrap();
        match docker.ping().await {
            Ok(_) => {
                info!("Docker client initialized successfully.");
                DOCKER_CLI.set(docker).unwrap();
            }
            Err(e) => {
                error!("Docker client initialization failed: {0:?}", e);
                process::exit(1);
            }
        }
    }

    async fn create(
        &self, name: String, challenge: crate::model::challenge::Model,
        injected_flag: crate::model::challenge::Flag,
    ) -> Result<Vec<crate::model::pod::Nat>, Box<dyn Error>> {
        let port_bindings: HashMap<String, Option<Vec<PortBinding>>> = challenge
            .ports
            .into_iter()
            .map(|port| {
                (
                    format!("{}/{}", port, "tcp"),
                    Some(vec![PortBinding {
                        host_ip: Some("0.0.0.0".to_string()),
                        host_port: None,
                    }]),
                )
            })
            .collect();

        let mut env_bindings: Vec<String> = challenge
            .envs
            .into_iter()
            .map(|env| format!("{}={}", env.key, env.value))
            .collect();

        env_bindings.push(format!(
            "{}={}",
            injected_flag.env.unwrap_or("FLAG".to_string()),
            injected_flag.value
        ));

        let cfg = Config {
            image: challenge.image_name.clone(),
            host_config: Some(HostConfig {
                memory: Some(challenge.memory_limit * 1024 * 1024),
                cpu_shares: Some(challenge.cpu_limit),
                port_bindings: Some(port_bindings),
                ..Default::default()
            }),
            env: Some(env_bindings),
            ..Default::default()
        };

        let _: ContainerCreateResponse = get_docker_client()
            .create_container(
                Some(CreateContainerOptions {
                    name: name.clone(),
                    platform: None,
                }),
                cfg,
            )
            .await?;

        get_docker_client()
            .start_container(name.clone().as_str(), None::<StartContainerOptions<String>>)
            .await?;

        let container_info = get_docker_client().inspect_container(&name, None).await?;
        let port_mappings = container_info.network_settings.unwrap().ports.unwrap();

        let mut nats: Vec<crate::model::pod::Nat> = Vec::new();
        for (port, bindings) in port_mappings {
            if let Some(binding) = bindings {
                for port_binding in binding {
                    if let Some((src, _)) = port.split_once("/") {
                        nats.push(crate::model::pod::Nat {
                            src: src.to_string(),
                            dst: port_binding.host_port.clone(),
                            proxy: config::get_config().container.proxy.enabled,
                            entry: Some(format!(
                                "{}:{}",
                                config::get_config().container.entry,
                                port_binding.host_port.unwrap_or("0".to_string())
                            )),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        return Ok(nats);
    }

    async fn delete(&self, name: String) {
        let _ = get_docker_client()
            .stop_container(name.clone().as_str(), None)
            .await;
        let _ = get_docker_client()
            .remove_container(name.clone().as_str(), None)
            .await;
    }
}
