extern crate::backend
  AppError(message:str)
  sip count_sip(limit:i64) progress=i64 -> i64
  sip fallible_sip(limit:i64) progress=i64 -> i64 ! AppError
