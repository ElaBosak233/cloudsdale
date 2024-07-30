pub mod request;

use axum::async_trait;
use sea_orm::{entity::prelude::*, FromJsonQueryResult, Set, TryIntoModel};
use serde::{Deserialize, Serialize};

use crate::database::get_db;

use super::{challenge, game, team, user};

#[derive(Debug, Clone, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "pods")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub flag: Option<String>, // The generated flag, which will be injected into the container.
    pub user_id: i64,
    pub team_id: Option<i64>,
    pub game_id: Option<i64>,
    pub challenge_id: i64,
    #[sea_orm(column_type = "Json")]
    pub nats: Vec<Nat>,
    pub removed_at: i64,
    pub created_at: i64,

    #[sea_orm(ignore)]
    pub user: Option<user::Model>,
    #[sea_orm(ignore)]
    pub team: Option<team::Model>,
    #[sea_orm(ignore)]
    pub challenge: Option<challenge::Model>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult, Default)]
pub struct Nat {
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub proxy: Option<String>,
    pub entry: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Challenge,
    User,
    Team,
    Game,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Challenge => Entity::belongs_to(challenge::Entity)
                .from(Column::ChallengeId)
                .to(challenge::Column::Id)
                .into(),
            Self::User => Entity::belongs_to(user::Entity)
                .from(Column::UserId)
                .to(user::Column::Id)
                .into(),
            Self::Team => Entity::belongs_to(team::Entity)
                .from(Column::TeamId)
                .to(team::Column::Id)
                .into(),
            Self::Game => Entity::belongs_to(game::Entity)
                .from(Column::GameId)
                .to(game::Column::Id)
                .into(),
        }
    }
}

impl Related<challenge::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Challenge.def()
    }
}

impl Related<user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl Related<team::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Team.def()
    }
}

impl Related<game::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

#[async_trait]
impl ActiveModelBehavior for ActiveModel {
    fn new() -> Self {
        Self {
            created_at: Set(chrono::Utc::now().timestamp()),
            ..ActiveModelTrait::default()
        }
    }
}

async fn preload(
    mut pods: Vec<crate::model::pod::Model>,
) -> Result<Vec<crate::model::pod::Model>, DbErr> {
    let users = pods
        .load_one(crate::model::user::Entity, &get_db().await)
        .await?;
    let teams = pods
        .load_one(crate::model::team::Entity, &get_db().await)
        .await?;
    let challenges = pods
        .load_one(crate::model::challenge::Entity, &get_db().await)
        .await?;

    for i in 0..pods.len() {
        let mut pod = pods[i].clone();
        pod.user = users[i].clone();
        pod.team = teams[i].clone();
        pod.challenge = challenges[i].clone();
        pods[i] = pod;
    }

    return Ok(pods);
}

pub async fn find(
    id: Option<i64>, name: Option<String>, user_id: Option<i64>, team_id: Option<i64>,
    game_id: Option<i64>, challenge_id: Option<i64>, is_available: Option<bool>,
) -> Result<(Vec<crate::model::pod::Model>, u64), DbErr> {
    let mut query = crate::model::pod::Entity::find();
    if let Some(id) = id {
        query = query.filter(crate::model::pod::Column::Id.eq(id));
    }

    if let Some(name) = name {
        query = query.filter(crate::model::pod::Column::Name.eq(name));
    }

    if let Some(user_id) = user_id {
        query = query.filter(crate::model::pod::Column::UserId.eq(user_id));
    }

    if let Some(team_id) = team_id {
        query = query.filter(crate::model::pod::Column::TeamId.eq(team_id));
    }

    if let Some(game_id) = game_id {
        query = query.filter(crate::model::pod::Column::GameId.eq(game_id));
    }

    if let Some(challenge_id) = challenge_id {
        query = query.filter(crate::model::pod::Column::ChallengeId.eq(challenge_id));
    }

    if let Some(is_available) = is_available {
        match is_available {
            true => {
                query = query.filter(
                    crate::model::pod::Column::RemovedAt.gte(chrono::Utc::now().timestamp()),
                )
            }
            false => {
                query = query.filter(
                    crate::model::pod::Column::RemovedAt.lte(chrono::Utc::now().timestamp()),
                )
            }
        }
    }

    let total = query.clone().count(&get_db().await).await?;

    let mut pods = query.all(&get_db().await).await?;

    pods = preload(pods).await?;

    return Ok((pods, total));
}

pub async fn create(
    pod: crate::model::pod::ActiveModel,
) -> Result<crate::model::pod::Model, DbErr> {
    return pod.insert(&get_db().await).await?.try_into_model();
}

pub async fn update(
    pod: crate::model::pod::ActiveModel,
) -> Result<crate::model::pod::Model, DbErr> {
    return pod.update(&get_db().await).await?.try_into_model();
}

pub async fn delete(id: i64) -> Result<(), DbErr> {
    let result = crate::model::pod::Entity::delete_by_id(id)
        .exec(&get_db().await)
        .await?;
    return Ok(if result.rows_affected == 0 {
        return Err(DbErr::RecordNotFound(format!(
            "Pod with id {} not found",
            id
        )));
    });
}
