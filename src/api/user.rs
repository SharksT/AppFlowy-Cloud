use crate::component::auth::{
    change_password, login, logout, register, ChangePasswordRequest, InputParamsError, LoggedUser,
    LoginRequest, RegisterRequest,
};
use crate::component::token_state::SessionToken;
use crate::domain::{UserEmail, UserName, UserPassword};
use crate::state::State;

use actix_web::web::{Data, Json};
use actix_web::Result;
use actix_web::{web, HttpResponse, Scope};

pub fn user_scope() -> Scope {
    web::scope("/api/user")
        .service(web::resource("/login").route(web::post().to(login_handler)))
        .service(web::resource("/logout").route(web::get().to(logout_handler)))
        .service(web::resource("/register").route(web::post().to(register_handler)))
        .service(web::resource("/password").route(web::post().to(change_password_handler)))
}

async fn login_handler(
    req: Json<LoginRequest>,
    state: Data<State>,
    session: SessionToken,
) -> Result<HttpResponse> {
    let req = req.into_inner();
    let email = UserEmail::parse(req.email)
        .map_err(|e| InputParamsError::InvalidEmail(e))?
        .0;
    let password = UserPassword::parse(req.password)
        .map_err(|_| InputParamsError::InvalidPassword)?
        .0;
    let (resp, token) = login(state.pg_pool.clone(), state.user.clone(), email, password).await?;

    // Renews the session key, assigning existing session state to new key.
    session.renew();
    if let Err(err) = session.insert_token(token) {
        // It needs to navigate to login page in web application
        tracing::error!("Insert session failed: {}", err);
    }

    Ok(HttpResponse::Ok().json(resp))
}

async fn logout_handler(logged_user: LoggedUser, state: Data<State>) -> Result<HttpResponse> {
    logout(logged_user, state.user.clone()).await;
    Ok(HttpResponse::Ok().finish())
}

#[tracing::instrument(level = "debug", skip(state))]
async fn register_handler(req: Json<RegisterRequest>, state: Data<State>) -> Result<HttpResponse> {
    let req = req.into_inner();
    let name = UserName::parse(req.name)
        .map_err(|e| InputParamsError::InvalidName(e))?
        .0;
    let email = UserEmail::parse(req.email)
        .map_err(|e| InputParamsError::InvalidEmail(e))?
        .0;
    let password = UserPassword::parse(req.password)
        .map_err(|_| InputParamsError::InvalidPassword)?
        .0;

    let resp = register(
        state.pg_pool.clone(),
        state.user.clone(),
        name,
        email,
        password,
    )
    .await?;

    Ok(HttpResponse::Ok().json(resp))
}

async fn change_password_handler(
    req: Json<ChangePasswordRequest>,
    logged_user: LoggedUser,
    // session: SessionToken,
    state: Data<State>,
) -> Result<HttpResponse> {
    let req = req.into_inner();
    if req.new_password != req.new_password_confirm {
        return Err(InputParamsError::PasswordNotMatch.into());
    }

    let new_password = UserPassword::parse(req.new_password)
        .map_err(|_| InputParamsError::InvalidPassword)?
        .0;

    change_password(
        state.pg_pool.clone(),
        logged_user.clone(),
        req.current_password,
        new_password,
    )
    .await?;

    Ok(HttpResponse::Ok().finish())
}