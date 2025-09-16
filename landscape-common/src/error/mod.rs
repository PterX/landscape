use sea_orm::DbErr;
use thiserror::Error;

use crate::error::pty::PtyError;

pub mod pty;

/// 仅定义当前 common 错误
#[derive(Error, Debug)]
pub enum LdError {
    #[error("Lnadscape boot error: {0}")]
    Boot(String),
    // OpenFileError
    #[error("I/O error occurred: {0}")]
    Io(#[from] std::io::Error),

    #[error("homedir error occurred: {0}")]
    HomeError(#[from] homedir::GetHomeError),

    #[error("setting cpu balance error: {0}")]
    SettingCpuBalanceError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] DbErr),

    #[error("data is expired")]
    DataIsExpired,

    #[error("Database error: {0}")]
    DbMsg(String),

    #[error(transparent)]
    PtyError(#[from] PtyError),
}

pub type LdResult<T> = Result<T, LdError>;

pub trait LandscapeErrRespTrait
where
    Self: std::fmt::Display,
{
    fn get_code(&self) -> u32;

    fn get_message(&self) -> String {
        self.to_string()
    }

    // fn error_to_response(&self) -> (u16, String) {
    //     let code = self.get_code();
    //     let http_code = code % 1000; // 取后三位作为 HTTP code
    //     let msg = self.get_message();

    //     (http_code as u16, msg)
    // }
}
