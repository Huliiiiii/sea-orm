mod db;
mod graphql;

use entity::async_graphql;

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use graphql::schema::{build_schema, AppSchema};

#[cfg(debug_assertions)]
use dotenvy::dotenv;

async fn graphql_handler(schema: State<AppSchema>, req: GraphQLRequest) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new(
        "/api/graphql",
    )))
}

#[tokio::main]
pub async fn main() {
    #[cfg(debug_assertions)]
    dotenv().ok();

    let schema = build_schema().await;

    let app = Router::new()
        .route(
            "/api/graphql",
            get(graphql_playground).post(graphql_handler),
        )
        .with_state(schema);

    println!("Playground: http://localhost:3000/api/graphql");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
