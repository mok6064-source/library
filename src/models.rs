use serde::{Deserialize, Serialize};

// ========== 请求结构体 ==========
/// 注册请求（图书馆用户）
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub student_id: String,  // 学号/工号
    pub username: String,
    pub password: String,
    pub email: String,
    pub phone: Option<String>,
}

/// 登录请求
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub student_id: String,
    pub password: String,
}

/// 修改密码请求
#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

/// 更新用户信息请求
#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub email: Option<String>,
    pub phone: Option<String>,
}

// ========== 响应结构体 ==========

/// 统一响应格式
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

/// 登录成功后返回的用户信息
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserInfo,
}

/// 用户信息（不含密码）
#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: i32,
    pub student_id: String,
    pub username: String,
    pub email: String,
    pub phone: Option<String>,
    pub role: String,
}

// ========== JWT Claims ==========

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // 学号
    pub user_id: i32,     // 用户ID
    pub role: String,     // 角色：admin / student
    pub exp: usize,       // 过期时间戳
}

// ========== 座位相关结构体 ==========
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Seat {
    pub id: i32,
    pub seat_number: String,
    pub area: String,
    pub status: String, // available, occupied, reserved, disabled
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateSeatRequest {
    pub seat_number: String,
    pub area: String,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeatRequest {
    pub seat_number: Option<String>,
    pub area: Option<String>,
    pub status: Option<String>,
    pub x_coord: Option<i32>,
    pub y_coord: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct GetSeatsQuery {
    pub area: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AvailableSeatsQuery {
    pub start_time: Option<String>, // ISO 8601格式
    pub end_time: Option<String>,
}

// ========== 预约相关结构体 ==========
#[derive(Debug, Serialize, Deserialize)]
pub struct Reservation {
    pub id: i32,
    pub user_id: i32,
    pub seat_id: i32,
    pub seat_number: String,  // 关联查询得到
    pub area: String,          // 关联查询得到
    pub start_time: String,
    pub end_time: String,
    pub status: String, // pending, active, completed, cancelled, expired
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReservationRequest {
    pub seat_id: i32,
    pub start_time: String,
    pub end_time: String,
}

#[derive(Debug, Deserialize)]
pub struct ExtendReservationRequest {
    pub extra_hours: i32, // 延长小时数
}

// ========== 签到签退相关结构体 ==========
#[derive(Debug, Deserialize)]
pub struct CheckinRequest {
    pub reservation_id: i32,
    pub qr_code: Option<String>,      // 二维码扫码
    pub location: Option<String>,      // 签到位置
    pub lat: Option<f64>,              // 纬度
    pub lng: Option<f64>,              // 经度
}

#[derive(Debug, Deserialize)]
pub struct CheckoutRequest {
    pub reservation_id: i32,
}

#[derive(Debug, Serialize)]
pub struct AttendanceStatus {
    pub is_checked_in: bool,
    pub reservation_id: Option<i32>,
    pub seat_id: Option<i32>,
    pub seat_number: Option<String>,
    pub checkin_time: Option<String>,
    pub checkout_time: Option<String>,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckinResponse {
    pub reservation_id: i32,
    pub checkin_time: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct CheckoutResponse {
    pub reservation_id: i32,
    pub checkout_time: String,
    pub duration_minutes: i64,
    pub message: String,
}