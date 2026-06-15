// reservation_handler.rs - 修复参数类型

use actix_web::{web, HttpResponse, Responder};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use std::sync::Mutex;

use crate::models::{
    ApiResponse, CreateReservationRequest,
};

/// 创建预约
pub async fn create_reservation(
    req: web::Json<CreateReservationRequest>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let now = Utc::now().to_rfc3339();
    
    // 验证时间
    let start_time = match DateTime::parse_from_rfc3339(&req.start_time) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: "开始时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let end_time = match DateTime::parse_from_rfc3339(&req.end_time) {
        Ok(t) => t,
        Err(_) => {
            return HttpResponse::BadRequest().json(ApiResponse::<()> {
                success: false,
                message: "结束时间格式错误".to_string(),
                data: None,
            });
        }
    };
    
    let now_time = Utc::now();
    if start_time < now_time {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "开始时间不能早于当前时间".to_string(),
            data: None,
        });
    }
    
    if end_time <= start_time {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "结束时间必须晚于开始时间".to_string(),
            data: None,
        });
    }
    
    let duration = end_time.signed_duration_since(start_time);
    if duration.num_hours() > 4 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "单次预约最长不能超过4小时".to_string(),
            data: None,
        });
    }
    
    let conn = db.lock().unwrap();
    
    // 检查座位是否存在且可用
    let seat_status: String = match conn.query_row(
        "SELECT status FROM seats WHERE id = ?1",
        params![req.seat_id],
        |row| row.get(0),
    ) {
        Ok(status) => status,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "座位不存在".to_string(),
                data: None,
            });
        }
    };
    
    if seat_status != "available" {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "该座位当前不可用".to_string(),
            data: None,
        });
    }
    
    // 修复：使用 params! 宏，所有参数使用引用
    let conflict: bool = match conn.query_row(
        "SELECT 1 FROM reservations 
         WHERE seat_id = ?1 
         AND status IN ('pending', 'active')
         AND start_time <= ?3 
         AND end_time >= ?2",
        params![req.seat_id, &req.start_time, &req.end_time],
        |_| Ok(true),
    ) {
        Ok(_) => true,
        Err(_) => false,
    };
    
    if conflict {
        return HttpResponse::Conflict().json(ApiResponse::<()> {
            success: false,
            message: "该时间段座位已被预约".to_string(),
            data: None,
        });
    }
    
    // 检查用户是否有未完成的预约
    let active_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM reservations WHERE user_id = ?1 AND status IN ('pending', 'active')",
        params![user_id],
        |row| row.get(0),
    ).unwrap_or(0);
    
    if active_count >= 3 {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: "您已有3个进行中的预约，请先取消或完成后再预约".to_string(),
            data: None,
        });
    }
    
    // 修复：使用 params! 宏
    match conn.execute(
        "INSERT INTO reservations (user_id, seat_id, start_time, end_time, status, created_at, updated_at) 
         VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?5)",
        params![user_id, req.seat_id, &req.start_time, &req.end_time, &now],
    ) {
        Ok(_) => {
            let _ = conn.execute(
                "UPDATE seats SET status = 'reserved', updated_at = ?1 WHERE id = ?2",
                params![&now, req.seat_id],
            );
            
            HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "预约成功，请在规定时间内签到".to_string(),
                data: None,
            })
        },
        Err(e) => {
            eprintln!("创建预约失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "预约失败，请稍后重试".to_string(),
                data: None,
            })
        }
    }
}

// 其他函数也需要类似修改，将 [&value] 替换为 params![value]
// 以下是其他需要修改的关键位置：

/// 取消预约 - 修复参数
pub async fn cancel_reservation(
    path: web::Path<i32>,
    query: web::Query<std::collections::HashMap<String, String>>,
    db: web::Data<Mutex<Connection>>,
) -> impl Responder {
    let reservation_id = path.into_inner();
    let user_id = query.get("user_id").and_then(|s| s.parse().ok()).unwrap_or(1);
    let conn = db.lock().unwrap();
    let now = Utc::now().to_rfc3339();
    
    // 使用 _ 前缀标记未使用的变量
    let (seat_id, _start_time, status): (i32, String, String) = match conn.query_row(
        "SELECT seat_id, start_time, status FROM reservations WHERE id = ?1 AND user_id = ?2",
        params![reservation_id, user_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ) {
        Ok(data) => data,
        Err(_) => {
            return HttpResponse::NotFound().json(ApiResponse::<()> {
                success: false,
                message: "预约记录不存在或无权限".to_string(),
                data: None,
            });
        }
    };
    
    // 检查状态
    if status != "pending" && status != "active" {
        return HttpResponse::BadRequest().json(ApiResponse::<()> {
            success: false,
            message: format!("当前状态({})无法取消预约", status),
            data: None,
        });
    }
    
    // 更新预约状态
    match conn.execute(
        "UPDATE reservations SET status = 'cancelled', updated_at = ?1 WHERE id = ?2",
        params![&now, reservation_id],
    ) {
        Ok(_) => {
            let _ = conn.execute(
                "UPDATE seats SET status = 'available', updated_at = ?1 WHERE id = ?2",
                params![&now, seat_id],
            );
            
            HttpResponse::Ok().json(ApiResponse::<()> {
                success: true,
                message: "预约已取消".to_string(),
                data: None,
            })
        },
        Err(e) => {
            eprintln!("取消预约失败: {}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()> {
                success: false,
                message: "取消预约失败".to_string(),
                data: None,
            })
        }
    }
}
