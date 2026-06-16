mod db;
mod models;
mod user_handler;
mod seat_handler;
mod reservation_handler;
mod attendance_handler;
use actix_cors::Cors;

use actix_web::{web, App, HttpServer, HttpResponse, Responder};
use std::sync::Mutex;

// 导入所有 handler 函数
use seat_handler::{
    get_seats,
    get_seat_by_id,
    get_available_seats,
    create_seat,
    update_seat,
    delete_seat,
};

use user_handler::{
    register,
    login,
    get_user_by_student_id,
    update_user_info,
    change_password,
    get_all_users,
};

use reservation_handler::{
    create_reservation,
    //get_my_reservations,
    //get_reservation_detail,
    cancel_reservation,
    //extend_reservation,
};

use attendance_handler::{
    checkin,
    checkout,
    get_attendance_status,
};

// 健康检查接口
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Server is running")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 初始化日志
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));

    // 初始化数据库连接
    let db_path = "library.db";
    let conn = rusqlite::Connection::open(db_path).expect("Failed to open database");

    // 初始化数据库表
    init_database(&conn);

    let db_state = web::Data::new(Mutex::new(conn));

    println!("Server starting at http://127.0.0.1:8080");

    // 使用 .service() 或 .route() 但保持一致的语法
    HttpServer::new(move || {
        App::new()

        .wrap(
              Cors::default().allow_any_origin().allow_any_method().allow_any_header()
            .supports_credentials()
        )
        
            .app_data(db_state.clone())
            // 健康检查
            .route("/health", web::get().to(health_check))
            // ========== 座位管理接口 ==========
            .route("/api/seats/available", web::get().to(get_available_seats))
            .route("/api/seats", web::get().to(get_seats))
            .route("/api/seats/{id}", web::get().to(get_seat_by_id))
            .route("/api/seats", web::post().to(create_seat))
            .route("/api/seats/{id}", web::put().to(update_seat))
            .route("/api/seats/{id}", web::delete().to(delete_seat))
            // ========== 用户管理接口 ==========
            .route("/api/auth/register", web::post().to(register))
            .route("/api/auth/login", web::post().to(login))
            .route("/api/users/{student_id}", web::get().to(get_user_by_student_id))
            .route("/api/users/{student_id}", web::put().to(update_user_info))
            .route("/api/users/password/{student_id}", web::put().to(change_password))
            .route("/api/users", web::get().to(get_all_users))
            // ========== 预约管理接口 ==========
            // 注意：这些接口的 user_id 应该从 JWT token 中获取，这里暂时作为路径参数
            .route("/api/reservations", web::post().to(create_reservation))
            //.route("/api/reservations", web::get().to(get_my_reservations))
            //.route("/api/reservations/{id}", web::get().to(get_reservation_detail))
            .route("/api/reservations/{id}", web::delete().to(cancel_reservation))
            //.route("/api/reservations/{id}/extend", web::put().to(extend_reservation))
            // ========== 签到签退接口 ==========
            .route("/api/checkin", web::post().to(checkin))
            .route("/api/checkout", web::post().to(checkout))
            .route("/api/attendance/status", web::get().to(get_attendance_status))
         })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

// 初始化数据库表
fn init_database(conn: &rusqlite::Connection) {
    // 创建 users 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            student_id TEXT UNIQUE NOT NULL,
            username TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            email TEXT NOT NULL,
            phone TEXT,
            role TEXT DEFAULT 'student',
            created_at TEXT NOT NULL
        )",
        [],
    ).expect("Failed to create users table");

    // 创建 seats 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS seats (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            seat_number TEXT NOT NULL UNIQUE,
            area TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'available',
            x_coord INTEGER,
            y_coord INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        )",
        [],
    ).expect("Failed to create seats table");
    
    // 创建 reservations 表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS reservations (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            seat_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (seat_id) REFERENCES seats(id)
        )",
        [],
    ).expect("Failed to create reservations table");
    
    // 创建 attendance 签到记录表
    conn.execute(
        "CREATE TABLE IF NOT EXISTS attendance (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            reservation_id INTEGER NOT NULL,
            seat_id INTEGER NOT NULL,
            checkin_time TEXT NOT NULL,
            checkout_time TEXT,
            location TEXT,
            FOREIGN KEY (user_id) REFERENCES users(id),
            FOREIGN KEY (reservation_id) REFERENCES reservations(id),
            FOREIGN KEY (seat_id) REFERENCES seats(id)
        )",
        [],
    ).expect("Failed to create attendance table");
    // main.rs - init_database 函数中添加

    // 插入管理员账号（密码是 admin123 的 bcrypt hash）
    let admin_hash = "$2b$12$2Yq4uQkE5ZvXxV5Yq4uQkE"; // 实际应该用正确的 hash
    let now = chrono::Utc::now().to_rfc3339();
    let _ = conn.execute(
        "INSERT OR IGNORE INTO users (student_id, username, password_hash, email, phone, role, created_at) 
         VALUES ('admin001', '管理员', ?1, 'admin@library.com', '13800000000', 'admin', ?2)",
        [admin_hash, &now],
    );
    
    // 插入测试座位数据
    let test_seats = [
        ("A001", "A区", Some(100), Some(100)),
        ("A002", "A区", Some(150), Some(100)),
        ("A003", "A区", Some(200), Some(100)),
        ("B001", "B区", Some(100), Some(200)),
        ("B002", "B区", Some(150), Some(200)),
        ("B003", "B区", Some(200), Some(200)),
        ("C001", "C区", Some(100), Some(300)),
        ("C002", "C区", Some(150), Some(300)),
        ("C003", "C区", Some(200), Some(300)),
    ];
    
    for (seat_num, area, x, y) in test_seats {
        let _ = conn.execute(
            "INSERT OR IGNORE INTO seats (seat_number, area, x_coord, y_coord, created_at, updated_at) 
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
            [seat_num, area, &x.map(|v| v.to_string()).unwrap_or_default(), 
             &y.map(|v| v.to_string()).unwrap_or_default(), &now],
        );
    }

    println!("✅ 数据库初始化成功，文件: library.db");
}