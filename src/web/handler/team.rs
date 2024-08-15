use axum::body::Body;
use axum::{
    extract::{Multipart, Path, Query},
    http::{Response, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use mime::Mime;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter, QuerySelect, Set,
};

use crate::database::get_db;
use crate::model::user::group::Group;
use crate::web::model::{team::*, Metadata};
use crate::web::traits::{Ext, WebError};

fn can_modify_team(user: crate::model::user::Model, team_id: i64) -> bool {
    return user.group == Group::Admin
        || user
            .teams
            .iter()
            .any(|team| team.id == team_id && team.captain_id == user.id);
}

pub async fn get(Query(params): Query<GetRequest>) -> Result<impl IntoResponse, WebError> {
    let (teams, total) = crate::model::team::find(
        params.id,
        params.name,
        params.email,
        params.page,
        params.size,
    )
    .await?;

    return Ok((
        StatusCode::OK,
        Json(GetResponse {
            code: StatusCode::OK.as_u16(),
            data: teams,
            total: total,
        }),
    ));
}

pub async fn create(
    Extension(ext): Extension<Ext>, Json(body): Json<CreateRequest>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if !(operator.group == Group::Admin || operator.id == body.captain_id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let team = crate::model::team::ActiveModel {
        name: Set(body.name),
        email: Set(Some(body.email)),
        captain_id: Set(body.captain_id),
        description: body.description.map_or(NotSet, |v| Set(Some(v))),
        ..Default::default()
    }
    .insert(&get_db())
    .await?;

    let _ = crate::model::user_team::ActiveModel {
        user_id: Set(body.captain_id),
        team_id: Set(team.id),
        ..Default::default()
    }
    .insert(&get_db())
    .await?;

    return Ok((
        StatusCode::OK,
        Json(CreateResponse {
            code: StatusCode::OK.as_u16(),
            data: team,
        }),
    ));
}

pub async fn update(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>, Json(mut body): Json<UpdateRequest>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;

    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }
    body.id = Some(id);

    let team = crate::model::team::ActiveModel {
        id: body.id.map_or(NotSet, |v| Set(v)),
        name: body.name.map_or(NotSet, |v| Set(v)),
        email: body.email.map_or(NotSet, |v| Set(Some(v))),
        captain_id: body.captain_id.map_or(NotSet, |v| Set(v)),
        description: body.description.map_or(NotSet, |v| Set(Some(v))),
        ..Default::default()
    }
    .insert(&get_db())
    .await?;

    return Ok((
        StatusCode::OK,
        Json(UpdateResponse {
            code: StatusCode::OK.as_u16(),
            data: team,
        }),
    ));
}

pub async fn delete(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;

    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let _ = crate::model::team::Entity::delete_by_id(id)
        .exec(&get_db())
        .await?;

    return Ok((
        StatusCode::OK,
        Json(DeleteResponse {
            code: StatusCode::OK.as_u16(),
        }),
    ));
}

pub async fn create_user(
    Extension(ext): Extension<Ext>, Json(body): Json<CreateUserRequest>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if operator.group != Group::Admin {
        return Err(WebError::Forbidden(String::new()));
    }

    let _ = crate::model::user_team::ActiveModel {
        user_id: Set(body.user_id),
        team_id: Set(body.team_id),
    }
    .insert(&get_db())
    .await?;

    return Ok((
        StatusCode::OK,
        Json(CreateUserResponse {
            code: StatusCode::OK.as_u16(),
        }),
    ));
}

pub async fn delete_user(
    Extension(ext): Extension<Ext>, Path((id, user_id)): Path<(i64, i64)>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if !can_modify_team(operator.clone(), id) && operator.id != user_id {
        return Err(WebError::Forbidden(String::new()));
    }

    let _ = crate::model::user_team::Entity::delete_many()
        .filter(crate::model::user_team::Column::UserId.eq(user_id))
        .filter(crate::model::user_team::Column::TeamId.eq(id))
        .exec(&get_db())
        .await?;

    return Ok((
        StatusCode::OK,
        Json(DeleteUserResponse {
            code: StatusCode::OK.as_u16(),
        }),
    ));
}

pub async fn get_invite_token(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;

    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let team = crate::model::team::Entity::find_by_id(id)
        .select_only()
        .column(crate::model::team::Column::InviteToken)
        .one(&get_db())
        .await?
        .ok_or_else(|| WebError::NotFound(String::new()))?;

    return Ok((
        StatusCode::OK,
        Json(GetInviteTokenResponse {
            code: StatusCode::OK.as_u16(),
            token: team.invite_token,
        }),
    ));
}

pub async fn update_invite_token(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let mut team = crate::model::team::Entity::find_by_id(id)
        .one(&get_db())
        .await?
        .ok_or_else(|| WebError::NotFound(String::new()))?
        .into_active_model();

    let token = uuid::Uuid::new_v4().simple().to_string();
    team.invite_token = Set(Some(token.clone()));

    let _ = team.update(&get_db()).await?;

    return Ok((
        StatusCode::OK,
        Json(UpdateInviteTokenResponse {
            code: StatusCode::OK.as_u16(),
            token: token,
        }),
    ));
}

pub async fn join(
    Extension(ext): Extension<Ext>, Json(mut body): Json<JoinRequest>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;

    body.user_id = operator.id;

    let _ = crate::model::user::Entity::find_by_id(body.user_id)
        .one(&get_db())
        .await?
        .ok_or_else(|| WebError::NotFound(String::from("invalid_user_or_team")))?;

    let team = crate::model::team::Entity::find_by_id(body.team_id)
        .one(&get_db())
        .await?
        .ok_or_else(|| WebError::NotFound(String::from("invalid_user_or_team")))?;

    if Some(body.invite_token.clone()) != team.invite_token {
        return Err(WebError::BadRequest(String::from("invalid_invite_token")));
    }

    let user_team = crate::model::user_team::ActiveModel {
        user_id: Set(body.user_id),
        team_id: Set(body.team_id),
        ..Default::default()
    }
    .insert(&get_db())
    .await?;

    return Ok((
        StatusCode::OK,
        Json(JoinResponse {
            code: StatusCode::OK.as_u16(),
            data: user_team,
        }),
    ));
}

pub async fn leave() -> impl IntoResponse {
    todo!()
}

pub async fn get_avatar_metadata(Path(id): Path<i64>) -> Result<impl IntoResponse, WebError> {
    let path = format!("teams/{}/avatar", id);
    match crate::media::scan_dir(path.clone()).await.unwrap().first() {
        Some((filename, size)) => {
            return Ok((
                StatusCode::OK,
                Json(GetAvatarMetadataResponse {
                    code: StatusCode::OK.as_u16(),
                    data: Metadata {
                        filename: filename.to_string(),
                        size: *size,
                    },
                }),
            ));
        }
        None => return Err(WebError::NotFound(String::new())),
    }
}

pub async fn get_avatar(Path(id): Path<i64>) -> Result<impl IntoResponse, WebError> {
    let path = format!("teams/{}/avatar", id);
    match crate::media::scan_dir(path.clone()).await.unwrap().first() {
        Some((filename, _size)) => {
            let buffer = crate::media::get(path, filename.to_string()).await.unwrap();
            return Ok(Response::builder().body(Body::from(buffer)).unwrap());
        }
        None => return Err(WebError::NotFound(String::new())),
    }
}

pub async fn save_avatar(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>, mut multipart: Multipart,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let path = format!("teams/{}/avatar", id);
    let mut filename = String::new();
    let mut data = Vec::<u8>::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        if field.name() == Some("file") {
            filename = field.file_name().unwrap().to_string();
            let content_type = field.content_type().unwrap().to_string();
            let mime: Mime = content_type.parse().unwrap();
            if mime.type_() != mime::IMAGE {
                return Err(WebError::BadRequest(String::from("forbidden_file_type")));
            }
            data = match field.bytes().await {
                Ok(bytes) => bytes.to_vec(),
                Err(_err) => {
                    return Err(WebError::BadRequest(String::from("size_too_large")));
                }
            };
        }
    }

    crate::media::delete(path.clone()).await.unwrap();

    let _ = crate::media::save(path, filename, data)
        .await
        .map_err(|_| WebError::InternalServerError(String::new()))?;

    return Ok((
        StatusCode::OK,
        Json(SaveAvatarResponse {
            code: StatusCode::OK.as_u16(),
        }),
    ));
}

pub async fn delete_avatar(
    Extension(ext): Extension<Ext>, Path(id): Path<i64>,
) -> Result<impl IntoResponse, WebError> {
    let operator = ext.operator.ok_or(WebError::Unauthorized(String::new()))?;
    if !can_modify_team(operator, id) {
        return Err(WebError::Forbidden(String::new()));
    }

    let path = format!("teams/{}/avatar", id);

    let _ = crate::media::delete(path)
        .await
        .map_err(|_| WebError::InternalServerError(String::new()))?;

    return Ok((
        StatusCode::OK,
        Json(DeleteAvatarResponse {
            code: StatusCode::OK.as_u16(),
        }),
    ));
}
