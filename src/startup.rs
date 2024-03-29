use crate::routes;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

pub fn run(address: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(routes::health_check))
            .route("/books", web::get().to(routes::books_index))
            .route("/books/{book_id}", web::get().to(routes::show_book))
            .route("/books/create", web::post().to(routes::create_book))
            .route("/books/delete", web::post().to(routes::delete_book))
            .route("/authors", web::get().to(routes::authors_index))
            .route("/authors/{author_id}", web::get().to(routes::show_author))
            .route("/authors/create", web::post().to(routes::create_author))
            .route("/authors/delete", web::post().to(routes::delete_author))
            .route("/users/create", web::post().to(routes::create_user))
            .route("/seed_authors", web::get().to(routes::seed_authors))
            .app_data(db_pool.clone())
    })
    .listen(address)?
    .run();

    Ok(server)
}
