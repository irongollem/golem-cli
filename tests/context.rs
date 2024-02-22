pub mod db;
pub mod golem_worker_service;
pub mod redis;
pub mod shard_manager;
pub mod worker;
pub mod nginx;

pub mod golem_template_service;

use crate::context::db::{Db, DbInfo};
use crate::context::redis::{Redis, RedisInfo};
use crate::context::shard_manager::{ShardManager, ShardManagerInfo};
use crate::context::worker::{WorkerExecutors, WorkerExecutorsInfo};
use libtest_mimic::Failed;
use std::path::PathBuf;
use testcontainers::clients;
use crate::context::golem_template_service::{GolemTemplateService, GolemTemplateServiceInfo};
use crate::context::golem_worker_service::{GolemWorkerService, GolemWorkerServiceInfo};

const NETWORK: &str = "golem_test_network";
const TAG: &str = "v0.0.60";

#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub verbose: bool,
    pub on_ci: bool,
    pub quiet: bool,
    pub redis_key_prefix: String,
    pub wasm_root: PathBuf,
    pub local_golem: bool,
    pub db_type: DbType,
}

#[derive(Debug, Clone)]
pub enum DbType {
    Postgres,
    Sqlite,
}

impl DbType {
    pub fn from_env() -> DbType {
        let db_type_str = std::env::var("GOLEM_TEST_DB")
            .unwrap_or("".to_string())
            .to_lowercase();
        if db_type_str == "sqlite" {
            DbType::Sqlite
        } else {
            DbType::Postgres
        }
    }
}

impl EnvConfig {
    pub fn from_env() -> EnvConfig {
        EnvConfig {
            verbose: std::env::var("CI").is_err(),
            on_ci: std::env::var("CI").is_ok(),
            quiet: std::env::var("QUIET").is_ok(),
            redis_key_prefix: std::env::var("REDIS_KEY_PREFIX").unwrap_or("".to_string()),
            wasm_root: PathBuf::from(
                std::env::var("GOLEM_TEST_TEMPLATES").unwrap_or("../test-templates".to_string()),
            ),
            local_golem: std::env::var("GOLEM_DOCKER_SERVICES").is_err(),
            db_type: DbType::from_env(),
        }
    }
}

pub struct Context<'docker_client> {
    env: EnvConfig,
    db: Db<'docker_client>,
    redis: Redis<'docker_client>,
    shard_manager: ShardManager<'docker_client>,
    golem_template_service: GolemTemplateService<'docker_client>,
    golem_worker_service: GolemWorkerService<'docker_client>,
    worker_executors: WorkerExecutors<'docker_client>,
    nginx: Nginx<'docker_client>
}

impl Context<'_> {
    pub fn start(docker: &clients::Cli) -> Result<Context, Failed> {
        let env_config = EnvConfig::from_env();

        println!("Starting context with env config: {env_config:?}");

        let db = Db::start(docker, &env_config)?;
        let redis = Redis::make(docker, &env_config)?;
        let shard_manager = ShardManager::start(docker, &env_config, &redis.info())?;

        let golem_template_service =
            GolemTemplateService::start(docker, &env_config, &db.info())?;

        let golem_worker_service =
            GolemWorkerService::start(docker, &env_config, &shard_manager.info(), &db.info(), &redis.info(), &golem_template_service.info())?;

        let worker_executors = WorkerExecutors::start(
            docker,
            &env_config,
            &redis.info(),
            &golem_worker_service.info(),
            &golem_template_service.info(),
            &shard_manager.info(),
        )?;

        Ok(Context {
            env: env_config,
            db,
            redis,
            shard_manager,
            golem_template_service,
            golem_worker_service,
            worker_executors,
        })
    }

    pub fn info(&self) -> ContextInfo {
        ContextInfo {
            env: self.env.clone(),
            db: self.db.info(),
            redis: self.redis.info(),
            shard_manager: self.shard_manager.info(),
            golem_template_service: self.golem_template_service.info(),
            golem_worker_service: self.golem_worker_service.info(),
            worker_executors: self.worker_executors.info(),
        }
    }
}

impl Drop for Context<'_> {
    fn drop(&mut self) {
        println!("Stopping Context")
    }
}

pub struct ContextInfo {
    pub env: EnvConfig,
    pub db: DbInfo,
    pub redis: RedisInfo,
    pub shard_manager: ShardManagerInfo,
    pub golem_template_service: GolemTemplateServiceInfo,
    pub golem_worker_service: GolemWorkerServiceInfo,
    pub worker_executors: WorkerExecutorsInfo,
}
