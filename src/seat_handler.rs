// seat_handler.rs - 简化版本，避免复杂的动态参数

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::models::{
    ApiResponse, CreateSeatRequest, GetSeatsQuery, Seat, UpdateSeatRequest,
    AvailableSeatsQuery,
};

/// 获取所有座位列表（支持按区域、状态筛选）
pub async fn get_seats(
    query: web::Query<GetSeatsQuery>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    // 简化：根据是否有参数使用不同的查询
    let (sql, params_vec) = if query.area.is_some() || query.status.is_some() {
        let mut sql = String::from(
            "SELECT id, seat_number, area, status, x_coord, y_coord, created_at, updated_at FROM seats WHERE 1=1"
        );
        let mut params: Vec<String> = Vec::new();
        
        if let Some(area) = &query.area {
            sql.push_str(" AND area = ?");
            params.push(area.clone());
        }
        
        if let Some(status) = &query.status {
            sql.push_str(" AND status = ?");
            params.push(status.clone());
        }
        
        sql.push_str(" ORDER BY area, seat_number");
        (sql, params)
    } else {
        (
            "SELECT id, seat_number, area, status, x_coord, y_coord, created_at, updated_at FROM seats ORDER BY area, seat_number".to_string(),
            vec![]
        )
    };
    
    let mut stmt = match conn.prepare(&sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    // 使用不同的查询方法避免闭包类型问题
    let seats: Vec<Seat> = if params_vec.is_empty() {
        let rows = stmt.query_map([], |row| {
            Ok(Seat {
                id: row.get(0)?,
                seat_number: row.get(1)?,
                area: row.get(2)?,
                status: row.get(3)?,
                x_coord: row.get(4)?,
                y_coord: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        });
        match rows {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        }
    } else {
        let param_refs: Vec<&str> = params_vec.iter().map(|s| s.as_str()).collect();
        let rows = stmt.query_map(rusqlite::params_from_iter(param_refs), |row| {
            Ok(Seat {
                id: row.get(0)?,
                seat_number: row.get(1)?,
                area: row.get(2)?,
                status: row.get(3)?,
                x_coord: row.get(4)?,
                y_coord: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        });
        match rows {
            Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
            Err(e) => {
                eprintln!("查询失败: {}", e);
                return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                    success: false,
                    message: "服务器内部错误".to_string(),
                    data: None,
                });
            }
        }
    };
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("共 {} 个座位", seats.len()),
        data: Some(seats),
    })
}

/// 获取单个座位详细信息
pub async fn get_seat_by_id(
    seat_id: web::Path<i32>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let conn = db.lock().unwrap();
    
    let mut stmt = match conn.prepare(
        "SELECT id, seat_number, area, status, x_coord, y_coord, created_at, updated_at FROM seats WHERE id = ?1"
    ) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Seat> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let mut rows = match stmt.query(params![seat_id]) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Seat> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    if let Some(row) = rows.next().unwrap() {
        let seat = Seat {
            id: row.get(0).unwrap(),
            seat_number: row.get(1).unwrap(),
            area: row.get(2).unwrap(),
            status: row.get(3).unwrap(),
            x_coord: row.get(4).unwrap(),
            y_coord: row.get(5).unwrap(),
            created_at: row.get(6).unwrap(),
            updated_at: row.get(7).unwrap(),
        };
        
        HttpResponse::Ok().json(ApiResponse {
            success: true,
            message: "获取成功".to_string(),
            data: Some(seat),
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<Seat> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        })
    }
}

/// 获取当前空闲座位
pub async fn get_available_seats(
    query: web::Query<AvailableSeatsQuery>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let conn = db.lock().unwrap();
    
    let now = Utc::now().to_rfc3339();
    
    let sql = r#"
        SELECT s.id, s.seat_number, s.area, s.status, s.x_coord, s.y_coord, s.created_at, s.updated_at 
        FROM seats s
        WHERE s.status = 'available'
        AND NOT EXISTS (
            SELECT 1 FROM reservations r 
            WHERE r.seat_id = s.id 
            AND r.status IN ('pending', 'active')
            AND r.start_time <= ?2
            AND r.end_time >= ?1
        )
        ORDER BY s.area, s.seat_number
    "#;
    
    let start_time = query.start_time.as_ref().unwrap_or(&now);
    let end_time = query.end_time.as_ref().unwrap_or(&now);
    
    let mut stmt = match conn.prepare(sql) {
        Ok(stmt) => stmt,
        Err(e) => {
            eprintln!("准备查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let rows = match stmt.query_map(params![start_time, end_time], |row| {
        Ok(Seat {
            id: row.get(0)?,
            seat_number: row.get(1)?,
            area: row.get(2)?,
            status: row.get(3)?,
            x_coord: row.get(4)?,
            y_coord: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    }) {
        Ok(rows) => rows,
        Err(e) => {
            eprintln!("查询失败: {}", e);
            return HttpResponse::InternalServerError().json(ApiResponse::<Vec<Seat>> {
                success: false,
                message: "服务器内部错误".to_string(),
                data: None,
            });
        }
    };
    
    let seats: Vec<Seat> = rows.filter_map(|r| r.ok()).collect();
    
    HttpResponse::Ok().json(ApiResponse {
        success: true,
        message: format!("当前共有 {} 个空闲座位", seats.len()),
        data: Some(seats),
    })
}

/// 管理员添加座位
pub async fn create_seat(
    req: web::Json<CreateSeatRequest>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    // 直接使用 Option 值，params! 可以处理 Option
    match conn.execute(
        "INSERT INTO seats (seat_number, area, status, x_coord, y_coord, created_at, updated_at) 
         VALUES (?1, ?2, 'available', ?3, ?4, ?5, ?5)",
        params![&req.seat_number, &req.area, req.x_coord, req.y_coord, &now],
    ) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "座位添加成功".to_string(),
            data: None,
        }),
        Err(e) => {
            if e.to_string().contains("UNIQUE") {
                HttpResponse::BadRequest().json(ApiResponse::<()> {
                    success: false,
                    message: "座位号已存在".to_string(),
                    data: None,
                })
            } else {
                eprintln!("添加座位失败: {}", e);
                HttpResponse::InternalServerError().json(ApiResponse::<()> {
                    success: false,
                    message: "添加座位失败".to_string(),
                    data: None,
                })
            }
        }
    }
}

/// 管理员修改座位信息
pub async fn update_seat(
    seat_id: web::Path<i32>,
    req: web::Json<UpdateSeatRequest>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let now = Utc::now().to_rfc3339();
    let conn = db.lock().unwrap();
    
    // 简化：分别处理每个字段的更新
    if let Some(seat_number) = &req.seat_number {
        let _ = conn.execute(
            "UPDATE seats SET seat_number = ?1, updated_at = ?2 WHERE id = ?3",
            params![seat_number, &now, seat_id],
        );
    }
    
    if let Some(area) = &req.area {
        let _ = conn.execute(
            "UPDATE seats SET area = ?1, updated_at = ?2 WHERE id = ?3",
            params![area, &now, seat_id],
        );
    }
    
    if let Some(status) = &req.status {
        let _ = conn.execute(
            "UPDATE seats SET status = ?1, updated_at = ?2 WHERE id = ?3",
            params![status, &now, seat_id],
        );
    }
    
    if let Some(x_coord) = &req.x_coord {
        let _ = conn.execute(
            "UPDATE seats SET x_coord = ?1, updated_at = ?2 WHERE id = ?3",
            params![x_coord, &now, seat_id],
        );
    }
    
    if let Some(y_coord) = &req.y_coord {
        let _ = conn.execute(
            "UPDATE seats SET y_coord = ?1, updated_at = ?2 WHERE id = ?3",
            params![y_coord, &now, seat_id],
        );
    }
    
    // 检查座位是否存在
    let exists: bool = conn.query_row(
        "SELECT 1 FROM seats WHERE id = ?1",
        params![seat_id],
        |_| Ok(()),
    ).is_ok();
    
    if exists {
        HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "座位更新成功".to_string(),
            data: None,
        })
    } else {
        HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        })
    }
}

/// 管理员删除座位
pub async fn delete_seat(
    seat_id: web::Path<i32>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let seat_id = seat_id.into_inner();
    let conn = db.lock().unwrap();
    
    // 检查是否有正在进行的预约
    let has_active_reservation: bool = match conn.query_row(
        "SELECT 1 FROM reservations WHERE seat_id = ?1 AND status IN ('pending', 'active') LIMIT 1",
        params![seat_id],
        |_| Ok(true),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };
    
    if has_active_reservation {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "该座位有正在进行的预约，无法删除".to_string(),
            data: None,
        });
    }
    
    match conn.execute("DELETE FROM seats WHERE id = ?1", params![seat_id]) {
        Ok(affected) if affected > 0 => HttpResponse::Ok().json(ApiResponse::<()> {
            success: true,
            message: "座位删除成功".to_string(),
            data: None,
        }),
        Ok(_) => HttpResponse::NotFound().json(ApiResponse::<()> {
            success: false,
            message: "座位不存在".to_string(),
            data: None,
        }),
        Err(e) => {
            eprintln!("删除座位失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "删除座位失败".to_string(),
                data: None,
            })
        }
    }
}